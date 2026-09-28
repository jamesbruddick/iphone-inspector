//! The system USB multiplexer: lists phones, keeps their pair records, and opens sockets to ports
//! on them.
//!
//! Each request opens its own connection, as libusbmuxd does - a `Connect` turns the connection
//! into the tunnel, so it cannot be reused anyway.

use plist::{Dictionary, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{BoxStream, Error, Result, dict, parse_dict};

/// Header: total length, protocol version (1 = plist), message type (8 = plist), tag. Little-endian.
const HEADER: usize = 16;
const PLIST_VERSION: u32 = 1;
const PLIST_MESSAGE: u32 = 8;
const PROG_NAME: &str = "iphone-inspector";

#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub id: u64,
    pub udid: String,
}

async fn open() -> Result<BoxStream> {
    // Same override libusbmuxd honors: `UNIX:/path` or `host:port`, e.g. to reach a Windows host's
    // service from WSL.
    if let Ok(address) = std::env::var("USBMUXD_SOCKET_ADDRESS") {
        #[cfg(unix)]
        if let Some(path) = address.strip_prefix("UNIX:") {
            return Ok(Box::new(tokio::net::UnixStream::connect(path).await.map_err(Error::NoUsbService)?));
        }
        return Ok(Box::new(tokio::net::TcpStream::connect(address).await.map_err(Error::NoUsbService)?));
    }
    #[cfg(unix)]
    let stream = tokio::net::UnixStream::connect("/var/run/usbmuxd").await;
    #[cfg(windows)]
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", 27015)).await;
    Ok(Box::new(stream.map_err(Error::NoUsbService)?))
}

async fn send(stream: &mut BoxStream, mut message: Dictionary) -> Result<()> {
    message.insert("ClientVersionString".into(), concat!("iphone-inspector ", env!("CARGO_PKG_VERSION")).into());
    message.insert("ProgName".into(), PROG_NAME.into());
    message.insert("kLibUSBMuxVersion".into(), 3.into());
    let mut body = Vec::new();
    Value::Dictionary(message).to_writer_xml(&mut body).map_err(|e| Error::Protocol(e.to_string()))?;
    let len = u32::try_from(HEADER + body.len()).map_err(|_| Error::Protocol("Request too large.".into()))?;
    let mut packet = Vec::with_capacity(HEADER + body.len());
    for word in [len, PLIST_VERSION, PLIST_MESSAGE, 1] {
        packet.extend_from_slice(&word.to_le_bytes());
    }
    packet.extend_from_slice(&body);
    stream.write_all(&packet).await?;
    stream.flush().await?;
    Ok(())
}

async fn recv(stream: &mut BoxStream) -> Result<Dictionary> {
    let mut header = [0u8; HEADER];
    stream.read_exact(&mut header).await?;
    let len = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if !(HEADER..=super::MAX_MESSAGE).contains(&len) {
        return Err(Error::Protocol(format!("The USB service sent a bad message length ({len}).")));
    }
    let mut body = vec![0u8; len - HEADER];
    stream.read_exact(&mut body).await?;
    parse_dict(&body)
}

async fn request(message: Dictionary) -> Result<(BoxStream, Dictionary)> {
    let mut stream = open().await?;
    send(&mut stream, message).await?;
    let reply = recv(&mut stream).await?;
    Ok((stream, reply))
}

/// The `Number` of a `Result` reply; 0 is success.
fn result_code(reply: &Dictionary) -> Option<u64> {
    reply.get("Number").and_then(Value::as_unsigned_integer)
}

/// Newer phones have 24-character UDIDs that the daemon reports without the dash everything else
/// shows (Finder, Xcode, Apple's own tools). libusbmuxd puts it back, and so does this.
fn normalize_udid(udid: &str) -> String {
    if udid.len() == 24 && !udid.contains('-') { format!("{}-{}", &udid[..8], &udid[8..]) } else { udid.to_string() }
}

