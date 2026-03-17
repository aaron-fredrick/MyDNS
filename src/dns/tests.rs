#[cfg(test)]
mod dns_handler_tests {
    use hickory_proto::rr::RecordType;
    use crate::dns::handler::build_record;

    #[test]
    fn build_a_record_from_valid_ip() {
        let record = build_record("example.com.", RecordType::A, "1.2.3.4", 300, None);
        assert!(record.is_some(), "Should build A record from valid IPv4");
        let r = record.unwrap();
        assert_eq!(r.ttl(), 300);
        assert_eq!(r.record_type(), RecordType::A);
    }

    #[test]
    fn build_a_record_from_invalid_ip_returns_none() {
        let record = build_record("example.com.", RecordType::A, "not-an-ip", 300, None);
        assert!(record.is_none(), "Invalid IP should yield None");
    }

    #[test]
    fn build_aaaa_record() {
        let record = build_record("example.com.", RecordType::AAAA, "::1", 60, None);
        assert!(record.is_some());
        assert_eq!(record.unwrap().record_type(), RecordType::AAAA);
    }

    #[test]
    fn build_cname_record() {
        let record = build_record("alias.example.com.", RecordType::CNAME, "target.example.com.", 300, None);
        assert!(record.is_some());
        assert_eq!(record.unwrap().record_type(), RecordType::CNAME);
    }

    #[test]
    fn build_mx_record_with_priority() {
        let record = build_record("example.com.", RecordType::MX, "mail.example.com.", 300, Some(20));
        assert!(record.is_some());
        let r = record.unwrap();
        assert_eq!(r.record_type(), RecordType::MX);
    }
}
