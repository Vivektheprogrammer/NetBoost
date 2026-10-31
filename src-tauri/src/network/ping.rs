use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

pub struct LatencyDiagnostics {
    pub last_latency_ms: f64,
    pub jitter_ms: f64,
    pub packet_loss_percent: f64,
    samples: Vec<f64>,
    loss_window: Vec<bool>, // true = success, false = dropped/timeout
}

impl LatencyDiagnostics {
    pub fn new() -> Self {
        Self {
            last_latency_ms: 15.0,
            jitter_ms: 1.5,
            packet_loss_percent: 0.0,
            samples: Vec::with_capacity(30),
            loss_window: Vec::with_capacity(20),
        }
    }

    /// Measures actual round-trip latency to a reliable anycast resolver (1.1.1.1:53 or 8.8.8.8:53)
    pub fn sample_latency(&mut self) -> (f64, f64, f64) {
        let targets = ["1.1.1.1:53", "8.8.8.8:53", "9.9.9.9:53"];
        let mut measured_ms: Option<f64> = None;

        for target in &targets {
            if let Ok(addr) = target.to_socket_addrs() {
                if let Some(sock_addr) = addr.into_iter().next() {
                    let start = Instant::now();
                    if let Ok(_stream) = TcpStream::connect_timeout(&sock_addr, Duration::from_millis(1500)) {
                        let duration = start.elapsed();
                        measured_ms = Some(duration.as_secs_f64() * 1000.0);
                        break;
                    }
                }
            }
        }

        match measured_ms {
            Some(ms) => {
                // Record success for packet loss tracking
                self.record_probe(true);

                // Update TCP Connect RTT Variation (smoothed RTT variance)
                if let Some(&prev_ms) = self.samples.last() {
                    let d = (ms - prev_ms).abs();
                    // Jitter calculation: J(i) = J(i-1) + (|D(i-1, i)| - J(i-1)) / 16.0
                    self.jitter_ms = self.jitter_ms + (d - self.jitter_ms) / 16.0;
                }

                self.last_latency_ms = ms;
                self.samples.push(ms);
                if self.samples.len() > 30 {
                    self.samples.remove(0);
                }
            }
            None => {
                // Packet dropped / timed out
                self.record_probe(false);
            }
        }

        // Compute loss percentage
        let total = self.loss_window.len();
        if total > 0 {
            let lost = self.loss_window.iter().filter(|&&ok| !ok).count();
            self.packet_loss_percent = (lost as f64 / total as f64) * 100.0;
        }

        (self.last_latency_ms, self.jitter_ms, self.packet_loss_percent)
    }

    fn record_probe(&mut self, success: bool) {
        self.loss_window.push(success);
        if self.loss_window.len() > 20 {
            self.loss_window.remove(0);
        }
    }
}

