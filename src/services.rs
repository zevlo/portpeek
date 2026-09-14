const SERVICES: &[(u16, &str)] = &[
    (22, "ssh"),
    (25, "smtp"),
    (53, "dns"),
    (80, "http"),
    (110, "pop3"),
    (123, "ntp"),
    (143, "imap"),
    (389, "ldap"),
    (443, "https"),
    (465, "smtps"),
    (587, "submission"),
    (636, "ldaps"),
    (993, "imaps"),
    (995, "pop3s"),
    (1025, "dev-smtp"),
    (1433, "mssql"),
    (1521, "oracle"),
    (1883, "mqtt"),
    (2375, "docker-reg"),
    (2376, "docker-regs"),
    (3000, "node-dev"),
    (3001, "node-dev-alt"),
    (3306, "mysql"),
    (4317, "otlp-grpc"),
    (4318, "otlp-http"),
    (5000, "flask"),
    (5432, "postgres"),
    (5555, "dev-alt"),
    (5601, "kibana"),
    (5672, "amqp"),
    (6379, "redis"),
    (6443, "k8s-api"),
    (8080, "http-alt"),
    (8081, "http-alt2"),
    (8123, "clickhouse"),
    (8443, "https-alt"),
    (8888, "jupyter"),
    (9090, "prometheus"),
    (9092, "kafka"),
    (9200, "elasticsearch"),
    (11211, "memcached"),
    (15672, "rabbit-mgmt"),
    (27017, "mongodb"),
];

pub fn name(port: u16) -> &'static str {
    SERVICES
        .binary_search_by(|(p, _)| p.cmp(&port))
        .ok()
        .map_or("-", |i| SERVICES[i].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ports() {
        assert_eq!(name(22), "ssh");
        assert_eq!(name(5432), "postgres");
        assert_eq!(name(6379), "redis");
        assert_eq!(name(27017), "mongodb");
    }

    #[test]
    fn unknown_port_is_dash() {
        assert_eq!(name(12345), "-");
        assert_eq!(name(1), "-");
    }

    #[test]
    fn table_is_sorted_and_unique() {
        for w in SERVICES.windows(2) {
            assert!(
                w[0].0 < w[1].0,
                "table not strictly increasing at {}",
                w[1].0
            );
        }
    }

    #[test]
    fn table_size() {
        assert!(
            SERVICES.len() >= 40,
            "expected ~40 entries, got {}",
            SERVICES.len()
        );
    }
}
