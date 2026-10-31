use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingMode {
    Streaming,
    Balanced,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationState {
    pub enabled: bool,
    pub mode: OperatingMode,
    #[serde(rename = "congestionDetected")]
    pub congestion_detected: bool,
    #[serde(rename = "congestionReason")]
    pub congestion_reason: Option<String>,
    #[serde(rename = "activeStreamingSessions")]
    pub active_streaming_sessions: u32,
    #[serde(rename = "activeDownloads")]
    pub active_downloads: u32,
    #[serde(rename = "policyDescription")]
    pub policy_description: String,
    #[serde(rename = "baselineLatencyMs")]
    pub baseline_latency_ms: f64,
    #[serde(rename = "medianLatencyMs")]
    pub median_latency_ms: f64,
    #[serde(rename = "pacingDelayMs")]
    pub pacing_delay_ms: u64,
    #[serde(rename = "effectiveConcurrencyFactor")]
    pub effective_concurrency_factor: f64,
}

#[derive(Debug, Clone)]
pub struct LatencySample {
    pub timestamp: Instant,
    pub rtt_ms: f64,
    pub jitter_ms: f64,
    pub packet_loss_pct: f64,
}

pub struct BandwidthManager {
    pub mode: OperatingMode,
    pub enabled: bool,
    pub baseline_latency_ms: f64,

    // Sliding window of measurements for median calculation (rejects single-spike outliers)
    recent_samples: VecDeque<LatencySample>,
    window_capacity: usize,

    // Hysteresis counters to prevent rapid switching/oscillations
    congestion_streak: u32,
    recovery_streak: u32,
    pub is_congested: bool,
    last_adaptation_time: Instant,
    cooldown_duration: Duration,

    // Adaptive pacing and concurrency controls
    current_pacing_delay_ms: u64,
    concurrency_throttle_factor: f64,
    pub last_congestion_reason: Option<String>,
}

impl BandwidthManager {
    pub fn new() -> Self {
        Self {
            mode: OperatingMode::Streaming,
            enabled: true,
            baseline_latency_ms: 20.0,
            recent_samples: VecDeque::with_capacity(20),
            window_capacity: 15,
            congestion_streak: 0,
            recovery_streak: 0,
            is_congested: false,
            last_adaptation_time: Instant::now() - Duration::from_secs(10),
            cooldown_duration: Duration::from_millis(3500),
            current_pacing_delay_ms: 0,
            concurrency_throttle_factor: 1.0,
            last_congestion_reason: None,
        }
    }

    pub fn set_mode(&mut self, mode: OperatingMode) {
        self.mode = mode;
        self.apply_mode_policy();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_pacing_delay_ms = 0;
            self.concurrency_throttle_factor = 1.0;
            self.is_congested = false;
        } else {
            self.apply_mode_policy();
        }
    }

    /// Record a probe sample into the sliding window
    pub fn record_probe(&mut self, rtt_ms: f64, jitter_ms: f64, packet_loss_pct: f64) {
        if self.recent_samples.len() >= self.window_capacity {
            self.recent_samples.pop_front();
        }
        self.recent_samples.push_back(LatencySample {
            timestamp: Instant::now(),
            rtt_ms,
            jitter_ms,
            packet_loss_pct,
        });
    }

    /// Calculate median latency from recent window to filter out transient spikes
    pub fn calculate_median_latency(&self) -> f64 {
        if self.recent_samples.is_empty() {
            return self.baseline_latency_ms;
        }
        let mut vals: Vec<f64> = self.recent_samples.iter().map(|s| s.rtt_ms).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = vals.len() / 2;
        if vals.len() % 2 == 0 && mid > 0 {
            (vals[mid - 1] + vals[mid]) / 2.0
        } else {
            vals[mid]
        }
    }

    /// Calculate median jitter from recent window
    pub fn calculate_median_jitter(&self) -> f64 {
        if self.recent_samples.is_empty() {
            return 2.0;
        }
        let mut vals: Vec<f64> = self.recent_samples.iter().map(|s| s.jitter_ms).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vals[vals.len() / 2]
    }

    /// Calculate median packet loss percentage
    pub fn calculate_median_loss(&self) -> f64 {
        if self.recent_samples.is_empty() {
            return 0.0;
        }
        let mut vals: Vec<f64> = self.recent_samples.iter().map(|s| s.packet_loss_pct).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        vals[vals.len() / 2]
    }

    /// Check congestion with hysteresis and evaluate whether NetBoost downloads are responsible.
    ///
    /// Requires >= 3 consecutive cycles of elevated median latency to declare congestion.
    /// Requires >= 5 consecutive cycles of normal median latency to declare recovery.
    pub fn evaluate_congestion_with_traffic(
        &mut self,
        current_latency_ms: f64,
        current_jitter_ms: f64,
        active_downloads: u32,
        netboost_speed_bps: f64,
        active_streaming_sessions: u32,
    ) -> (bool, Option<String>) {
        if !self.enabled {
            self.current_pacing_delay_ms = 0;
            self.concurrency_throttle_factor = 1.0;
            return (false, None);
        }

        // Record incoming sample
        self.record_probe(current_latency_ms, current_jitter_ms, 0.0);

        let median_lat = self.calculate_median_latency();
        let median_jit = self.calculate_median_jitter();
        let median_loss = self.calculate_median_loss();

        // Bufferbloat criteria:
        // 1. Current probe exceeds 1.75x baseline AND absolute increase > 25ms, OR
        // 2. High jitter (> 18ms), OR
        // 3. Significant packet loss (> 5.0%)
        let is_elevated = (current_latency_ms > self.baseline_latency_ms * 1.75 && (current_latency_ms - self.baseline_latency_ms) > 25.0)
            || current_jitter_ms > 18.0
            || median_loss > 5.0;

        let is_calm = current_latency_ms <= (self.baseline_latency_ms * 1.35)
            && current_jitter_ms <= 8.0;

        // Apply hysteresis streaks
        if is_elevated {
            self.congestion_streak += 1;
            self.recovery_streak = 0;
        } else if is_calm {
            self.recovery_streak += 1;
            self.congestion_streak = 0;
        } else {
            // Neutral band - maintain current state
            self.congestion_streak = 0;
            self.recovery_streak = 0;
        }

        let now = Instant::now();
        let cooldown_passed = now.duration_since(self.last_adaptation_time) >= self.cooldown_duration;

        // Transition to Congested: requires >= 3 consecutive elevated cycles
        if self.congestion_streak >= 3 && !self.is_congested {
            self.is_congested = true;
            self.last_adaptation_time = now;
            self.apply_mode_policy_on_congestion(active_downloads, netboost_speed_bps, active_streaming_sessions);
        }
        // Transition to Recovered: requires >= 5 consecutive calm cycles
        else if self.recovery_streak >= 5 && self.is_congested {
            self.is_congested = false;
            self.last_adaptation_time = now;
            self.apply_mode_policy_on_recovery(active_streaming_sessions);

            // Update baseline latency gently when idle and recovered
            if active_downloads == 0 && active_streaming_sessions == 0 && median_lat > 5.0 && median_lat < 120.0 {
                self.baseline_latency_ms = self.baseline_latency_ms * 0.85 + median_lat * 0.15;
            }
        }
        // Re-tune pacing if cooldown passed while in sustained congested state
        else if self.is_congested && cooldown_passed {
            self.last_adaptation_time = now;
            self.apply_mode_policy_on_congestion(active_downloads, netboost_speed_bps, active_streaming_sessions);
        }

        // Construct explanatory reason
        if self.is_congested {
            let reason = if active_downloads > 0 && netboost_speed_bps > 150_000.0 {
                format!(
                    "Bufferbloat under download load: Median latency {:.1} ms (baseline: {:.1} ms, +{:.1} ms). NetBoost is pacing chunks ({}ms) to protect latency.",
                    median_lat,
                    self.baseline_latency_ms,
                    median_lat - self.baseline_latency_ms,
                    self.current_pacing_delay_ms
                )
            } else if active_streaming_sessions > 0 {
                format!(
                    "Network queueing detected during active streaming session (latency {:.1} ms, jitter {:.1} ms). NetBoost is prioritizing playback queues.",
                    median_lat, median_jit
                )
            } else {
                format!(
                    "External network congestion detected ({:.1} ms vs baseline {:.1} ms, NetBoost downloads idle). Monitoring for recovery.",
                    median_lat, self.baseline_latency_ms
                )
            };
            self.last_congestion_reason = Some(reason.clone());
            (true, Some(reason))
        } else {
            self.last_congestion_reason = None;
            (false, None)
        }
    }

    /// Internal adaptation when congestion triggers
    fn apply_mode_policy_on_congestion(
        &mut self,
        _active_downloads: u32,
        _netboost_speed_bps: f64,
        active_streaming_sessions: u32,
    ) {
        match self.mode {
            OperatingMode::Streaming => {
                // Congestion / bufferbloat in Streaming mode:
                // Strongly throttle download workers (45ms pacing delay, 40% concurrency) to instantly clear queues
                self.current_pacing_delay_ms = if active_streaming_sessions > 0 { 45 } else { 35 };
                self.concurrency_throttle_factor = 0.40;
            }
            OperatingMode::Balanced => {
                self.current_pacing_delay_ms = if active_streaming_sessions > 0 { 20 } else { 12 };
                self.concurrency_throttle_factor = 0.70;
            }
            OperatingMode::Download => {
                // Download mode: prioritize raw throughput; only pace slightly if severe packet loss
                let median_loss = self.calculate_median_loss();
                if median_loss > 10.0 {
                    self.current_pacing_delay_ms = 8;
                    self.concurrency_throttle_factor = 0.85;
                } else {
                    self.current_pacing_delay_ms = 0;
                    self.concurrency_throttle_factor = 1.0;
                }
            }
        }
    }

    /// Internal adaptation when network recovers or operates normally
    fn apply_mode_policy_on_recovery(&mut self, active_streaming_sessions: u32) {
        match self.mode {
            OperatingMode::Streaming => {
                // When in Streaming mode, actively protect playback bandwidth!
                if active_streaming_sessions > 0 {
                    // Active video/audio stream detected: inject 30ms pacing delay and 50% concurrency
                    // to guarantee the stream gets primary bandwidth without buffering!
                    self.current_pacing_delay_ms = 30;
                    self.concurrency_throttle_factor = 0.50;
                } else {
                    // Preemptive headroom: moderate 15ms pacing to maintain flat ping for incoming streams
                    self.current_pacing_delay_ms = 15;
                    self.concurrency_throttle_factor = 0.65;
                }
            }
            OperatingMode::Balanced => {
                if active_streaming_sessions > 0 {
                    self.current_pacing_delay_ms = 10;
                    self.concurrency_throttle_factor = 0.80;
                } else {
                    self.current_pacing_delay_ms = 0;
                    self.concurrency_throttle_factor = 1.0;
                }
            }
            OperatingMode::Download => {
                self.current_pacing_delay_ms = 0;
                self.concurrency_throttle_factor = 1.0;
            }
        }
    }

    fn apply_mode_policy(&mut self) {
        if !self.enabled {
            self.current_pacing_delay_ms = 0;
            self.concurrency_throttle_factor = 1.0;
            return;
        }

        match self.mode {
            OperatingMode::Streaming => {
                // Immediate enforcement upon selecting Streaming Mode
                self.current_pacing_delay_ms = if self.is_congested { 45 } else { 20 };
                self.concurrency_throttle_factor = if self.is_congested { 0.40 } else { 0.60 };
            }
            OperatingMode::Balanced => {
                self.current_pacing_delay_ms = if self.is_congested { 15 } else { 0 };
                self.concurrency_throttle_factor = if self.is_congested { 0.70 } else { 1.0 };
            }
            OperatingMode::Download => {
                self.current_pacing_delay_ms = 0;
                self.concurrency_throttle_factor = 1.0;
            }
        }
    }

    /// Direct threshold congestion check for instantaneous evaluation and backward compatibility
    pub fn check_congestion(&self, current_latency_ms: f64, current_jitter_ms: f64) -> (bool, Option<String>) {
        if !self.enabled {
            return (false, None);
        }

        if current_latency_ms > self.baseline_latency_ms * 2.5 && current_latency_ms > 45.0 {
            (
                true,
                Some(format!(
                    "Bufferbloat detected: Latency spiked to {:.0} ms (baseline: {:.0} ms). NetBoost is prioritizing playback queues.",
                    current_latency_ms, self.baseline_latency_ms
                )),
            )
        } else if current_jitter_ms > 12.0 {
            (
                true,
                Some(format!(
                    "High packet variance (jitter {:.1} ms) detected. Smoothing connection concurrency.",
                    current_jitter_ms
                )),
            )
        } else {
            (false, None)
        }
    }

    /// Get current read pacing delay in milliseconds
    pub fn get_pacing_delay_ms(&self) -> u64 {
        if !self.enabled {
            0
        } else {
            self.current_pacing_delay_ms
        }
    }

    /// Get dynamically throttled concurrency limit for a task
    pub fn get_effective_concurrency(&self, requested: usize) -> usize {
        if !self.enabled {
            requested
        } else {
            let throttled = (requested as f64 * self.concurrency_throttle_factor).round() as usize;
            throttled.max(1)
        }
    }

    pub fn get_state(
        &self,
        current_latency_ms: f64,
        _current_jitter_ms: f64,
        streaming_sessions: u32,
        active_downloads: u32,
    ) -> OptimizationState {
        let median_lat = self.calculate_median_latency();

        let reason = if self.is_congested {
            self.last_congestion_reason.clone().or_else(|| {
                Some(format!(
                    "Active bufferbloat/congestion ({:.1} ms vs baseline {:.1} ms). NetBoost is enforcing {}ms chunk pacing.",
                    median_lat, self.baseline_latency_ms, self.current_pacing_delay_ms
                ))
            })
        } else {
            None
        };

        let policy_description = match self.mode {
            OperatingMode::Streaming => {
                "Streaming Priority: Protecting video buffer integrity with dynamic pacing and queue mitigation.".to_string()
            }
            OperatingMode::Balanced => {
                "Balanced Mode: Fairly distributing local connection capacity among all applications.".to_string()
            }
            OperatingMode::Download => {
                "Download Priority: Maximizing segmented HTTP download throughput.".to_string()
            }
        };

        OptimizationState {
            enabled: self.enabled,
            mode: self.mode.clone(),
            congestion_detected: self.is_congested,
            congestion_reason: reason,
            active_streaming_sessions: streaming_sessions,
            active_downloads,
            policy_description,
            baseline_latency_ms: self.baseline_latency_ms,
            median_latency_ms: if self.recent_samples.is_empty() { current_latency_ms } else { median_lat },
            pacing_delay_ms: self.current_pacing_delay_ms,
            effective_concurrency_factor: self.concurrency_throttle_factor,
        }
    }
}
