use std::net::{Ipv4Addr, Ipv6Addr};

use hickory_proto::rr::Name;

use crate::db::records::{CreateRecord, UpdateRecord};
use crate::error::ApiError;

pub const MIN_TTL: u32 = 1;
pub const MAX_TTL: u32 = 86_400;
const MAX_DNS_NAME_LEN: usize = 253;
const MAX_LABEL_LEN: usize = 63;

/// Checks that `name` belongs to one of the configured `allowed_zones`.
///
/// Semantics:
/// - Empty `allowed_zones` → check skipped (allow any name; backwards-compatible
///   with unconfigured deployments).
/// - `allowed_zones` contains `"."` → the root zone is authoritative for the
///   entire namespace; any valid DNS name is accepted.
/// - Otherwise → `name` must be an exact match for a zone or a subdomain of one.
pub fn validate_zone(name: &str, allowed_zones: &[String]) -> Result<(), ApiError> {
    if allowed_zones.is_empty() {
        return Ok(());
    }
    // Root zone "." is authoritative for every name.
    if allowed_zones.iter().any(|z| z == ".") {
        return Ok(());
    }
    let normalized = name.trim().trim_end_matches('.').to_lowercase();
    let matches = allowed_zones.iter().any(|z| {
        let zone = z.trim().trim_end_matches('.').to_lowercase();
        normalized == zone || normalized.ends_with(&format!(".{zone}"))
    });
    if matches {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Record name '{}' does not belong to any allowed zone: [{}]",
            name,
            allowed_zones.join(", ")
        )))
    }
}

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

pub(crate) fn validate_name(raw: &str) -> Result<(), ApiError> {
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

pub(crate) fn validate_record_type(raw: &str) -> Result<(), ApiError> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "A" | "AAAA" | "CNAME" | "MX" | "NS" | "PTR" | "TXT" => Ok(()),
        _ => Err(ApiError::BadRequest(
            "Unsupported record type; supported types are A, AAAA, CNAME, MX, NS, PTR, and TXT"
                .into(),
        )),
    }
}

pub(crate) fn validate_ttl(ttl: u32) -> Result<(), ApiError> {
    if !(MIN_TTL..=MAX_TTL).contains(&ttl) {
        return Err(ApiError::BadRequest(format!(
            "TTL must be between {} and {} seconds",
            MIN_TTL, MAX_TTL
        )));
    }
    Ok(())
}

pub(crate) fn validate_value(record_type: &str, value: &str) -> Result<(), ApiError> {
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
        "CNAME" | "PTR" | "MX" | "NS" => {
            let target = value.trim_end_matches('.');
            if target.is_empty() {
                return Err(ApiError::BadRequest(
                    "Record target must not be empty".into(),
                ));
            }
            validate_name(target).map_err(|_| {
                ApiError::BadRequest("Record target must be a valid DNS name".into())
            })?;
            Ok(())
        }
        "TXT" => {
            if value.is_empty() {
                return Err(ApiError::BadRequest(
                    "TXT record value must not be empty".into(),
                ));
            }
            if value.len() > 255 {
                return Err(ApiError::BadRequest(
                    "TXT record value must not exceed 255 bytes".into(),
                ));
            }
            Ok(())
        }
        _ => unreachable!("record type validated before value"),
    }
}

pub(crate) fn validate_priority(record_type: &str, priority: Option<u16>) -> Result<(), ApiError> {
    if priority.is_some() && !record_type.trim().eq_ignore_ascii_case("MX") {
        return Err(ApiError::BadRequest(
            "Priority is only valid for MX records".into(),
        ));
    }
    Ok(())
}
