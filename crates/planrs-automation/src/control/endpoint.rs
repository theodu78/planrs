//! Adresse du canal de contrôle dans le rendez-vous (C-CTL-11, ADR-0016
//! D12) : `unix:<chemin>` (socket Unix) ou `tcp:127.0.0.1:<port>` (boucle
//! locale, Windows). Contrat posé par le lot L0 ; le rendez-vous et le client
//! l'adoptent en L8.

use std::net::SocketAddrV4;
use std::path::PathBuf;

use super::ControlError;

/// Adresse d'écoute de l'application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    /// Socket Unix.
    Unix(PathBuf),
    /// TCP en boucle locale (IPv4 seulement).
    Tcp(SocketAddrV4),
}

impl Endpoint {
    /// Lecture de `unix:<chemin>` ou `tcp:<ipv4>:<port>` ; un hôte TCP qui
    /// n'est pas en boucle locale est refusé (C-CTL-11).
    ///
    /// # Errors
    ///
    /// [`ControlError::BadEndpoint`] (`control.bad_endpoint`).
    pub fn parse(s: &str) -> Result<Self, ControlError> {
        let bad = || ControlError::BadEndpoint(s.chars().take(200).collect());
        if let Some(path) = s.strip_prefix("unix:") {
            if path.is_empty() || path.contains('\0') {
                return Err(bad());
            }
            return Ok(Self::Unix(PathBuf::from(path)));
        }
        if let Some(addr) = s.strip_prefix("tcp:") {
            let addr: SocketAddrV4 = addr.parse().map_err(|_| bad())?;
            if !addr.ip().is_loopback() || addr.port() == 0 {
                return Err(bad());
            }
            return Ok(Self::Tcp(addr));
        }
        Err(bad())
    }

    /// Écriture (inverse de [`Endpoint::parse`]) ; un chemin non UTF-8 est
    /// écrit avec des caractères de remplacement (relu différemment, donc
    /// refusé au rattachement plutôt que mal interprété).
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Unix(path) => format!("unix:{}", path.to_string_lossy()),
            Self::Tcp(addr) => format!("tcp:{addr}"),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]

    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn c_ctl_11_endpoint_roundtrip() {
        let tcp = Endpoint::parse("tcp:127.0.0.1:49152").unwrap();
        assert_eq!(tcp, Endpoint::Tcp(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 49152)));
        assert_eq!(tcp.render(), "tcp:127.0.0.1:49152");
        let unix = Endpoint::parse("unix:/run/user/1000/planrs/a.sock").unwrap();
        assert_eq!(unix.render(), "unix:/run/user/1000/planrs/a.sock");
        assert_eq!(Endpoint::parse(&unix.render()).unwrap(), unix);
    }

    #[test]
    fn c_ctl_11_endpoint_refuses_non_loopback_and_garbage() {
        for bad in [
            "",
            "tcp:",
            "tcp:0.0.0.0:4000",
            "tcp:192.168.1.16:4000",
            "tcp:127.0.0.1:0",
            "tcp:127.0.0.1",
            "tcp:[::1]:4000",
            "tcp:localhost:4000",
            "unix:",
            "pipe:planrs",
            "TCP:127.0.0.1:4000",
        ] {
            let err = Endpoint::parse(bad).unwrap_err();
            assert!(matches!(err, ControlError::BadEndpoint(_)), "{bad}");
        }
    }
}