/// Phones connected by cable. Wi-Fi sync connections are left out, as `idevice_id -l` does.
pub async fn devices() -> Result<Vec<Device>> {
    let (_, reply) = request(dict([("MessageType", "ListDevices".into())])).await?;
    let list = reply.get("DeviceList").and_then(Value::as_array).ok_or_else(|| Error::Protocol("The USB service sent no device list.".into()))?;
    let mut out: Vec<Device> = Vec::new();
    for entry in list {
        let Some(props) = entry.as_dictionary().and_then(|d| d.get("Properties")).and_then(Value::as_dictionary) else { continue };
        let usb = props.get("ConnectionType").and_then(Value::as_string).is_none_or(|t| t == "USB");
        let id = props.get("DeviceID").and_then(Value::as_unsigned_integer);
        let udid = props.get("SerialNumber").and_then(Value::as_string).map(normalize_udid);
        if let (true, Some(id), Some(udid)) = (usb, id, udid)
            && !out.iter().any(|d| d.udid == udid)
        {
            out.push(Device { id, udid });
        }
    }
    Ok(out)
}

pub async fn find(udid: &str) -> Result<Device> {
    devices().await?.into_iter().find(|d| d.udid == udid).ok_or(Error::NotConnected)
}

/// Open a socket to a port on the phone.
pub async fn connect(device: &Device, port: u16) -> Result<BoxStream> {
    // The daemon expects the port in network byte order, read back as a host integer.
    let port = u16::from_ne_bytes(port.to_be_bytes());
    let (stream, reply) = request(dict([("MessageType", "Connect".into()), ("DeviceID", device.id.into()), ("PortNumber", u64::from(port).into())])).await?;
    match result_code(&reply) {
        Some(0) => Ok(stream),
        // 2 = no such device, 3 = the phone refused the port.
        Some(2) => Err(Error::NotConnected),
        Some(code) => Err(Error::Protocol(format!("The iPhone refused the connection (usbmux error {code})."))),
        None => Err(Error::Protocol("The USB service sent an unexpected reply to Connect.".into())),
    }
}

/// The pair record the system holds for a phone, if it has one. On macOS this includes the one
/// Finder made, so a phone that already trusts the Mac needs no pairing here.
pub async fn read_pair_record(udid: &str) -> Result<Option<Dictionary>> {
    // Records are filed under the dashed UDID; the bare form is tried too in case a daemon keeps it.
    for id in [udid.to_string(), udid.replace('-', "")].into_iter().take(if udid.contains('-') { 2 } else { 1 }) {
        let (_, reply) = request(dict([("MessageType", "ReadPairRecord".into()), ("PairRecordID", id.into())])).await?;
        if let Some(data) = reply.get("PairRecordData").and_then(Value::as_data) {
            return Ok(Some(parse_dict(data)?));
        }
    }
    Ok(None)
}

pub async fn save_pair_record(device: &Device, record: &Dictionary) -> Result<()> {
    let mut data = Vec::new();
    Value::Dictionary(record.clone()).to_writer_xml(&mut data).map_err(|e| Error::Protocol(e.to_string()))?;
    let (_, reply) = request(dict([
        ("MessageType", "SavePairRecord".into()),
        ("PairRecordID", device.udid.as_str().into()),
        ("PairRecordData", Value::Data(data)),
        ("DeviceID", device.id.into()),
    ]))
    .await?;
    match result_code(&reply) {
        Some(0) | None => Ok(()),
        Some(code) => Err(Error::Protocol(format!("The system would not save the pairing (usbmux error {code})."))),
    }
}

/// The ID this computer presents to phones, shared by everything that pairs through the daemon.
pub async fn system_buid() -> Result<String> {
    let (_, reply) = request(dict([("MessageType", "ReadBUID".into())])).await?;
    reply.get("BUID").and_then(Value::as_string).map(str::to_string).ok_or_else(|| Error::Protocol("The USB service has no system ID.".into()))
}

#[cfg(test)]
mod tests {
    use super::normalize_udid;

    #[test]
    fn udids() {
        assert_eq!(normalize_udid("00008150000A1B2C3D4E5F60"), "00008150-000A1B2C3D4E5F60");
        assert_eq!(normalize_udid("00008150-000A1B2C3D4E5F60"), "00008150-000A1B2C3D4E5F60");
        assert_eq!(normalize_udid("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678"), "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678");
    }
}

/// Needs the system USB service (always there on macOS): `cargo test -- --ignored`. Lists any
/// plugged-in phone and whether this computer holds a pair record for it.
#[cfg(test)]
mod live {
    #[tokio::test]
    #[ignore]
    async fn system_usb_service() {
        println!("system BUID: {}", super::system_buid().await.unwrap());
        for device in super::devices().await.unwrap() {
            let paired = super::read_pair_record(&device.udid).await.unwrap().is_some();
            println!("{} (id {}): pair record {}", device.udid, device.id, if paired { "found" } else { "missing" });
        }
    }
}
