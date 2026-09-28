//! Lockdown, the phone's front door on port 62078: values, sessions, services and pairing.

use plist::{Dictionary, Value};

use super::pairing::PairRecord;
use super::{BoxStream, Error, Result, dict, recv_plist, send_plist, tls, usbmux};

const PORT: u16 = 62078;
const LABEL: &str = "iphone-inspector";

pub struct Lockdown {
    stream: BoxStream,
    device: usbmux::Device,
    /// Kept after a session starts, for wrapping service connections in TLS.
    record: Option<PairRecord>,
}

impl Lockdown {
    pub async fn connect(device: &usbmux::Device) -> Result<Self> {
        let stream = usbmux::connect(device, PORT).await?;
        let mut lockdown = Self { stream, device: device.clone(), record: None };
        let reply = lockdown.request("QueryType", Dictionary::new()).await?;
        match reply.get("Type").and_then(Value::as_string) {
            Some("com.apple.mobile.lockdown") => Ok(lockdown),
            other => Err(Error::Protocol(format!("Port {PORT} is not lockdown ({other:?})."))),
        }
    }

    /// Connect and start a session with this computer's pair record: what reading anything
    /// beyond the basics requires. `NotPaired` when the phone has not trusted this computer.
    pub async fn session(udid: &str) -> Result<Self> {
        let device = usbmux::find(udid).await?;
        let record = PairRecord::load(udid).await?.ok_or(Error::NotPaired)?;
        let mut lockdown = Self::connect(&device).await?;
        let reply = lockdown
            .request("StartSession", dict([("HostID", record.host_id.as_str().into()), ("SystemBUID", record.system_buid.as_str().into())]))
            .await
            .map_err(|e| match e {
                // The phone has forgotten this computer - erased, or "Reset Location & Privacy".
                Error::Device(code) if code == "InvalidHostID" => Error::NotPaired,
                e => e,
            })?;
        let Self { mut stream, device, .. } = lockdown;
        if reply.get("EnableSessionSSL").and_then(Value::as_boolean) == Some(true) {
            stream = tls::wrap(stream, &record.host_certificate, &record.host_private_key).await?;
        }
        Ok(Self { stream, device, record: Some(record) })
    }

    async fn request(&mut self, request: &str, mut fields: Dictionary) -> Result<Dictionary> {
        fields.insert("Label".into(), LABEL.into());
        fields.insert("Request".into(), request.into());
        send_plist(&mut self.stream, &fields).await?;
        let reply = recv_plist(&mut self.stream).await?;
        match reply.get("Error").and_then(Value::as_string) {
            Some(code) => Err(Error::Device(code.to_string())),
            None => Ok(reply),
        }
    }

    /// One value, a whole domain (`key` None), or everything (both None).
    pub async fn get_value(&mut self, domain: Option<&str>, key: Option<&str>) -> Result<Value> {
        let mut fields = Dictionary::new();
        if let Some(domain) = domain {
            fields.insert("Domain".into(), domain.into());
        }
        if let Some(key) = key {
            fields.insert("Key".into(), key.into());
        }
        let mut reply = self.request("GetValue", fields).await?;
        reply.remove("Value").ok_or_else(|| Error::Device("MissingValue".into()))
    }

    /// A domain as a dictionary, or None when the phone has nothing for it.
    pub async fn get_domain(&mut self, domain: Option<&str>) -> Result<Option<Dictionary>> {
        match self.get_value(domain, None).await {
            Ok(Value::Dictionary(d)) => Ok(Some(d)),
            Ok(_) | Err(Error::Device(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_string(&mut self, key: &str) -> Result<Option<String>> {
        match self.get_value(None, Some(key)).await {
            Ok(Value::String(s)) => Ok(Some(s)),
            Ok(_) | Err(Error::Device(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Start a service and connect to it, in TLS when the phone asks for it.
    pub async fn start_service(&mut self, name: &str) -> Result<BoxStream> {
        let reply = self.request("StartService", dict([("Service", name.into())])).await?;
        let port = reply.get("Port").and_then(Value::as_unsigned_integer).and_then(|p| u16::try_from(p).ok());
        let port = port.ok_or_else(|| Error::Protocol(format!("The iPhone did not open {name}.")))?;
        let stream = usbmux::connect(&self.device, port).await?;
        if reply.get("EnableServiceSSL").and_then(Value::as_boolean) != Some(true) {
            return Ok(stream);
        }
        let record = self.record.as_ref().ok_or(Error::NotPaired)?;
        tls::wrap(stream, &record.host_certificate, &record.host_private_key).await
    }

    /// Offer a pair record (without its private keys). The reply carries the escrow bag.
    pub async fn pair(&mut self, record: Dictionary) -> Result<Dictionary> {
        self.request(
            "Pair",
            dict([
                ("PairRecord", Value::Dictionary(record)),
                ("ProtocolVersion", "2".into()),
                ("PairingOptions", Value::Dictionary(dict([("ExtendedPairingErrors", true.into())]))),
            ]),
        )
        .await
    }
}
