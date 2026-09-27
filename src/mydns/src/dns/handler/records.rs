use hickory_proto::op::{Header, HeaderCounts, Metadata, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use hickory_server::server::{Request, ResponseInfo};

pub(crate) fn build_record(
    name: &str,
    rtype: RecordType,
    value: &str,
    ttl: u32,
    priority: Option<i64>,
) -> Option<Record> {
    use hickory_proto::rr::rdata::{A, AAAA, CNAME, MX, NS, PTR, SOA, TXT};
    let fqdn_str = if name.ends_with('.') {
        name.to_string()
    } else {
        format!("{}.", name)
    };
    let fqdn: Name = fqdn_str.parse().ok()?;
    let rdata = match rtype {
        RecordType::A => RData::A(A(value.parse().ok()?)),
        RecordType::AAAA => RData::AAAA(AAAA(value.parse().ok()?)),
        RecordType::CNAME => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::CNAME(CNAME(target_str.parse().ok()?))
        }
        RecordType::MX => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::MX(MX::new(
                priority.unwrap_or(10) as u16,
                target_str.parse().ok()?,
            ))
        }
        RecordType::NS => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::NS(NS(target_str.parse().ok()?))
        }
        RecordType::PTR => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::PTR(PTR(target_str.parse().ok()?))
        }
        RecordType::TXT => RData::TXT(TXT::new(vec![value.to_string()])),
        RecordType::SOA => {
            // Expected value format (space-separated):
            //   "<mname>. <rname>. <serial> <refresh> <retry> <expire> <minimum>"
            // Example: "ns1.example.com. hostmaster.example.com. 1 3600 600 86400 300"
            let parts: Vec<&str> = value.split_whitespace().collect();
            if parts.len() != 7 {
                tracing::warn!(value = %value, "SOA record has invalid format; expected 7 fields");
                return None;
            }
            let mname_str = if parts[0].ends_with('.') {
                parts[0].to_string()
            } else {
                format!("{}.", parts[0])
            };
            let rname_str = if parts[1].ends_with('.') {
                parts[1].to_string()
            } else {
                format!("{}.", parts[1])
            };
            let mname: Name = mname_str.parse().ok()?;
            let rname: Name = rname_str.parse().ok()?;
            let serial: u32 = parts[2].parse().ok()?;
            let refresh: i32 = parts[3].parse().ok()?;
            let retry: i32 = parts[4].parse().ok()?;
            let expire: i32 = parts[5].parse().ok()?;
            let minimum: u32 = parts[6].parse().ok()?;
            RData::SOA(SOA::new(
                mname, rname, serial, refresh, retry, expire, minimum,
            ))
        }
        _ => return None,
    };
    Some(Record::from_rdata(fqdn, ttl, rdata))
}

pub(crate) fn failed_response_info(request: &Request) -> ResponseInfo {
    let mut metadata = Metadata::response_from_request(&request.metadata);
    metadata.response_code = ResponseCode::ServFail;
    ResponseInfo::from(Header {
        metadata,
        counts: HeaderCounts::default(),
    })
}
