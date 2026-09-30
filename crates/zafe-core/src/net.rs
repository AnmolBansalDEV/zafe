//! Why a connection to the relay or lightwalletd failed, read from the error's types (not
//! its text), so the app can tell "unreachable" from a certificate problem or a timeout.

use std::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetFailure {
    /// No connection: DNS, refused, reset, no route, offline.
    Unreachable,
    /// The TLS handshake failed: an untrusted or expired certificate, a hostname mismatch,
    /// or a server that doesn't speak TLS.
    Tls,
    /// The server didn't answer in time.
    Timeout,
    /// The server answered, with an error.
    Server,
}

impl NetFailure {
    /// Classifies a transport error by walking its causes.
    pub fn of(e: &(dyn Error + 'static)) -> Self {
        let mut timeout = false;
        let mut current = Some(e);
        while let Some(err) = current {
            if is_tls(err) {
                return NetFailure::Tls;
            }
            timeout |= err.is::<tonic::TimeoutExpired>();
            if let Some(status) = err.downcast_ref::<tonic::Status>() {
                timeout |= status.code() == tonic::Code::DeadlineExceeded;
            }
            if let Some(r) = err.downcast_ref::<reqwest::Error>() {
                timeout |= r.is_timeout();
            }
            current = match err.downcast_ref::<std::io::Error>() {
                // `io::Error::source` skips the error it wraps (and reqwest nests them:
                // io(Other, io(InvalidData, rustls::Error))), so step into it instead.
                Some(io) => {
                    timeout |= io.kind() == std::io::ErrorKind::TimedOut;
                    match io.get_ref() {
                        Some(inner) => Some(inner as &(dyn Error + 'static)),
                        None => err.source(),
                    }
                }
                None => err.source(),
            };
        }
        if timeout {
            NetFailure::Timeout
        } else {
            NetFailure::Unreachable
        }
    }

    /// Classifies a gRPC status: transport codes by their cause, anything else is the
    /// server's own answer.
    pub fn of_status(status: &tonic::Status) -> Self {
        use tonic::Code;
        match status.code() {
            // tonic reports its own request timeout as `Cancelled`.
            Code::Cancelled => match Self::of(status) {
                NetFailure::Unreachable => NetFailure::Timeout,
                f => f,
            },
            Code::Unavailable | Code::Unknown | Code::DeadlineExceeded => Self::of(status),
            _ => NetFailure::Server,
        }
    }
}

fn is_tls(e: &(dyn Error + 'static)) -> bool {
    e.is::<rustls::Error>()
}

/// An error and its causes on one line ("error sending request: ...: invalid peer
/// certificate: UnknownIssuer"), since the outer messages alone say little.
pub fn error_chain(e: &dyn Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        if !message.ends_with(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        source = cause.source();
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_io_causes() {
        let refused = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        assert_eq!(NetFailure::of(&refused), NetFailure::Unreachable);
        let timed_out = std::io::Error::new(std::io::ErrorKind::TimedOut, "slow");
        assert_eq!(NetFailure::of(&timed_out), NetFailure::Timeout);
        let tls = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer),
        );
        assert_eq!(NetFailure::of(&tls), NetFailure::Tls);
        // As reqwest reports it: an io error wrapping the handshake's io error.
        let nested = std::io::Error::other(tls);
        assert_eq!(NetFailure::of(&nested), NetFailure::Tls);
    }

    #[test]
    fn classifies_statuses() {
        assert_eq!(
            NetFailure::of_status(&tonic::Status::not_found("no such block")),
            NetFailure::Server
        );
        assert_eq!(
            NetFailure::of_status(&tonic::Status::unavailable("down")),
            NetFailure::Unreachable
        );
        assert_eq!(
            NetFailure::of_status(&tonic::Status::deadline_exceeded("slow")),
            NetFailure::Timeout
        );
        assert_eq!(
            NetFailure::of_status(&tonic::Status::cancelled("Timeout expired")),
            NetFailure::Timeout
        );
    }
}
