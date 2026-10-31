use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsBenchmarkResult {
    pub name: String,
    #[serde(rename = "primaryIp")]
    pub primary_ip: String,
    #[serde(rename = "secondaryIp")]
    pub secondary_ip: String,
    pub description: String,
    #[serde(rename = "medianLatencyMs")]
    pub median_latency_ms: f64,
    #[serde(rename = "minLatencyMs")]
    pub min_latency_ms: f64,
    #[serde(rename = "maxLatencyMs")]
    pub max_latency_ms: f64,
    #[serde(rename = "successRatePercent")]
    pub success_rate_percent: f64,
    #[serde(rename = "reliabilityPercent")]
    pub reliability_percent: f64,
    #[serde(rename = "isCurrent")]
    pub is_current: bool,
    #[serde(rename = "isRecommended")]
    pub is_recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsBenchmarkReport {
    pub providers: Vec<DnsBenchmarkResult>,
    #[serde(rename = "recommendedProvider")]
    pub recommended_provider: Option<String>,
    #[serde(rename = "currentDns")]
    pub current_dns: Option<String>,
    #[serde(rename = "activeInterface")]
    pub active_interface: Option<String>,
    pub disclaimer: String,
}

pub struct DnsBenchmarkEngine;

impl DnsBenchmarkEngine {
    /// Build standard DNS Type A query packet (RFC 1035 wire format)
    pub fn build_dns_query(domain: &str, tx_id: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(64);
        // 12-byte DNS Header
        packet.extend_from_slice(&tx_id.to_be_bytes()); // Transaction ID
        packet.extend_from_slice(&[0x01, 0x00]); // Standard query, recursion desired
        packet.extend_from_slice(&[0x00, 0x01]); // QDCOUNT: 1 question
        packet.extend_from_slice(&[0x00, 0x00]); // ANCOUNT: 0
        packet.extend_from_slice(&[0x00, 0x00]); // NSCOUNT: 0
        packet.extend_from_slice(&[0x00, 0x00]); // ARCOUNT: 0

        // Question: QNAME
        for label in domain.split('.') {
            if !label.is_empty() {
                packet.push(label.len() as u8);
                packet.extend_from_slice(label.as_bytes());
            }
        }
        packet.push(0x00); // Root null terminator

        // QTYPE: 0x0001 (Type A, IPv4 address)
        packet.extend_from_slice(&[0x00, 0x01]);
        // QCLASS: 0x0001 (Class IN, Internet)
        packet.extend_from_slice(&[0x00, 0x01]);

        packet
    }

    /// Query resolver with repeated domains to measure real lookup latency & reliability
    pub fn probe_resolver(ip: &str, test_domains: &[&str]) -> (Vec<f64>, usize, usize) {
        let mut latencies = Vec::new();
        let mut successes = 0;
        let mut attempts = 0;

        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(_) => return (latencies, 0, test_domains.len()),
        };
        let _ = socket.set_read_timeout(Some(Duration::from_millis(1500)));

        let target_addr: SocketAddr = match format!("{}:53", ip).parse() {
            Ok(a) => a,
            Err(_) => return (latencies, 0, test_domains.len()),
        };

        let mut tx_id = 0x2000u16;

        for &domain in test_domains {
            attempts += 1;
            tx_id = tx_id.wrapping_add(1);
            let query = Self::build_dns_query(domain, tx_id);

            let start = Instant::now();
            if socket.send_to(&query, target_addr).is_ok() {
                let mut buf = [0u8; 512];
                if let Ok((len, _)) = socket.recv_from(&mut buf) {
                    let rtt_ms = start.elapsed().as_secs_f64() * 1000.0;
                    if len >= 12 && u16::from_be_bytes([buf[0], buf[1]]) == tx_id {
                        let rcode = buf[3] & 0x0F;
                        // NOERROR (0) or NXDOMAIN (3) indicates authoritative response
                        if rcode == 0 || rcode == 3 {
                            latencies.push(rtt_ms);
                            successes += 1;
                        }
                    }
                }
            }
        }

        (latencies, successes, attempts)
    }

    /// Benchmark major resolvers and current system resolver
    pub fn benchmark_all(current_dns_primary: Option<&str>, active_iface: Option<&str>) -> DnsBenchmarkReport {
        let test_domains = [
            "google.com",
            "cloudflare.com",
            "microsoft.com",
            "wikipedia.org",
            "amazon.com",
        ];

        let provider_specs = vec![
            (
                "Cloudflare DNS",
                "1.1.1.1",
                "1.0.0.1",
                "Fastest global anycast resolver with strict privacy and zero user tracking.",
            ),
            (
                "Google Public DNS",
                "8.8.8.8",
                "8.8.4.4",
                "Broad geo-distributed anycast infrastructure with global cache peering.",
            ),
            (
                "Quad9 DNS",
                "9.9.9.9",
                "149.112.112.112",
                "Security-hardened threat-blocking resolver operated by a non-profit foundation.",
            ),
            (
                "OpenDNS (Cisco)",
                "208.67.222.222",
                "208.67.220.220",
                "Reliable enterprise-backed public recursive DNS with PhishTank threat blocking.",
            ),
        ];

        let mut results = Vec::new();

        for (name, p_ip, s_ip, desc) in provider_specs {
            let (mut lats, successes, attempts) = Self::probe_resolver(p_ip, &test_domains);

            let (median, min, max, success_pct) = if !lats.is_empty() {
                lats.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = lats.len() / 2;
                let med = if lats.len() % 2 == 0 && mid > 0 {
                    (lats[mid - 1] + lats[mid]) / 2.0
                } else {
                    lats[mid]
                };
                let min_l = lats[0];
                let max_l = lats[lats.len() - 1];
                let s_pct = (successes as f64 / attempts as f64) * 100.0;
                (med, min_l, max_l, s_pct)
            } else {
                (999.0, 999.0, 999.0, 0.0)
            };

            let is_curr = current_dns_primary.map(|c| c == p_ip).unwrap_or(false);

            results.push(DnsBenchmarkResult {
                name: name.to_string(),
                primary_ip: p_ip.to_string(),
                secondary_ip: s_ip.to_string(),
                description: desc.to_string(),
                median_latency_ms: median,
                min_latency_ms: min,
                max_latency_ms: max,
                success_rate_percent: success_pct,
                reliability_percent: success_pct,
                is_current: is_curr,
                is_recommended: false, // will mark after sorting
            });
        }

        // Add current system resolver if available and distinct from public ones
        if let Some(curr) = current_dns_primary {
            let is_already_tested = results.iter().any(|r| r.primary_ip == curr);
            if !is_already_tested && !curr.is_empty() {
                let (mut lats, successes, attempts) = Self::probe_resolver(curr, &test_domains);
                let (median, min, max, success_pct) = if !lats.is_empty() {
                    lats.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let mid = lats.len() / 2;
                    let med = if lats.len() % 2 == 0 && mid > 0 {
                        (lats[mid - 1] + lats[mid]) / 2.0
                    } else {
                        lats[mid]
                    };
                    (med, lats[0], lats[lats.len() - 1], (successes as f64 / attempts as f64) * 100.0)
                } else {
                    (999.0, 999.0, 999.0, 0.0)
                };

                results.push(DnsBenchmarkResult {
                    name: "Current System Resolver".to_string(),
                    primary_ip: curr.to_string(),
                    secondary_ip: "—".to_string(),
                    description: "Active local network gateway / ISP configured DNS resolver.".to_string(),
                    median_latency_ms: median,
                    min_latency_ms: min,
                    max_latency_ms: max,
                    success_rate_percent: success_pct,
                    reliability_percent: success_pct,
                    is_current: true,
                    is_recommended: false,
                });
            }
        }

        // Recommend provider with lowest median latency that achieved >= 90% success
        let mut valid_providers: Vec<&mut DnsBenchmarkResult> = results
            .iter_mut()
            .filter(|r| r.success_rate_percent >= 80.0 && r.median_latency_ms < 500.0)
            .collect();

        let mut recommended_name = None;
        if !valid_providers.is_empty() {
            valid_providers.sort_by(|a, b| {
                a.median_latency_ms
                    .partial_cmp(&b.median_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            valid_providers[0].is_recommended = true;
            recommended_name = Some(valid_providers[0].name.clone());
        }

        DnsBenchmarkReport {
            providers: results,
            recommended_provider: recommended_name,
            current_dns: current_dns_primary.map(|s| s.to_string()),
            active_interface: active_iface.map(|s| s.to_string()),
            disclaimer: "DNS optimization reduces domain lookup latency (Time To First Byte). It does NOT increase maximum ISP download/upload bandwidth.".to_string(),
        }
    }
}

