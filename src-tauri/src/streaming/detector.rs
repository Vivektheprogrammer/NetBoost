use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNetworkInfo {
    pub pid: u32,
    pub name: String,
    pub path: String,
    #[serde(rename = "downloadSpeedBps")]
    pub download_speed_bps: f64,
    #[serde(rename = "uploadSpeedBps")]
    pub upload_speed_bps: f64,
    #[serde(rename = "totalBytesReceived")]
    pub total_bytes_received: u64,
    #[serde(rename = "totalBytesSent")]
    pub total_bytes_sent: u64,
    #[serde(rename = "activeConnections")]
    pub active_connections: u32,
    #[serde(rename = "isStreaming")]
    pub is_streaming: bool,
    #[serde(rename = "streamingConfidence")]
    pub streaming_confidence: String,
    pub measured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    #[serde(rename = "defaultApps")]
    pub default_apps: Vec<String>,
    #[serde(rename = "userConfiguredApps")]
    pub user_configured_apps: Vec<String>,
}

pub struct StreamingDetector {
    default_streaming_binaries: HashSet<&'static str>,
    user_configured_apps: RwLock<HashSet<String>>,
    known_cdn_hints: Vec<&'static str>,
}

impl StreamingDetector {
    pub fn new() -> Self {
        let mut defaults = HashSet::new();
        // Dedicated streaming applications & media players
        defaults.insert("netflix.exe");
        defaults.insert("spotify.exe");
        defaults.insert("primevideo.exe");
        defaults.insert("amazonprimevideo.exe");
        defaults.insert("disneyplus.exe");
        defaults.insert("hotstar.exe");
        defaults.insert("vlc.exe");
        defaults.insert("stremio.exe");
        defaults.insert("kodi.exe");
        defaults.insert("plex.exe");
        defaults.insert("youtube.exe");
        defaults.insert("discord.exe");
        defaults.insert("zoom.exe");
        defaults.insert("teams.exe");
        defaults.insert("obs64.exe");
        defaults.insert("twitch.exe");
        // Browsers capable of streaming video (analyzed with cadence/rate heuristics)
        defaults.insert("chrome.exe");
        defaults.insert("msedge.exe");
        defaults.insert("firefox.exe");
        defaults.insert("brave.exe");
        defaults.insert("opera.exe");
        defaults.insert("vivaldi.exe");

        let cdn_hints = vec![
            "googlevideo",
            "youtube",
            "nflxvideo",
            "nflximg",
            "netflix",
            "aiv-cdn",
            "primevideo",
            "hotstar",
            "disney",
            "akamai",
            "fastly",
            "cloudfront",
            "spotify",
            "audio-ak",
        ];

        Self {
            default_streaming_binaries: defaults,
            user_configured_apps: RwLock::new(HashSet::new()),
            known_cdn_hints: cdn_hints,
        }
    }

    pub fn load_user_apps(&self, apps: Vec<String>) {
        let mut guard = self.user_configured_apps.write().unwrap();
        guard.clear();
        for app in apps {
            guard.insert(app.to_lowercase());
        }
    }

    pub fn add_user_app(&self, app_name: &str) {
        let mut guard = self.user_configured_apps.write().unwrap();
        guard.insert(app_name.to_lowercase());
    }

    pub fn remove_user_app(&self, app_name: &str) {
        let mut guard = self.user_configured_apps.write().unwrap();
        guard.remove(&app_name.to_lowercase());
    }

    pub fn get_config(&self) -> StreamingConfig {
        let guard = self.user_configured_apps.read().unwrap();
        let user_apps: Vec<String> = guard.iter().cloned().collect();
        let mut defaults: Vec<String> = self.default_streaming_binaries.iter().map(|&s| s.to_string()).collect();
        defaults.sort();

        StreamingConfig {
            default_apps: defaults,
            user_configured_apps: user_apps,
        }
    }

    /// Evaluates streaming activity based on process metadata, connection ports/states,
    /// traffic volume patterns, and user configuration.
    pub fn classify_process(
        &self,
        name: &str,
        active_connections: u32,
        remote_addrs: &[String],
        read_rate_bps: f64,
    ) -> (bool, String, Option<String>) {
        let name_lower = name.to_lowercase();

        // 1. Check user-configured override
        let is_user_app = {
            let guard = self.user_configured_apps.read().unwrap();
            guard.contains(&name_lower)
        };

        if is_user_app && active_connections > 0 {
            return (
                true,
                "high".to_string(),
                Some(format!("User-configured streaming application with {} active sockets.", active_connections)),
            );
        }

        // 2. Check dedicated media player applications
        let is_dedicated_stream = name_lower.contains("netflix")
            || name_lower.contains("spotify")
            || name_lower.contains("primevideo")
            || name_lower.contains("hotstar")
            || name_lower.contains("disney")
            || name_lower.contains("vlc")
            || name_lower.contains("stremio")
            || name_lower.contains("kodi")
            || name_lower.contains("plex");

        if is_dedicated_stream && active_connections > 0 {
            return (
                true,
                "high".to_string(),
                Some(format!("Dedicated streaming media process with {} active sockets.", active_connections)),
            );
        }

        // 3. Check Web Browsers (Chrome, Edge, Firefox, Brave, Opera)
        let is_browser = name_lower.contains("chrome")
            || name_lower.contains("edge")
            || name_lower.contains("firefox")
            || name_lower.contains("brave")
            || name_lower.contains("opera")
            || name_lower.contains("vivaldi");

        if is_browser && active_connections > 0 {
            let https_count = remote_addrs
                .iter()
                .filter(|addr| addr.ends_with(":443") || addr.ends_with(":8443"))
                .count();

            // Active video / media streaming: active HTTPS traffic or multiple media sockets
            if https_count >= 2 || read_rate_bps >= 300_000.0 {
                return (
                    true,
                    "high".to_string(),
                    Some(format!(
                        "Active web streaming/media playback detected ({} secure connections, {:.1} Mbps throughput).",
                        https_count,
                        read_rate_bps / 1_000_000.0
                    )),
                );
            }

            if https_count >= 1 {
                return (
                    true,
                    "medium".to_string(),
                    Some("Web browser maintaining active secure connection for media/playback.".to_string()),
                );
            }

            return (
                false,
                "low".to_string(),
                Some("Web browser with background socket connections.".to_string()),
            );
        }

        // 4. Other processes with active network sockets
        if active_connections > 0 {
            (false, "unknown".to_string(), None)
        } else {
            (false, "none".to_string(), None)
        }
    }
}
