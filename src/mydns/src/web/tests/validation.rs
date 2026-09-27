use crate::db::records::{CreateRecord, UpdateRecord};
use crate::mydns::web::validation::{
    validate_create_record, validate_name, validate_priority, validate_record,
    validate_record_type, validate_ttl, validate_update_record, validate_value, validate_zone,
    MAX_TTL, MIN_TTL,
};

fn create(record_type: &str, value: &str) -> CreateRecord {
    CreateRecord {
        name: "test.example.com".into(),
        record_type: record_type.into(),
        value: value.into(),
        ttl: 300,
        priority: None,
        is_dev: false,
    }
}

#[test]
fn accepts_supported_records() {
    assert!(validate_create_record(&create("A", "192.0.2.1")).is_ok());
    assert!(validate_create_record(&create("AAAA", "2001:db8::1")).is_ok());
    assert!(validate_create_record(&create("CNAME", "target.local.")).is_ok());
    assert!(validate_create_record(&create("PTR", "host.local.")).is_ok());
    assert!(validate_create_record(&create("NS", "ns1.example.com.")).is_ok());
    assert!(validate_create_record(&create("TXT", "v=spf1 include:example.com ~all")).is_ok());
    let mut mx = create("MX", "mail.local.");
    mx.priority = Some(10);
    assert!(validate_create_record(&mx).is_ok());
}

#[test]
fn rejects_invalid_names_and_values() {
    assert!(validate_create_record(&create("A", "not-an-ip")).is_err());
    let mut invalid = create("A", "192.0.2.1");
    invalid.name = "bad..name.local".into();
    assert!(validate_create_record(&invalid).is_err());
}

#[test]
fn rejects_unsupported_types_and_bad_ttls() {
    // SRV and SPF are not supported record types.
    assert!(validate_create_record(&create("SRV", "hello")).is_err());
    assert!(validate_create_record(&create("SPF", "hello")).is_err());
    let mut invalid = create("A", "192.0.2.1");
    invalid.ttl = 0;
    assert!(validate_create_record(&invalid).is_err());
    invalid.ttl = MAX_TTL + 1;
    assert!(validate_create_record(&invalid).is_err());
}

#[test]
fn rejects_non_mx_priority() {
    let mut invalid = create("A", "192.0.2.1");
    invalid.priority = Some(10);
    assert!(validate_create_record(&invalid).is_err());

    let mut aaaa_invalid = create("AAAA", "2001:db8::1");
    aaaa_invalid.priority = Some(5);
    assert!(validate_create_record(&aaaa_invalid).is_err());

    let mut txt_invalid = create("TXT", "some text");
    txt_invalid.priority = Some(1);
    assert!(validate_create_record(&txt_invalid).is_err());
}

#[test]
fn rejects_oversized_txt_value() {
    // 256 bytes exceeds the single-string limit.
    let long_value = "x".repeat(256);
    assert!(validate_create_record(&create("TXT", &long_value)).is_err());
}

#[test]
fn rejects_empty_txt_value() {
    assert!(validate_create_record(&create("TXT", "")).is_err());
    assert!(validate_create_record(&create("TXT", "   ")).is_err());
}

#[test]
fn rejects_invalid_dns_name_shapes() {
    for name in [
        "",
        ".",
        "@",
        "*.example.local",
        "bad..example.local",
        "-bad.example.local",
        "bad-.example.local",
        "bad_name.example.local",
    ] {
        let mut req = create("A", "192.0.2.1");
        req.name = name.into();
        assert!(
            validate_create_record(&req).is_err(),
            "accepted invalid name {name:?}"
        );
    }

    let oversized_label = format!("{}.example.local", "x".repeat(64));
    let mut req = create("A", "192.0.2.1");
    req.name = oversized_label;
    assert!(validate_create_record(&req).is_err());

    let oversized_name = format!("{}.example.local", "x".repeat(240));
    req.name = oversized_name;
    assert!(validate_create_record(&req).is_err());
}

#[test]
fn accepts_boundary_ttls() {
    let mut req = create("A", "192.0.2.1");
    req.ttl = MIN_TTL;
    assert!(validate_create_record(&req).is_ok());
    req.ttl = MAX_TTL;
    assert!(validate_create_record(&req).is_ok());
}

#[test]
fn validates_all_supported_value_types() {
    assert!(validate_create_record(&create("A", "192.0.2.1")).is_ok());
    assert!(validate_create_record(&create("AAAA", "2001:db8::1")).is_ok());
    assert!(validate_create_record(&create("CNAME", "target.example.local.")).is_ok());
    assert!(validate_create_record(&create("PTR", "host.example.local")).is_ok());
    assert!(validate_create_record(&create("MX", "mail.example.local")).is_ok());
    assert!(validate_create_record(&create("NS", "ns1.example.local")).is_ok());
}

#[test]
fn rejects_empty_values_and_invalid_address_or_target_values() {
    for record_type in ["A", "AAAA", "CNAME", "PTR", "MX", "NS", "TXT"] {
        assert!(
            validate_create_record(&create(record_type, "   ")).is_err(),
            "empty value accepted for {record_type}"
        );
    }

    assert!(validate_create_record(&create("A", "999.999.999.999")).is_err());
    assert!(validate_create_record(&create("AAAA", "192.0.2.1")).is_err());
    assert!(validate_create_record(&create("AAAA", "invalid:ipv6::address::extra")).is_err());
    assert!(validate_create_record(&create("CNAME", ".")).is_err());
    assert!(validate_create_record(&create("CNAME", "")).is_err());
    assert!(validate_create_record(&create("MX", "not a dns name")).is_err());
    assert!(validate_create_record(&create("NS", "bad_name.example")).is_err());
    assert!(validate_create_record(&create("PTR", "bad_name.example")).is_err());
}

