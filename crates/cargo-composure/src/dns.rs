//! Just enough DNS for the quick tunnel (ADR 0018): the zone's nameservers from a public resolver,
//! then the tunnel's A record from those nameservers directly. The tunnel's hostname never goes to
//! a recursive resolver, which would cache "no such host" for a minute.

use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{net::UdpSocket, time::timeout};

const TYPE_A: u16 = 1;
const TYPE_NS: u16 = 2;
const CLASS_IN: u16 = 1;
const RCODE_NX_DOMAIN: u8 = 3;
/// A name in a packet is at most this many compression pointers deep
const MAX_POINTERS: usize = 16;

/// Asked for the zone's nameservers, never for the tunnel's hostname
const RESOLVERS: [IpAddr; 2] = [
    IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
    IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
];
const QUERY_TIMEOUT: Duration = Duration::from_secs(2);

/// The DNS the quick tunnel asks; tests script it
pub trait Dns: Send + Sync {
    /// The addresses of `zone`'s nameservers
    fn nameservers(&self, zone: &str) -> impl Future<Output = Result<Vec<IpAddr>, String>> + Send;

    /// `host`'s first A record, asked of `servers` directly
    fn a_record(
        &self,
        servers: &[IpAddr],
        host: &str,
    ) -> impl Future<Output = Result<Option<Ipv4Addr>, String>> + Send;
}

/// DNS over UDP
pub struct UdpDns;

impl Dns for UdpDns {
    async fn nameservers(&self, zone: &str) -> Result<Vec<IpAddr>, String> {
        let records = ask(&RESOLVERS, zone, TYPE_NS, true).await?;
        let mut addresses = Vec::new();
        for record in records {
            if let Record::Ns(name) = record {
                // Ordinary lookups: these names exist, unlike the tunnel's a moment ago
                if let Ok(found) = tokio::net::lookup_host((name.as_str(), 53)).await {
                    addresses.extend(found.map(|a| a.ip()).filter(IpAddr::is_ipv4));
                }
            }
        }
        match addresses.is_empty() {
            true => Err(format!("no address for {zone}'s nameservers")),
            false => Ok(addresses),
        }
    }

    async fn a_record(&self, servers: &[IpAddr], host: &str) -> Result<Option<Ipv4Addr>, String> {
        let records = ask(servers, host, TYPE_A, false).await?;
        Ok(records.into_iter().find_map(|record| match record {
            Record::A(ip) => Some(ip),
            Record::Ns(_) => None,
        }))
    }
}

/// The first answer from `servers`, tried in order
async fn ask(
    servers: &[IpAddr],
    name: &str,
    record_type: u16,
    recursive: bool,
) -> Result<Vec<Record>, String> {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |t| t.subsec_nanos() as u16);
    let query = encode_query(id, name, record_type, recursive);
    let mut last = "no servers".to_string();
    for server in servers {
        match exchange(*server, &query).await {
            Ok(response) => return parse_response(&response, id),
            Err(error) => last = format!("{server}: {error}"),
        }
    }
    Err(last)
}

async fn exchange(server: IpAddr, query: &[u8]) -> io::Result<Vec<u8>> {
    let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?;
    socket.connect((server, 53)).await?;
    socket.send(query).await?;
    let mut buffer = vec![0; 1232];
    let received = timeout(QUERY_TIMEOUT, socket.recv(&mut buffer))
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
    buffer.truncate(received);
    Ok(buffer)
}

fn encode_query(id: u16, name: &str, record_type: u16, recursive: bool) -> Vec<u8> {
    let mut packet = Vec::with_capacity(name.len() + 18);
    packet.extend(id.to_be_bytes());
    packet.extend([if recursive { 1 } else { 0 }, 0]); // flags: RD
    packet.extend([0, 1, 0, 0, 0, 0, 0, 0]); // one question
    for label in name.trim_end_matches('.').split('.') {
        packet.push(label.len() as u8);
        packet.extend(label.as_bytes());
    }
    packet.push(0);
    packet.extend(record_type.to_be_bytes());
    packet.extend(CLASS_IN.to_be_bytes());
    packet
}

#[derive(Debug, PartialEq, Eq)]
enum Record {
    A(Ipv4Addr),
    Ns(String),
}

