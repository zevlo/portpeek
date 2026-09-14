use std::collections::BTreeSet;
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    EmptyItem,
    Whitespace(String),
    NotAPort(String),
    PortZero,
    InvalidRange { lo: u16, hi: u16 },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "port spec is empty"),
            ParseError::EmptyItem => write!(f, "empty item in port spec"),
            ParseError::Whitespace(item) => {
                write!(
                    f,
                    "whitespace in port spec: {item:?} (quote the spec, no spaces)"
                )
            }
            ParseError::NotAPort(s) => write!(f, "not a valid port: {s:?}"),
            ParseError::PortZero => write!(f, "port 0 cannot be scanned"),
            ParseError::InvalidRange { lo, hi } => write!(f, "invalid range {lo}-{hi}: low > high"),
        }
    }
}

fn parse_port(s: &str) -> Result<u16, ParseError> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::NotAPort(s.to_string()));
    }
    let n: u16 = s.parse().map_err(|_| ParseError::NotAPort(s.to_string()))?;
    if n == 0 {
        return Err(ParseError::PortZero);
    }
    Ok(n)
}

pub fn parse(spec: &str) -> Result<Vec<u16>, ParseError> {
    if spec.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut out: BTreeSet<u16> = BTreeSet::new();
    for item in spec.split(',') {
        if item.is_empty() {
            return Err(ParseError::EmptyItem);
        }
        if item.chars().any(|c| c.is_whitespace()) {
            return Err(ParseError::Whitespace(item.to_string()));
        }
        match item.split_once('-') {
            None => {
                let n = parse_port(item)?;
                out.insert(n);
            }
            Some((a, b)) => {
                let lo = parse_port(a)?;
                let hi = parse_port(b)?;
                if lo > hi {
                    return Err(ParseError::InvalidRange { lo, hi });
                }
                for p in lo..=hi {
                    out.insert(p);
                }
            }
        }
    }
    Ok(out.into_iter().collect())
}

pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let (num, unit) = s
        .strip_suffix("ms")
        .map(|n| (n, "ms"))
        .or_else(|| s.strip_suffix('s').map(|n| (n, "s")))
        .or_else(|| s.strip_suffix('m').map(|n| (n, "m")))
        .or_else(|| s.strip_suffix('h').map(|n| (n, "h")))
        .unwrap_or((s, "s"));
    if num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "not a valid duration: {s:?} (expected e.g. 500ms, 2s, 1m; bare numbers mean seconds)"
        ));
    }
    let n: u64 = num
        .parse()
        .map_err(|_| format!("duration out of range: {s:?}"))?;
    let d = match unit {
        "ms" => Duration::from_millis(n),
        "s" => Duration::from_secs(n),
        "m" => Duration::from_secs(n * 60),
        _ => Duration::from_secs(n * 3600),
    };
    if d.is_zero() {
        return Err(format!("duration must be greater than zero: {s:?}"));
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_port() {
        assert_eq!(parse("80"), Ok(vec![80]));
    }

    #[test]
    fn comma_list() {
        assert_eq!(parse("80,443"), Ok(vec![80, 443]));
    }

    #[test]
    fn range() {
        assert_eq!(parse("8080-8082"), Ok(vec![8080, 8081, 8082]));
    }

    #[test]
    fn full_range() {
        let v = parse("1-1024").unwrap();
        assert_eq!(v.len(), 1024);
        assert_eq!(v[0], 1);
        assert_eq!(v[1023], 1024);
    }

    #[test]
    fn dedup_and_sort() {
        assert_eq!(parse("443,80,80,1-5"), Ok(vec![1, 2, 3, 4, 5, 80, 443]));
    }

    #[test]
    fn mixed_spec() {
        assert_eq!(
            parse("80,443,8080-8081,22"),
            Ok(vec![22, 80, 443, 8080, 8081])
        );
    }

    #[test]
    fn max_port() {
        assert_eq!(parse("65535"), Ok(vec![65535]));
    }

    #[test]
    fn empty_spec_rejected() {
        assert_eq!(parse(""), Err(ParseError::Empty));
    }

    #[test]
    fn empty_item_rejected() {
        assert_eq!(parse("80,,443"), Err(ParseError::EmptyItem));
    }

    #[test]
    fn whitespace_rejected() {
        let err = parse("80, 443").unwrap_err();
        assert_eq!(err, ParseError::Whitespace(" 443".to_string()));
    }

    #[test]
    fn port_zero_rejected() {
        assert_eq!(parse("0"), Err(ParseError::PortZero));
    }

    #[test]
    fn out_of_range_rejected() {
        assert!(matches!(parse("65536"), Err(ParseError::NotAPort(_))));
    }

    #[test]
    fn non_numeric_rejected() {
        assert!(matches!(parse("http"), Err(ParseError::NotAPort(_))));
    }

    #[test]
    fn reversed_range_rejected() {
        assert_eq!(
            parse("443-80"),
            Err(ParseError::InvalidRange { lo: 443, hi: 80 })
        );
    }

    #[test]
    fn duration_units() {
        assert_eq!(parse_duration("500ms"), Ok(Duration::from_millis(500)));
        assert_eq!(parse_duration("2s"), Ok(Duration::from_secs(2)));
        assert_eq!(parse_duration("90"), Ok(Duration::from_secs(90)));
        assert_eq!(parse_duration("1m"), Ok(Duration::from_secs(60)));
        assert_eq!(parse_duration("1h"), Ok(Duration::from_secs(3600)));
    }

    #[test]
    fn duration_errors() {
        assert!(parse_duration("1x").is_err());
        assert!(parse_duration("-5s").is_err());
        assert!(parse_duration("0s").is_err());
        assert!(parse_duration("").is_err());
    }
}
