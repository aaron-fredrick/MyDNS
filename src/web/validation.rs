use std::net::{Ipv4Addr, Ipv6Addr};

use hickory_proto::rr::Name;

use crate::db::records::{CreateRecord, UpdateRecord};
use crate::web::error::ApiError;

pub const MIN_TTL: u32 = 1;
pub const MAX_TTL: u32 = 86_400;
const MAX_DNS_NAME_LEN: usize = 253;
const MAX_LABEL_LEN: usize = 63;

pub fn validate_create_record(req: &CreateRecord) -> Result<(), ApiError> {
    validate_record(
        &req.name,
        &req.record_type,
        &req.value,
        req.ttl,
        req.priority,
    )
}

pub fn validate_update_record(req: &UpdateRecord) -> Result<(), ApiError> {
    if let Some(name) = req.name.as_deref() {
        validate_name(name)?;
    }
    if let Some(record_type) = req.record_type.as_deref() {
        validate_record_type(record_type)?;
    }
    if let Some(ttl) = req.ttl {
        validate_ttl(ttl)?;
    }
    if let Some(priority) = req.priority {
        if let Some(record_type) = req.record_type.as_deref() {
            validate_priority(record_type, Some(priority))?;
        }
    }
    Ok(())
}

pub fn validate_record(
    name: &str,
    record_type: &str,
    value: &str,
    ttl: u32,
    priority: Option<u16>,
) -> Result<(), ApiError> {
    validate_name(name)?;
    validate_record_type(record_type)?;
    validate_ttl(ttl)?;
    validate_value(record_type, value)?;
    validate_priority(record_type, priority)?;
    Ok(())
}

fn validate_name(raw: &str) -> Result<(), ApiError> {
    let name = raw.trim().trim_end_matches('.');
    if name.is_empty() {
        return Err(ApiError::BadRequest("DNS name must not be empty".into()));
    }
    if name.len() > MAX_DNS_NAME_LEN {
        return Err(ApiError::BadRequest(
            "DNS name exceeds 253 characters".into(),
        ));
    }
    if name == "@" || name.contains('*') {
        return Err(ApiError::BadRequest(
            "Wildcard and zone-apex shorthand names are not supported".into(),
        ));
    }
    if name
        .split('.')
        .any(|label| label.is_empty() || label.len() > MAX_LABEL_LEN)
    {
        return Err(ApiError::BadRequest(
            "DNS name contains an empty or oversized label".into(),
        ));
    }
    if name
        .split('.')
        .any(|label| label.starts_with('-') || label.ends_with('-'))
    {
        return Err(ApiError::BadRequest(
            "DNS labels must not start or end with '-'".into(),
        ));
    }
    if name
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.'))
    {
        return Err(ApiError::BadRequest(
            "DNS name contains unsupported characters".into(),
        ));
    }
    format!("{}.", name)
        .parse::<Name>()
        .map_err(|_| ApiError::BadRequest("Invalid DNS name".into()))?;
    Ok(())
}

fn validate_record_type(raw: &str) -> Result<(), ApiError> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "A" | "AAAA" | "CNAME" | "MX" | "PTR" => Ok(()),
        _ => Err(ApiError::BadRequest(
            "Unsupported record type; supported types are A, AAAA, CNAME, MX, and PTR".into(),
        )),
    }
}

fn validate_ttl(ttl: u32) -> Result<(), ApiError> {
    if !(MIN_TTL..=MAX_TTL).contains(&ttl) {
        return Err(ApiError::BadRequest(format!(
            "TTL must be between {} and {} seconds",
            MIN_TTL, MAX_TTL
        )));
    }
    Ok(())
}

fn validate_value(record_type: &str, value: &str) -> Result<(), ApiError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApiError::BadRequest(
            "Record value must not be empty".into(),
        ));
    }

    match record_type.trim().to_ascii_uppercase().as_str() {
        "A" => value.parse::<Ipv4Addr>().map(|_| ()).map_err(|_| {
            ApiError::BadRequest("A record value must be a valid IPv4 address".into())
        }),
        "AAAA" => value.parse::<Ipv6Addr>().map(|_| ()).map_err(|_| {
            ApiError::BadRequest("AAAA record value must be a valid IPv6 address".into())
        }),
        "CNAME" | "PTR" | "MX" => {
            let target = value.trim_end_matches('.');
            if target.is_empty() {
                return Err(ApiError::BadRequest(
                    "Record target must not be empty".into(),
                ));
            }
            format!("{}.", target).parse::<Name>().map_err(|_| {
                ApiError::BadRequest("Record target must be a valid DNS name".into())
            })?;
            Ok(())
        }
        _ => unreachable!("record type validated before value"),
    }
}

fn validate_priority(record_type: &str, priority: Option<u16>) -> Result<(), ApiError> {
    if priority.is_some() && !record_type.trim().eq_ignore_ascii_case("MX") {
        return Err(ApiError::BadRequest(
            "Priority is only valid for MX records".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(record_type: &str, value: &str) -> CreateRecord {
        CreateRecord {
            name: "example.local".into(),
            record_type: record_type.into(),
            value: value.into(),
            ttl: 300,
            priority: None,
        }
    }

    #[test]
    fn accepts_supported_records() {
        assert!(validate_create_record(&create("A", "192.0.2.1")).is_ok());
        assert!(validate_create_record(&create("AAAA", "2001:db8::1")).is_ok());
        assert!(validate_create_record(&create("CNAME", "target.local.")).is_ok());
        assert!(validate_create_record(&create("PTR", "host.local.")).is_ok());
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
        assert!(validate_create_record(&create("TXT", "hello")).is_err());
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
    }
}
