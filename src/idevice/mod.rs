//! A small, self-contained client for the protocols libimobiledevice speaks, so the app needs
//! nothing installed beyond the system's own USB service.
//!
//! The layers, bottom up:
//!
//! - `usbmux`: the system daemon that owns the USB link (built into macOS, the Apple Mobile Device
//!   Service on Windows, `usbmuxd` on Linux). It lists phones, keeps pair records, and turns a
//!   device port into a plain socket.
//! - `lockdown`: the phone's port 62078. Values, sessions, services, pairing.
//! - `tls`: lockdown sessions and most services are wrapped in TLS, authenticated by the pair
//!   record's host certificate.
//! - `pairing`: the certificates a pair record is made of.
//! - `services`: diagnostics_relay and installation_proxy, the two services the app reads.

pub mod lockdown;
pub mod pairing;
pub mod services;
mod tls;
pub mod usbmux;

use std::fmt;
use std::io;

use plist::{Dictionary, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Anything a plist conversation can run over: a Unix socket, a TCP socket, or TLS on either.
pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}
pub type BoxStream = Box<dyn Stream>;

#[derive(Debug)]
pub enum Error {
    /// The system USB service is not there to talk to.
    NoUsbService(io::Error),
    /// The phone is not (or no longer) connected.
    NotConnected,
    /// This computer holds no pair record for the phone, or the phone no longer accepts it.
    NotPaired,
    /// The phone answered with an error, such as `PasswordProtected` or `UserDeniedPairing`.
    Device(String),
    Io(io::Error),
    Protocol(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoUsbService(e) => write!(f, "The system's iPhone USB service is not running ({e})."),
            Self::NotConnected => f.write_str("The iPhone was disconnected."),
            Self::NotPaired => f.write_str("The iPhone has not trusted this computer yet. Unlock it, tap Trust, then press Pair."),
            Self::Device(code) => f.write_str(&describe(code)),
            Self::Io(e) => write!(f, "Lost the connection to the iPhone ({e})."),
            Self::Protocol(message) => f.write_str(message),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Lockdown's error codes, in words a person can act on.
fn describe(code: &str) -> String {
    match code {
        "PasswordProtected" => "The iPhone is locked. Unlock it, then try again.".into(),
        "PairingDialogResponsePending" => "Tap Trust on the iPhone and enter its passcode, then press Pair again.".into(),
        "UserDeniedPairing" => "Trust was declined on the iPhone. Unplug it, plug it back in, and tap Trust this time.".into(),
        "InvalidHostID" | "InvalidPairRecord" | "SessionInactive" => {
            "The iPhone has not trusted this computer yet. Unlock it, tap Trust, then press Pair.".into()
        }
        other => format!("The iPhone refused the request ({other})."),
    }
}

/// The largest message either side is allowed to send. The biggest real ones, full lockdown
/// dumps and ioreg entries, are tens of kilobytes.
const MAX_MESSAGE: usize = 16 << 20;

/// Lockdown and its services frame each plist with a 32-bit big-endian length.
pub async fn send_plist(stream: &mut (impl AsyncWrite + Unpin + ?Sized), message: &Dictionary) -> Result<()> {
    let mut body = Vec::new();
    Value::Dictionary(message.clone()).to_writer_xml(&mut body).map_err(|e| Error::Protocol(format!("Could not encode a request: {e}")))?;
    let len = u32::try_from(body.len()).map_err(|_| Error::Protocol("Request too large.".into()))?;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;
    Ok(())
}

pub async fn recv_plist(stream: &mut (impl AsyncRead + Unpin + ?Sized)) -> Result<Dictionary> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_MESSAGE {
        return Err(Error::Protocol(format!("The iPhone sent an oversized message ({len} bytes).")));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    parse_dict(&body)
}

/// XML or binary - the phone and the daemons use both.
pub fn parse_dict(bytes: &[u8]) -> Result<Dictionary> {
    match plist::from_bytes::<Value>(bytes) {
        Ok(Value::Dictionary(dict)) => Ok(dict),
        Ok(_) => Err(Error::Protocol("Expected a dictionary from the iPhone.".into())),
        Err(e) => Err(Error::Protocol(format!("Could not read what the iPhone sent: {e}"))),
    }
}

/// Builds a request dictionary from key-value pairs.
pub fn dict<const N: usize>(entries: [(&str, Value); N]) -> Dictionary {
    entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn framing_round_trip() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        let sent = dict([("Request", "QueryType".into()), ("Label", "test".into())]);
        send_plist(&mut a, &sent).await.unwrap();
        assert_eq!(recv_plist(&mut b).await.unwrap(), sent);
    }

    #[tokio::test]
    async fn oversized_messages_are_refused() {
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&u32::MAX.to_be_bytes()).await.unwrap();
        assert!(matches!(recv_plist(&mut b).await, Err(Error::Protocol(_))));
    }
}
