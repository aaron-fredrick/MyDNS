use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use colored::Colorize;
use hickory_proto::op::ResponseCode;
use hickory_proto::rr::{Name, RData, Record, RecordType};
use super::{UpstreamResolution, UpstreamResolver};
use super::forward::query_resolver;

async fn raw_dns_query(
    server: SocketAddr,
    name: &Name,
    rtype: RecordType,
) -> Option<hickory_proto::op::Message> {
    use hickory_proto::op::{Message, MessageType, OpCode, Query as DnsQuery};
    let id = rand::random::<u16>();
    let mut msg = Message::new(id, MessageType::Query, OpCode::Query);
    msg.metadata.recursion_desired = false;
    msg.add_query(DnsQuery::query(name.clone(), rtype));
    let bytes = msg.to_vec().ok()?;
    let bind_addr = if server.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let socket = tokio::net::UdpSocket::bind(bind_addr).await.ok()?;
    socket.send_to(&bytes, server).await.ok()?;
    let mut recv_buf = vec![0u8; 4096];
    let timeout = Duration::from_secs(2);
    let start = tokio::time::Instant::now();
    let udp_response = loop {
        let elapsed = start.elapsed();
        if elapsed >= timeout { return None; }
        let recv_result = tokio::time::timeout(timeout - elapsed, socket.recv_from(&mut recv_buf)).await;
        let (len, src_addr) = match recv_result { Ok(Ok(res)) => res, _ => return None };
        if src_addr != server { continue; }
        let parsed = match Message::from_vec(&recv_buf[..len]) { Ok(p) => p, Err(_) => continue };
        if parsed.id != id || parsed.message_type != MessageType::Response || parsed.op_code != OpCode::Query { continue; }
        let has_matching_query = parsed.queries.iter().any(|q| q.name() == name && (q.query_type() == rtype || rtype == hickory_proto::rr::RecordType::ANY));
        if !has_matching_query && !parsed.queries.is_empty() { continue; }
        break parsed;
    };
    if udp_response.metadata.truncation { return raw_dns_query_tcp(server, &bytes, id).await; }
    Some(udp_response)
}
async fn raw_dns_query_tcp(server: SocketAddr, query_bytes: &[u8], expected_id: u16) -> Option<hickory_proto::op::Message> {
    use hickory_proto::op::Message;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::time::timeout(Duration::from_secs(4), tokio::net::TcpStream::connect(server)).await.ok()?.ok()?;
    let len_prefix = (query_bytes.len() as u16).to_be_bytes();
    stream.write_all(&len_prefix).await.ok()?;
    stream.write_all(query_bytes).await.ok()?;
    let response_len = tokio::time::timeout(Duration::from_secs(4), stream.read_u16()).await.ok()?.ok()? as usize;
    let mut recv_buf = vec![0u8; response_len];
    tokio::time::timeout(Duration::from_secs(4), stream.read_exact(&mut recv_buf)).await.ok()?.ok()?;
    let parsed = Message::from_vec(&recv_buf).ok()?;
    if parsed.id != expected_id { return None; }
    Some(parsed)
}
impl UpstreamResolver {
    #[tracing::instrument(
        name = "resolve_iterative",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype, root_hints_count = self.root_hints.len()),
        skip(self)
    )]
    pub(super) async fn resolve_iterative(&self, name: &Name, rtype: RecordType) -> UpstreamResolution {
        let mut current_servers = self.root_hints.clone();
        eprintln!(
            "{} Starting iterative recursion for {} (type {}) with {} root hints",
            "[RECURSE START]".magenta().bold(),
            name.to_string().cyan().bold(),
            rtype.to_string().yellow().bold(),
            current_servers.len()
        );

        for depth in 0..super::MAX_RECURSION_DEPTH {
            let depth_span = tracing::span!(
                tracing::Level::DEBUG,
                "recursion_depth",
                depth = depth,
                servers_count = current_servers.len()
            );
            let _depth_enter = depth_span.enter();

            let mut next_servers: Vec<SocketAddr> = Vec::new();
            let mut ns_names: Vec<Name> = Vec::new();
            let mut got_response = false;

            let level_tag = match depth {
                0 => "ROOT",
                1 => "TLD",
                _ => "AUTH/DELEGATION",
            };
            tracing::debug!(level = %level_tag, "Starting recursion depth");

            let step_indent = "  ".repeat(depth);
            let res_indent = format!("{}  ↳", step_indent);

            // Try up to 3 servers at this delegation level
            for (idx, &server) in current_servers.iter().take(3).enumerate() {
                let server_span = tracing::span!(
                    tracing::Level::DEBUG,
                    "server_query",
                    server = %server,
                    candidate_index = idx + 1,
                    total_candidates = current_servers.len().min(3)
                );
                let _server_enter = server_span.enter();

                eprintln!(
                    "{}{} Querying {} (candidate {}/{}) for {}",
                    step_indent,
                    format!("[RECURSE step {} | {}]", depth, level_tag)
                        .magenta()
                        .bold(),
                    server.to_string().cyan().bold(),
                    idx + 1,
                    current_servers.len().min(3),
                    name.to_string().bold()
                );

                let Some(response) = raw_dns_query(server, name, rtype).await else {
                    tracing::warn!(server = %server, "Server query timed out");
                    eprintln!(
                        "{} {} Server {} timed out, trying next server...",
                        res_indent,
                        "[RECURSE TIMEOUT]".yellow().bold(),
                        server.to_string().cyan().bold()
                    );
                    continue;
                };
                got_response = true;
                tracing::debug!(server = %server, "Server query succeeded");

                // Authoritative answer
                if !response.answers.is_empty() {
                    let records: Vec<Record> = response.answers.clone();
                    let ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300);
                    let values = records
                        .iter()
                        .map(|r| r.data.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    tracing::debug!(
                        server = %server,
                        record_count = records.len(),
                        ttl = ttl,
                        "Authoritative answer received"
                    );
                    eprintln!(
                        "{} {} Authoritative answer from {}: {} (TTL={}s)",
                        res_indent,
                        "[RECURSE ANSWER]".green().bold(),
                        server.to_string().cyan().bold(),
                        format!("[{}]", values).green().bold(),
                        ttl
                    );
                    return UpstreamResolution::Positive(records, ttl);
                }

                match response.metadata.response_code {
                    ResponseCode::NXDomain => {
                        tracing::debug!(server = %server, "Server returned NXDOMAIN");
                        eprintln!(
                            "{} {} Server {} returned NXDOMAIN for {}",
                            res_indent,
                            "[RECURSE NXDOMAIN]".red().bold(),
                            server.to_string().cyan().bold(),
                            name.to_string().bold()
                        );
                        return UpstreamResolution::NxDomain;
                    }
                    ResponseCode::NoError => {
                        tracing::debug!(server = %server, "Server returned NoError");
                    }
                    _ => {
                        tracing::warn!(server = %server, response_code = ?response.metadata.response_code, "Server returned error code");
                        continue;
                    }
                }

                // Check if this is an authoritative NODATA (NoError, 0 answers, SOA in authority, no NS)
                let has_soa = response
                    .authorities
                    .iter()
                    .any(|r| matches!(r.data, RData::SOA(_)));
                let has_ns = response
                    .authorities
                    .iter()
                    .any(|r| matches!(r.data, RData::NS(_)));

                if has_soa && !has_ns {
                    tracing::debug!(server = %server, "Server returned NODATA");
                    eprintln!(
                        "{} {} Server {} returned NODATA for {} (type {})",
                        res_indent,
                        "[RECURSE NODATA]".yellow().bold(),
                        server.to_string().cyan().bold(),
                        name.to_string().bold(),
                        rtype
                    );
                    return UpstreamResolution::Nodata;
                }

                // Gather valid NS hostnames first (with bailiwick validation)
                for rec in &response.authorities {
                    if let RData::NS(ns) = &rec.data {
                        // Bailiwick check: the NS record's domain must be a super-domain of the queried name
                        if rec.name.zone_of(name) {
                            ns_names.push(ns.0.clone());
                        }
                    }
                }

                // Extract glue A/AAAA records from the additional section
                for rec in &response.additionals {
                    // Bailiwick check: the glue record name MUST match one of the valid NS names
                    if ns_names.contains(&rec.name) {
                        match &rec.data {
                            RData::A(a) => {
                                next_servers.push(SocketAddr::new(IpAddr::V4(a.0), 53));
                            }
                            RData::AAAA(aaaa) => {
                                next_servers.push(SocketAddr::new(IpAddr::V6(aaaa.0), 53));
                            }
                            _ => {}
                        }
                    }
                }

                if !next_servers.is_empty() {
                    tracing::debug!(
                        server = %server,
                        glue_count = next_servers.len(),
                        "Extracted valid glue records"
                    );
                } else if !ns_names.is_empty() {
                    tracing::debug!(
                        server = %server,
                        ns_count = ns_names.len(),
                        "No valid glue, need to resolve NS hostnames"
                    );
                }

                if !next_servers.is_empty() {
                    let glue_str = next_servers
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    tracing::debug!(
                        server = %server,
                        next_hops = %glue_str,
                        "Referral with glue records"
                    );
                    eprintln!(
                        "{} {} Server {} referred with {} glue IPs: {}",
                        res_indent,
                        "[RECURSE REFERRAL]".blue().bold(),
                        server.to_string().cyan().bold(),
                        next_servers.len(),
                        format!("[{}]", glue_str).cyan().bold()
                    );
                    break;
                }
            }

            let step_indent = "  ".repeat(depth);
            let res_indent = format!("{}  ↳", step_indent);

            if !got_response {
                tracing::error!(depth = depth, "All servers timed out at this depth");
                eprintln!(
                    "{} {} All iterative servers timed out at step {} for query {}",
                    res_indent,
                    "[RECURSE FAILED]".red().bold(),
                    depth,
                    name
                );
                return UpstreamResolution::ServFail;
            }

            // No glue in the additional section — resolve the NS hostnames via
            // the forwarding resolver (Cloudflare) and use those IPs.
            if next_servers.is_empty() {
                let ns_str = ns_names
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                tracing::debug!(ns_names = %ns_str, "Resolving un-glued NS hostnames");
                eprintln!(
                    "{} {} Un-glued referral to NS [{}]; resolving NS IP addresses via upstream...",
                    res_indent,
                    "[RECURSE RESOLVE-NS]".blue().bold(),
                    ns_str
                );

                for ns_name in ns_names.iter().take(2) {
                    // Prefer IPv4 glue; also collect IPv6 for full NS reachability.
                    if let UpstreamResolution::Positive(a_records, _) =
                        query_resolver(&self.cloudflare, ns_name, RecordType::A).await
                    {
                        for rec in &a_records {
                            if let RData::A(a) = &rec.data {
                                next_servers.push(SocketAddr::new(IpAddr::V4(a.0), 53));
                            }
                        }
                    }

                    if let UpstreamResolution::Positive(aaaa_records, _) =
                        query_resolver(&self.cloudflare, ns_name, RecordType::AAAA).await
                    {
                        for rec in &aaaa_records {
                            if let RData::AAAA(aaaa) = &rec.data {
                                next_servers.push(SocketAddr::new(IpAddr::V6(aaaa.0), 53));
                            }
                        }
                    }

                    tracing::debug!(
                        ns_name = %ns_name,
                        resolved_count = next_servers.len(),
                        "Resolved NS hostname (A + AAAA)"
                    );

                    if !next_servers.is_empty() {
                        break;
                    }
                }
            }

            if next_servers.is_empty() {
                tracing::error!(depth = depth, "Could not resolve any next-hop servers");
                eprintln!(
                    "{} {} Referral with no resolvable next-hop servers at step {} for {}",
                    res_indent,
                    "[RECURSE FAILED]".red().bold(),
                    depth,
                    name
                );
                return UpstreamResolution::ServFail;
            }

            current_servers = next_servers;
        }

        tracing::error!("Iterative resolution exceeded maximum depth");
        eprintln!(
            "{} Iterative resolution exceeded maximum depth for {}",
            "[RECURSE LIMIT]".red().bold(),
            name
        );
        UpstreamResolution::ServFail
    }
}

#[tracing::instrument(
    name = "query_resolver",
    level = tracing::Level::DEBUG,
    fields(name = %name, rtype = ?rtype)
    }
}
