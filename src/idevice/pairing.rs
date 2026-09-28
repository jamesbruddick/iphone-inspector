//! Pairing: the certificates that make up a pair record, and the exchange that gets the phone to
//! accept them.
//!
//! A pair record holds a root CA, a host certificate signed by it (which this computer presents
//! in TLS), and a device certificate signed by it over the phone's own public key. Fresh keys are
//! made for every pairing, as libimobiledevice does. The record is kept by the system USB service,
//! where libimobiledevice, Finder and iTunes find it too.

use plist::{Dictionary, Value};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, IsCa, Issuer, KeyPair, KeyUsagePurpose, PKCS_RSA_SHA256, PublicKeyData, SignatureAlgorithm,
};
use rsa::pkcs1::{DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};
use rsa::pkcs8::EncodePrivateKey;
use rsa::rand_core::{OsRng, RngCore};
use rsa::{RsaPrivateKey, RsaPublicKey};

use super::lockdown::Lockdown;
use super::{Error, Result, dict, usbmux};

/// What a lockdown session needs from a pair record.
pub struct PairRecord {
    pub host_id: String,
    pub system_buid: String,
    pub host_certificate: Vec<u8>,
    pub host_private_key: Vec<u8>,
}

impl PairRecord {
    pub fn from_dict(record: &Dictionary) -> Option<Self> {
        let string = |k: &str| record.get(k).and_then(Value::as_string).map(str::to_string);
        let data = |k: &str| record.get(k).and_then(Value::as_data).map(<[u8]>::to_vec);
        Some(Self {
            host_id: string("HostID")?,
            system_buid: string("SystemBUID")?,
            host_certificate: data("HostCertificate")?,
            host_private_key: data("HostPrivateKey")?,
        })
    }

    /// The system's record for this phone. None means it has never been paired with this computer.
    pub async fn load(udid: &str) -> Result<Option<Self>> {
        Ok(usbmux::read_pair_record(udid).await?.as_ref().and_then(Self::from_dict))
    }
}

/// A phone's public key as lockdown hands it over: PKCS#1, which is exactly the key bits an X.509
/// certificate carries for RSA.
struct DeviceKey(Vec<u8>);

impl PublicKeyData for DeviceKey {
    fn der_bytes(&self) -> &[u8] {
        &self.0
    }

    fn algorithm(&self) -> &'static SignatureAlgorithm {
        &PKCS_RSA_SHA256
    }
}

/// Everything generated for one pairing, all PEM.
struct Certificates {
    root_certificate: String,
    root_private_key: String,
    host_certificate: String,
    host_private_key: String,
    device_certificate: String,
}

fn cert_err(e: impl std::fmt::Display) -> Error {
    Error::Protocol(format!("Could not create the pairing certificates: {e}"))
}

fn rsa_key() -> Result<(RsaPrivateKey, KeyPair)> {
    let key = RsaPrivateKey::new(&mut OsRng, 2048).map_err(cert_err)?;
    let pkcs8 = key.to_pkcs8_der().map_err(cert_err)?;
    let signer = KeyPair::from_pkcs8_der_and_sign_algo(&pkcs8.as_bytes().into(), &PKCS_RSA_SHA256).map_err(cert_err)?;
    Ok((key, signer))
}

/// Ten years from the start of this year, as Apple's and libimobiledevice's certificates run.
fn params(is_ca: bool) -> CertificateParams {
    use chrono::Datelike;
    let year = chrono::Utc::now().year();
    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params.not_before = rcgen::date_time_ymd(year, 1, 1);
    params.not_after = rcgen::date_time_ymd(year + 10, 1, 1);
    if is_ca {
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    } else {
        params.is_ca = IsCa::ExplicitNoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
    }
    params
}

fn generate(device_public_key: &[u8]) -> Result<Certificates> {
    let unreadable = || Error::Protocol("The iPhone sent an unreadable public key.".into());
    let device_key = std::str::from_utf8(device_public_key).ok().and_then(|pem| RsaPublicKey::from_pkcs1_pem(pem.trim()).ok()).ok_or_else(unreadable)?;
    let device_key = device_key.to_pkcs1_der().map_err(|_| unreadable())?;
    let (root_key, root_signer) = rsa_key()?;
    let (host_key, host_signer) = rsa_key()?;

    let root_params = params(true);
    let root = root_params.self_signed(&root_signer).map_err(cert_err)?;
    let issuer = Issuer::new(root_params, &root_signer);
    let host = params(false).signed_by(&host_signer, &issuer).map_err(cert_err)?;
    let device = params(false).signed_by(&DeviceKey(device_key.into_vec()), &issuer).map_err(cert_err)?;

    // Keys go in the traditional "RSA PRIVATE KEY" form, which every pair-record reader accepts.
    let pkcs1 = |key: &RsaPrivateKey| key.to_pkcs1_pem(LineEnding::LF).map(|p| p.to_string()).map_err(cert_err);
    Ok(Certificates {
        root_certificate: root.pem(),
        root_private_key: pkcs1(&root_key)?,
        host_certificate: host.pem(),
        host_private_key: pkcs1(&host_key)?,
        device_certificate: device.pem(),
    })
}

