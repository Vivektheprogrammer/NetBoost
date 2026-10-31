use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SegmentStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSegment {
    pub index: usize,
    #[serde(rename = "startByte")]
    pub start_byte: u64,
    #[serde(rename = "endByte")]
    pub end_byte: u64,
    #[serde(rename = "downloadedBytes")]
    pub downloaded_bytes: u64,
    #[serde(rename = "speedBps")]
    pub speed_bps: f64,
    pub status: SegmentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Probing,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub filename: String,
    #[serde(rename = "destinationPath")]
    pub destination_path: String,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "downloadedBytes")]
    pub downloaded_bytes: u64,
    #[serde(rename = "speedBps")]
    pub speed_bps: f64,
    #[serde(rename = "etaSeconds")]
    pub eta_seconds: u64,
    pub status: DownloadStatus,
    #[serde(rename = "supportsRange")]
    pub supports_range: bool,
    pub etag: Option<String>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    #[serde(rename = "checksumSha256")]
    pub checksum_sha256: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    pub concurrency: usize,
    pub segments: Vec<DownloadSegment>,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<u64>,
}