/// The A and NS records in the answer section; none for a name that doesn't exist yet
fn parse_response(packet: &[u8], id: u16) -> Result<Vec<Record>, String> {
    let malformed = || "malformed DNS response".to_string();
    let header = packet.get(..12).ok_or_else(malformed)?;
    if header[..2] != id.to_be_bytes() {
        return Err("DNS response for another query".to_string());
    }
    match header[3] & 0x0f {
        0 => {}
        RCODE_NX_DOMAIN => return Ok(Vec::new()),
        code => return Err(format!("DNS error {code}")),
    }
    let questions = u16::from_be_bytes([header[4], header[5]]);
    let answers = u16::from_be_bytes([header[6], header[7]]);

    let mut at = 12;
    for _ in 0..questions {
        at = skip_name(packet, at).ok_or_else(malformed)? + 4;
    }
    let mut records = Vec::new();
    for _ in 0..answers {
        at = skip_name(packet, at).ok_or_else(malformed)?;
        let fixed = packet.get(at..at + 10).ok_or_else(malformed)?;
        let record_type = u16::from_be_bytes([fixed[0], fixed[1]]);
        let length = u16::from_be_bytes([fixed[8], fixed[9]]) as usize;
        at += 10;
        let data = packet.get(at..at + length).ok_or_else(malformed)?;
        match record_type {
            TYPE_A => {
                let octets: [u8; 4] = data.try_into().map_err(|_| malformed())?;
                records.push(Record::A(octets.into()));
            }
            TYPE_NS => records.push(Record::Ns(read_name(packet, at).ok_or_else(malformed)?)),
            _ => {}
        }
        at += length;
    }
    Ok(records)
}

/// The offset after the name at `at`
fn skip_name(packet: &[u8], mut at: usize) -> Option<usize> {
    loop {
        match *packet.get(at)? {
            0 => return Some(at + 1),
            length if length & 0xc0 == 0xc0 => return Some(at + 2),
            length => at += 1 + length as usize,
        }
    }
}

fn read_name(packet: &[u8], mut at: usize) -> Option<String> {
    let mut labels: Vec<String> = Vec::new();
    let mut pointers = 0;
    loop {
        let length = *packet.get(at)?;
        match length {
            0 => return Some(labels.join(".")),
            length if length & 0xc0 == 0xc0 => {
                pointers += 1;
                if pointers > MAX_POINTERS {
                    return None;
                }
                at = u16::from_be_bytes([length & 0x3f, *packet.get(at + 1)?]) as usize;
            }
            length => {
                let label = packet.get(at + 1..at + 1 + length as usize)?;
                labels.push(String::from_utf8_lossy(label).into_owned());
                at += 1 + length as usize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A response to `encode_query(7, "a.io", ..)` with these answer records, each named by a
    /// pointer to the question
    fn response(flags: u8, answers: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut packet = encode_query(7, "a.io", TYPE_A, false);
        packet[2] = 0x80;
        packet[3] = flags;
        packet[7] = answers.len() as u8;
        for (record_type, data) in answers {
            packet.extend([0xc0, 12]);
            packet.extend(record_type.to_be_bytes());
            packet.extend(CLASS_IN.to_be_bytes());
            packet.extend([0, 0, 0, 60]);
            packet.extend((data.len() as u16).to_be_bytes());
            packet.extend(data);
        }
        packet
    }

    #[test]
    fn queries_are_well_formed() {
        assert_eq!(
            encode_query(0x0102, "a.io", TYPE_A, true),
            [
                1, 2, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, // header, RD set
                1, b'a', 2, b'i', b'o', 0, // name
                0, 1, 0, 1, // A, IN
            ]
        );
        // Asked of an authoritative server: no recursion
        assert_eq!(encode_query(1, "a.io", TYPE_A, false)[2], 0);
    }

    #[test]
    fn reads_a_records() {
        let packet = response(0, &[(TYPE_A, vec![104, 16, 0, 1])]);
        assert_eq!(
            parse_response(&packet, 7),
            Ok(vec![Record::A(Ipv4Addr::new(104, 16, 0, 1))])
        );
    }

    #[test]
    fn reads_ns_records_through_compression() {
        // ns1. + a pointer to the question's `a.io`
        let data = vec![3, b'n', b's', b'1', 0xc0, 12];
        assert_eq!(
            parse_response(&response(0, &[(TYPE_NS, data)]), 7),
            Ok(vec![Record::Ns("ns1.a.io".into())])
        );
    }

    #[test]
    fn a_name_not_published_yet_has_no_records() {
        assert_eq!(
            parse_response(&response(RCODE_NX_DOMAIN, &[]), 7),
            Ok(vec![])
        );
        assert_eq!(parse_response(&response(0, &[]), 7), Ok(vec![]));
    }

    #[test]
    fn other_answers_are_ignored_and_errors_reported() {
        assert_eq!(parse_response(&response(0, &[(5, vec![0])]), 7), Ok(vec![]));
        assert!(parse_response(&response(2, &[]), 7).is_err());
        assert!(parse_response(&response(0, &[]), 8).is_err());
        assert!(parse_response(&[0, 7], 7).is_err());
    }

    #[test]
    fn pointer_loops_end() {
        let mut packet = vec![0; 12];
        packet.extend([0xc0, 12]);
        assert_eq!(read_name(&packet, 12), None);
    }
}
