use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkHealthReport {
    pub score: String,
    pub score_number: u32,
    pub reasons: Vec<String>,
    pub latency_grade: String,
    pub jitter_grade: String,
    pub stability_grade: String,
}

pub fn evaluate_network_health(latency_ms: f64, jitter_ms: f64, packet_loss_percent: f64) -> NetworkHealthReport {
    let mut score = 100i32;
    let mut reasons = Vec::new();

    // 1. Latency impact
    let latency_grade = if latency_ms <= 20.0 {
        reasons.push(format!("Ultra-low round-trip latency ({:.0} ms)", latency_ms));
        "A+".to_string()
    } else if latency_ms <= 40.0 {
        score -= 5;
        reasons.push(format!("Optimal round-trip latency ({:.0} ms)", latency_ms));
        "A".to_string()
    } else if latency_ms <= 80.0 {
        score -= 15;
        reasons.push(format!("Moderate latency ({:.0} ms) - good for streaming", latency_ms));
        "B".to_string()
    } else if latency_ms <= 150.0 {
        score -= 30;
        reasons.push(format!("Elevated network latency ({:.0} ms)", latency_ms));
        "C".to_string()
    } else {
        score -= 50;
        reasons.push(format!("Severe queue latency / bufferbloat ({:.0} ms)", latency_ms));
        "D".to_string()
    };

    // 2. Jitter impact (RFC 3550 variance)
    let jitter_grade = if jitter_ms <= 2.5 {
        reasons.push(format!("Rock-solid packet cadence (jitter {:.1} ms)", jitter_ms));
        "A+".to_string()
    } else if jitter_ms <= 6.0 {
        score -= 5;
        reasons.push(format!("Consistent packet arrival (jitter {:.1} ms)", jitter_ms));
        "A".to_string()
    } else if jitter_ms <= 12.0 {
        score -= 15;
        reasons.push(format!("Minor packet interval fluctuation (jitter {:.1} ms)", jitter_ms));
        "B".to_string()
    } else {
        score -= 30;
        reasons.push(format!("Noticeable packet jitter ({:.1} ms) may risk buffer starvation", jitter_ms));
        "C".to_string()
    };

    // 3. Packet loss impact
    let stability_grade = if packet_loss_percent <= 0.01 {
        reasons.push("Zero packet loss (100% transmission integrity)".to_string());
        "100%".to_string()
    } else if packet_loss_percent <= 1.0 {
        score -= 10;
        reasons.push(format!("Minimal retransmissions ({:.1}% packet loss)", packet_loss_percent));
        "99%".to_string()
    } else if packet_loss_percent <= 3.0 {
        score -= 25;
        reasons.push(format!("Packet drop detected ({:.1}% loss) - causing TCP backoff", packet_loss_percent));
        "95%".to_string()
    } else {
        score -= 50;
        reasons.push(format!("High packet loss ({:.1}%) degrades streaming quality", packet_loss_percent));
        "Degraded".to_string()
    };

    let final_score = score.clamp(0, 100) as u32;

    let score_label = if final_score >= 90 {
        "excellent"
    } else if final_score >= 75 {
        "good"
    } else if final_score >= 55 {
        "moderate"
    } else {
        "poor"
    };

    NetworkHealthReport {
        score: score_label.to_string(),
        score_number: final_score,
        reasons,
        latency_grade,
        jitter_grade,
        stability_grade,
    }
}