/// An uppercase random (v4) UUID, the form lockdown uses for host IDs.
fn host_id() -> String {
    let mut b = [0u8; 16];
    OsRng.fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02X}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// How long `pair` waits for Trust to be tapped (and the passcode entered).
pub const TRUST_WAIT: std::time::Duration = std::time::Duration::from_secs(60);
const WAIT_FOR: [&str; 2] = ["PairingDialogResponsePending", "PasswordProtected"];

/// Ask the phone to trust this computer, wait for Trust to be tapped, and save the pair record.
pub async fn pair(udid: &str) -> Result<()> {
    let device = usbmux::find(udid).await?;
    let mut lockdown = Lockdown::connect(&device).await?;
    let public_key = match lockdown.get_value(None, Some("DevicePublicKey")).await? {
        Value::Data(key) => key,
        _ => return Err(Error::Protocol("The iPhone did not send its public key.".into())),
    };
    let wifi_address = lockdown.get_value(None, Some("WiFiAddress")).await.ok();
    let system_buid = usbmux::system_buid().await?;
    let certs = tokio::task::spawn_blocking(move || generate(&public_key)).await.map_err(|e| Error::Protocol(e.to_string()))??;
    let host_id = host_id();

    let data = |pem: &str| Value::Data(pem.as_bytes().to_vec());
    let offered = dict([
        ("DeviceCertificate", data(&certs.device_certificate)),
        ("HostCertificate", data(&certs.host_certificate)),
        ("RootCertificate", data(&certs.root_certificate)),
        ("HostID", host_id.as_str().into()),
        ("SystemBUID", system_buid.as_str().into()),
    ]);
    // The phone answers at once that the dialog is up (or that it is locked, so the dialog cannot
    // be); asking again with the same record goes through once Trust is tapped.
    let started = std::time::Instant::now();
    let reply = loop {
        match lockdown.pair(offered.clone()).await {
            Err(Error::Device(code)) if WAIT_FOR.contains(&code.as_str()) && started.elapsed() < TRUST_WAIT => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            result => break result?,
        }
    };

    let mut record = offered;
    record.insert("HostPrivateKey".into(), data(&certs.host_private_key));
    record.insert("RootPrivateKey".into(), data(&certs.root_private_key));
    if let Some(bag) = reply.get("EscrowBag") {
        record.insert("EscrowBag".into(), bag.clone());
    }
    if let Some(Value::String(address)) = wifi_address {
        record.insert("WiFiMACAddress".into(), address.into());
    }
    usbmux::save_pair_record(&device, &record).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::pem::PemObject;

    #[test]
    fn certificates_for_a_device_key() {
        let device = RsaPrivateKey::new(&mut OsRng, 1024).unwrap();
        let public = device.to_public_key().to_pkcs1_pem(LineEnding::LF).unwrap();
        let certs = generate(public.as_bytes()).unwrap();
        for pem in [&certs.root_certificate, &certs.host_certificate, &certs.device_certificate] {
            assert!(pem.starts_with("-----BEGIN CERTIFICATE-----"));
        }
        assert!(certs.host_private_key.starts_with("-----BEGIN RSA PRIVATE KEY-----"));
        // The device certificate carries the phone's key, not one of ours.
        let der = rustls::pki_types::CertificateDer::from_pem_slice(certs.device_certificate.as_bytes()).unwrap();
        let key = device.to_public_key().to_pkcs1_der().unwrap().into_vec();
        assert!(der.windows(key.len()).any(|w| w == key.as_slice()));
    }

    /// A stand-in phone: a TLS server with the device certificate, which, like lockdownd, only
    /// talks to a client presenting a host certificate issued by the pair record's root.
    #[tokio::test]
    async fn session_tls_with_a_pair_record() {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use std::sync::Arc;

        let device = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        let public = device.to_public_key().to_pkcs1_pem(LineEnding::LF).unwrap();
        let certs = generate(public.as_bytes()).unwrap();

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from_pem_slice(certs.root_certificate.as_bytes()).unwrap()).unwrap();
        let clients = rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone()).build().unwrap();
        let server = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_client_cert_verifier(clients)
            .with_single_cert(
                vec![CertificateDer::from_pem_slice(certs.device_certificate.as_bytes()).unwrap()],
                PrivateKeyDer::Pkcs8(device.to_pkcs8_der().unwrap().as_bytes().to_vec().into()),
            )
            .unwrap();

        let (phone_end, host_end) = tokio::io::duplex(1 << 16);
        let phone = tokio::spawn(async move {
            let mut tls = tokio_rustls::TlsAcceptor::from(Arc::new(server)).accept(phone_end).await.unwrap();
            let request = crate::idevice::recv_plist(&mut tls).await.unwrap();
            crate::idevice::send_plist(&mut tls, &request).await.unwrap();
        });

        let mut stream = crate::idevice::tls::wrap(Box::new(host_end), certs.host_certificate.as_bytes(), certs.host_private_key.as_bytes()).await.unwrap();
        let sent = dict([("Request", "GetValue".into())]);
        crate::idevice::send_plist(&mut stream, &sent).await.unwrap();
        assert_eq!(crate::idevice::recv_plist(&mut stream).await.unwrap(), sent);
        phone.await.unwrap();
    }

    #[test]
    fn host_ids() {
        let id = host_id();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert_eq!(id, id.to_uppercase());
    }
}