#[test]
fn accepts_txt_at_255_bytes_and_rejects_256_bytes() {
    assert!(validate_create_record(&create("TXT", &"x".repeat(255))).is_ok());
    assert!(validate_create_record(&create("TXT", &"x".repeat(256))).is_err());
}

#[test]
fn update_validation_checks_only_supplied_fields() {
    let req = UpdateRecord {
        name: Some("valid.example.local".into()),
        record_type: Some("a".into()),
        value: None,
        ttl: Some(MIN_TTL),
        priority: None,
    };
    assert!(validate_update_record(&req).is_ok());

    let invalid = UpdateRecord {
        name: Some("bad..name".into()),
        record_type: None,
        value: None,
        ttl: None,
        priority: None,
    };
    assert!(validate_update_record(&invalid).is_err());

    let invalid_ttl = UpdateRecord {
        name: None,
        record_type: None,
        value: None,
        ttl: Some(0),
        priority: None,
    };
    assert!(validate_update_record(&invalid_ttl).is_err());

    let invalid_type = UpdateRecord {
        name: None,
        record_type: Some("INVALID".into()),
        value: None,
        ttl: None,
        priority: None,
    };
    assert!(validate_update_record(&invalid_type).is_err());
}

#[test]
fn update_priority_is_validated_when_record_type_is_supplied() {
    let valid = UpdateRecord {
        name: None,
        record_type: Some("mx".into()),
        value: None,
        ttl: None,
        priority: Some(10),
    };
    assert!(validate_update_record(&valid).is_ok());

    let invalid = UpdateRecord {
        name: None,
        record_type: Some("A".into()),
        value: None,
        ttl: None,
        priority: Some(10),
    };
    assert!(validate_update_record(&invalid).is_err());

    // Priority without record_type is accepted in update (record_type checked if provided)
    let priority_only = UpdateRecord {
        name: None,
        record_type: None,
        value: None,
        ttl: None,
        priority: Some(10),
    };
    assert!(validate_update_record(&priority_only).is_ok());
}

#[test]
fn zone_matching_is_case_insensitive_and_respects_label_boundaries() {
    let zones = vec!["Example.COM".to_string()];
    assert!(validate_zone("HOST.example.com.", &zones).is_ok());
    assert!(validate_zone("notexample.com", &zones).is_err());
    assert!(validate_zone("example.com", &zones).is_ok());
}

#[test]
fn zone_allow_all_when_list_is_empty() {
    assert!(validate_zone("anything.example.com", &[]).is_ok());
}

#[test]
fn valid_zone() {
    assert!(validate_zone("host.example.com", &["example.com".into()]).is_ok());
}

#[test]
fn root_zone_allows_any_name() {
    assert!(validate_zone("google.com", &[".".into()]).is_ok());
    assert!(validate_zone("home.local", &[".".into()]).is_ok());
    assert!(validate_zone("host.example.com", &["example.com".into(), ".".into()]).is_ok());
}

#[test]
fn zone_accepts_exact_zone_match() {
    let zones = vec!["home.local".to_string()];
    assert!(validate_zone("home.local", &zones).is_ok());
}

#[test]
fn zone_accepts_subdomain_of_allowed_zone() {
    let zones = vec!["home.local".to_string()];
    assert!(validate_zone("server.home.local", &zones).is_ok());
    assert!(validate_zone("deep.sub.home.local", &zones).is_ok());
}

#[test]
fn zone_rejects_name_not_in_any_zone() {
    let zones = vec!["home.local".to_string(), "lab.local".to_string()];
    assert!(validate_zone("server.corp.example", &zones).is_err());
}

#[test]
fn zone_handles_trailing_dot_in_name() {
    let zones = vec!["home.local".to_string()];
    assert!(validate_zone("server.home.local.", &zones).is_ok());
}

#[test]
fn direct_validation_helpers() {
    assert!(validate_name("valid.example.com").is_ok());
    assert!(validate_record_type("a").is_ok());
    assert!(validate_record_type("AAAA").is_ok());
    assert!(validate_record_type("cname").is_ok());
    assert!(validate_record_type("mx").is_ok());
    assert!(validate_record_type("ns").is_ok());
    assert!(validate_record_type("ptr").is_ok());
    assert!(validate_record_type("txt").is_ok());
    assert!(validate_record_type("unknown").is_err());

    assert!(validate_ttl(100).is_ok());
    assert!(validate_ttl(0).is_err());
    assert!(validate_ttl(100_000).is_err());

    assert!(validate_value("A", "10.0.0.1").is_ok());
    assert!(validate_value("AAAA", "::1").is_ok());
    assert!(validate_value("CNAME", "target.com").is_ok());
    assert!(validate_value("PTR", "host.com").is_ok());
    assert!(validate_value("MX", "mail.com").is_ok());
    assert!(validate_value("NS", "ns.com").is_ok());
    assert!(validate_value("TXT", "sample").is_ok());

    assert!(validate_priority("MX", Some(10)).is_ok());
    assert!(validate_priority("MX", None).is_ok());
    assert!(validate_priority("A", None).is_ok());
    assert!(validate_priority("A", Some(10)).is_err());

    assert!(validate_record("host.example.com", "A", "1.2.3.4", 300, None).is_ok());
}
