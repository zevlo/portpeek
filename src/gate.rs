use std::fmt;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    Local,
    Remote,
}

pub fn classify(host: &str) -> HostKind {
    if host.eq_ignore_ascii_case("localhost") {
        return HostKind::Local;
    }
    match host.parse::<IpAddr>() {
        Ok(ip) if ip.is_loopback() || ip.is_unspecified() => HostKind::Local,
        _ => HostKind::Remote,
    }
}

pub fn gate(host: &str, allow_remote: bool) -> Result<(), String> {
    match classify(host) {
        HostKind::Local => Ok(()),
        HostKind::Remote if allow_remote => Ok(()),
        HostKind::Remote => Err(format!(
            "refusing to scan remote host {host:?} without --allow-remote.\n\
             \n\
             This tool scans 127.0.0.1 by default. Scanning another machine \
             without the owner's consent can be illegal in your jurisdiction.\n\
             If you own {host:?} or have permission, re-run with --allow-remote."
        )),
    }
}

impl fmt::Display for HostKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostKind::Local => write!(f, "local"),
            HostKind::Remote => write!(f, "remote"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use HostKind::*;

    #[test]
    fn localhost_is_local() {
        assert_eq!(classify("localhost"), Local);
        assert_eq!(classify("LOCALHOST"), Local);
        assert_eq!(classify("Localhost"), Local);
    }

    #[test]
    fn loopback_ipv4_is_local() {
        for host in ["127.0.0.1", "127.0.0.2", "127.255.0.9"] {
            assert_eq!(classify(host), Local, "{host} should be local");
        }
    }

    #[test]
    fn loopback_ipv6_is_local() {
        assert_eq!(classify("::1"), Local);
    }

    #[test]
    fn unspecified_is_local() {
        assert_eq!(classify("0.0.0.0"), Local);
        assert_eq!(classify("::"), Local);
    }

    #[test]
    fn rfc1918_is_remote() {
        assert_eq!(classify("192.168.1.1"), Remote);
        assert_eq!(classify("10.0.0.5"), Remote);
        assert_eq!(classify("172.16.0.1"), Remote);
    }

    #[test]
    fn public_ips_are_remote() {
        assert_eq!(classify("8.8.8.8"), Remote);
        assert_eq!(classify("2001:db8::1"), Remote);
    }

    #[test]
    fn ipv4_mapped_loopback_is_remote() {
        assert_eq!(classify("::ffff:127.0.0.1"), Remote);
    }

    #[test]
    fn hostnames_are_remote() {
        assert_eq!(classify("example.com"), Remote);
        assert_eq!(classify("corp-dev.example.com"), Remote);
        assert_eq!(classify("localhost.localdomain"), Remote);
    }

    #[test]
    fn gate_passes_local() {
        assert_eq!(gate("127.0.0.1", false), Ok(()));
        assert_eq!(gate("::1", false), Ok(()));
    }

    #[test]
    fn gate_blocks_remote_without_flag() {
        let err = gate("192.168.1.1", false).unwrap_err();
        assert!(
            err.contains("--allow-remote"),
            "message should name the flag: {err}"
        );
    }

    #[test]
    fn gate_passes_remote_with_flag() {
        assert_eq!(gate("8.8.8.8", true), Ok(()));
    }
}
