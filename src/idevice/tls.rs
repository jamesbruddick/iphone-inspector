//! TLS for lockdown sessions and services.
//!
//! The phone proves nothing here that matters: the connection already runs over a cable to a
//! device the USB service vouched for, and its certificate is one this computer minted at pairing
//! time. What matters is the other direction - the phone only talks once the host certificate
//! from the pair record is presented. So the phone's certificate is not checked (libimobiledevice
//! does not check it either), and handshake signatures are not verified, because older phones sign
//! with 1024-bit keys that no modern TLS library accepts.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, ring};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tokio_rustls::TlsConnector;

use super::{BoxStream, Error, Result};

#[derive(Debug)]
struct TrustTheCable(Arc<CryptoProvider>);

impl ServerCertVerifier for TrustTheCable {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Wrap a stream in TLS, presenting the host certificate and key (PEM) from a pair record.
pub async fn wrap(stream: BoxStream, host_cert_pem: &[u8], host_key_pem: &[u8]) -> Result<BoxStream> {
    let bad_record = |what: &str| Error::Protocol(format!("The pair record's {what} is unreadable; pair again."));
    let cert = CertificateDer::from_pem_slice(host_cert_pem).map_err(|_| bad_record("host certificate"))?;
    let key = PrivateKeyDer::from_pem_slice(host_key_pem).map_err(|_| bad_record("host key"))?;

    let provider = Arc::new(ring::default_provider());
    let mut config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| Error::Protocol(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(TrustTheCable(provider)))
        .with_client_auth_cert(vec![cert], key)
        .map_err(|_| bad_record("host key"))?;
    config.enable_sni = false;

    let name = ServerName::IpAddress(std::net::Ipv4Addr::LOCALHOST.into());
    let tls = TlsConnector::from(Arc::new(config))
        .connect(name, stream)
        .await
        .map_err(|e| Error::Protocol(format!("Secure connection to the iPhone failed: {e}")))?;
    Ok(Box::new(tls))
}
