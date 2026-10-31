use crate::bandwidth::manager::BandwidthManager;
use crate::downloads::types::{DownloadSegment, DownloadStatus, DownloadTask, SegmentStatus};
use crate::storage::db::{Database, DbDownloadRecord};
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, ETAG, LAST_MODIFIED, RANGE, RETRY_AFTER};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct ProbeInfo {
    pub total_bytes: u64,
    pub supports_range: bool,
    pub filename: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

pub struct ActiveDownloadControl {
    pub cancel_sender: broadcast::Sender<()>,
    pub is_paused: Arc<AtomicBool>,
}

pub struct DownloadEngine {
    client: reqwest::Client,
    pub tasks: Arc<Mutex<HashMap<String, DownloadTask>>>,
    active_controls: Mutex<HashMap<String, ActiveDownloadControl>>,
    pub db: Arc<Database>,
    pub default_download_dir: PathBuf,
}

impl DownloadEngine {
    pub fn new(db: Arc<Database>, download_dir: Option<PathBuf>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let dir = download_dir.unwrap_or_else(|| {
            // Default to user's standard Downloads folder or working directory
            if let Ok(user_profile) = std::env::var("USERPROFILE") {
                PathBuf::from(user_profile).join("Downloads")
            } else {
                PathBuf::from("downloads")
            }
        });

        // Ensure directory exists
        let _ = std::fs::create_dir_all(&dir);

        // Load existing tasks from SQLite database
        let mut initial_tasks = HashMap::new();
        if let Ok(records) = db.get_all_downloads() {
            for r in records {
                let status = match r.status.as_str() {
                    "completed" => DownloadStatus::Completed,
                    "paused" => DownloadStatus::Paused,
                    "failed" => DownloadStatus::Failed,
                    "cancelled" => DownloadStatus::Cancelled,
                    _ => DownloadStatus::Paused, // reset active to paused on reload
                };

                let task = DownloadTask {
                    id: r.id.clone(),
                    url: r.url,
                    filename: r.filename,
                    destination_path: r.destination_path,
                    total_bytes: r.total_bytes,
                    downloaded_bytes: r.downloaded_bytes,
                    speed_bps: 0.0,
                    eta_seconds: 0,
                    status,
                    supports_range: true,
                    etag: r.etag,
                    last_modified: r.last_modified,
                    checksum_sha256: r.checksum_sha256,
                    error_message: r.error_message,
                    concurrency: r.concurrency as usize,
                    segments: Vec::new(),
                    created_at: r.created_at,
                    completed_at: r.completed_at,
                };
                initial_tasks.insert(r.id, task);
            }
        }

        Self {
            client,
            tasks: Arc::new(Mutex::new(initial_tasks)),
            active_controls: Mutex::new(HashMap::new()),
            db,
            default_download_dir: dir,
        }
    }

    /// Probe URL capabilities: Content-Length, Accept-Ranges, ETag, filename
    pub async fn probe_url(&self, url: &str) -> Result<ProbeInfo, String> {
        // Try range probe request: Range: bytes=0-0
        let mut attempt = 0;
        let mut retry_after_delay = Duration::from_secs(1);

        loop {
            attempt += 1;
            let resp_res = self
                .client
                .get(url)
                .header(RANGE, "bytes=0-0")
                .send()
                .await;

            match resp_res {
                Ok(resp) => {
                    let status = resp.status();

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        if attempt >= 3 {
                            return Err("Server returned HTTP 429 Too Many Requests (Rate limit exceeded)".to_string());
                        }

                        // Parse Retry-After header
                        if let Some(val) = resp.headers().get(RETRY_AFTER) {
                            if let Ok(s) = val.to_str() {
                                if let Ok(secs) = s.parse::<u64>() {
                                    retry_after_delay = Duration::from_secs(secs);
                                }
                            }
                        }

                        tokio::time::sleep(retry_after_delay).await;
                        continue;
                    }

                    if !status.is_success() && status != StatusCode::PARTIAL_CONTENT {
                        return Err(format!("Server returned HTTP status: {}", status));
                    }

                    let headers = resp.headers().clone();

                    let supports_range = status == StatusCode::PARTIAL_CONTENT
                        || headers
                            .get(ACCEPT_RANGES)
                            .and_then(|v| v.to_str().ok())
                            .map(|v| v.eq_ignore_ascii_case("bytes"))
                            .unwrap_or(false);

                    let total_bytes = if let Some(cr) = headers.get(CONTENT_RANGE).and_then(|v| v.to_str().ok()) {
                        // Format: bytes 0-0/104857600
                        cr.split('/')
                            .last()
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0)
                    } else if let Some(cl) = headers.get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()) {
                        cl.parse::<u64>().unwrap_or(0)
                    } else {
                        0
                    };

                    let etag = headers
                        .get(ETAG)
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.to_string());

                    let last_modified = headers
                        .get(LAST_MODIFIED)
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.to_string());

                    // Infer filename
                    let filename = derive_filename(url, &headers);

                    return Ok(ProbeInfo {
                        total_bytes,
                        supports_range,
                        filename,
                        etag,
                        last_modified,
                    });
                }
                Err(e) => {
                    if attempt >= 3 {
                        return Err(format!("Network connection error: {}", e));
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Add a new download task and start accelerating
    pub async fn start_download(
        &self,
        url: &str,
        custom_filename: Option<String>,
        concurrency: Option<usize>,
        bandwidth_mgr: Arc<Mutex<BandwidthManager>>,
    ) -> Result<String, String> {
        let probe = self.probe_url(url).await?;

        let fname = custom_filename.unwrap_or(probe.filename);
        let dest_path = self.default_download_dir.join(&fname);

        let id = format!("dl-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());

        // Determine number of segments (dynamic concurrency with bandwidth coordinator adaptation)
        let requested_concurrency = concurrency.unwrap_or(4).clamp(1, 16);
        let adapted_concurrency = {
            let mgr = bandwidth_mgr.lock().unwrap();
            mgr.get_effective_concurrency(requested_concurrency)
        };
        let active_concurrency = if probe.supports_range && probe.total_bytes >= 2 * 1024 * 1024 {
            adapted_concurrency
        } else {
            1 // Single worker if server does not support byte ranges or file is tiny
        };

        // Divide file into byte range segments
        let mut segments = Vec::new();
        if active_concurrency > 1 && probe.total_bytes > 0 {
            let chunk_size = probe.total_bytes / (active_concurrency as u64);
            for i in 0..active_concurrency {
                let start = (i as u64) * chunk_size;
                let end = if i == active_concurrency - 1 {
                    probe.total_bytes - 1
                } else {
                    ((i + 1) as u64) * chunk_size - 1
                };

                segments.push(DownloadSegment {
                    index: i,
                    start_byte: start,
                    end_byte: end,
                    downloaded_bytes: 0,
                    speed_bps: 0.0,
                    status: SegmentStatus::Pending,
                });
            }
        } else {
            segments.push(DownloadSegment {
                index: 0,
                start_byte: 0,
                end_byte: if probe.total_bytes > 0 { probe.total_bytes - 1 } else { 0 },
                downloaded_bytes: 0,
                speed_bps: 0.0,
                status: SegmentStatus::Pending,
            });
        }

        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

        let task = DownloadTask {
            id: id.clone(),
            url: url.to_string(),
            filename: fname,
            destination_path: dest_path.to_string_lossy().to_string(),
            total_bytes: probe.total_bytes,
            downloaded_bytes: 0,
            speed_bps: 0.0,
            eta_seconds: 0,
            status: DownloadStatus::Downloading,
            supports_range: probe.supports_range,
            etag: probe.etag.clone(),
            last_modified: probe.last_modified.clone(),
            checksum_sha256: None,
            error_message: None,
            concurrency: active_concurrency,
            segments,
            created_at: now_ms,
            completed_at: None,
        };

        // Preallocate .part file
        let part_path = format!("{}.part", dest_path.to_string_lossy());
        if let Ok(file) = OpenOptions::new().create(true).write(true).open(&part_path).await {
            if probe.total_bytes > 0 {
                let _ = file.set_len(probe.total_bytes).await;
            }
        }

        // Save to SQLite
        let db_rec = DbDownloadRecord {
            id: task.id.clone(),
            url: task.url.clone(),
            filename: task.filename.clone(),
            destination_path: task.destination_path.clone(),
            total_bytes: task.total_bytes,
            downloaded_bytes: 0,
            status: "downloading".to_string(),
            etag: task.etag.clone(),
            last_modified: task.last_modified.clone(),
            checksum_sha256: None,
            error_message: None,
            concurrency: task.concurrency as u32,
            created_at: task.created_at,
            completed_at: None,
        };
        let _ = self.db.save_download(&db_rec);

        self.tasks.lock().unwrap().insert(id.clone(), task);

        // Spawn parallel worker orchestration
        self.spawn_workers(&id, bandwidth_mgr);

        Ok(id)
    }

    /// Orchestrate worker tasks with Tokio
    fn spawn_workers(&self, id: &str, bandwidth_mgr: Arc<Mutex<BandwidthManager>>) {
        let (cancel_tx, _) = broadcast::channel(1);
        let is_paused = Arc::new(AtomicBool::new(false));

        self.active_controls.lock().unwrap().insert(
            id.to_string(),
            ActiveDownloadControl {
                cancel_sender: cancel_tx.clone(),
                is_paused: is_paused.clone(),
            },
        );

        let tasks_arc = self.tasks.clone();
        let db_arc = self.db.clone();
        let client = self.client.clone();
        let id_string = id.to_string();

        tokio::spawn(async move {
            let (url, part_path, segments, _total_size, supports_range) = {
                let guard = tasks_arc.lock().unwrap();
                if let Some(t) = guard.get(&id_string) {
                    (
                        t.url.clone(),
                        format!("{}.part", t.destination_path),
                        t.segments.clone(),
                        t.total_bytes,
                        t.supports_range,
                    )
                } else {
                    return;
                }
            };

            let mut worker_handles = Vec::new();

            for seg in segments {
                let client_clone = client.clone();
                let url_clone = url.clone();
                let part_path_clone = part_path.clone();
                let tasks_ref = tasks_arc.clone();
                let id_clone = id_string.clone();
                let is_paused_clone = is_paused.clone();
                let mut cancel_rx = cancel_tx.subscribe();
                let b_mgr = bandwidth_mgr.clone();

                let handle = tokio::spawn(async move {
                    run_segment_worker(
                        id_clone,
                        seg.index,
                        url_clone,
                        part_path_clone,
                        seg.start_byte,
                        seg.end_byte,
                        seg.downloaded_bytes,
                        supports_range,
                        client_clone,
                        tasks_ref,
                        is_paused_clone,
                        &mut cancel_rx,
                        b_mgr,
                    )
                    .await
                });
                worker_handles.push(handle);
            }

            // Wait for all workers to complete
            let mut all_succeeded = true;
            for h in worker_handles {
                match h.await {
                    Ok(Ok(())) => {}
                    _ => {
                        all_succeeded = false;
                    }
                }
            }

            // Check if download is 100% complete without holding mutex across await
            let is_complete = {
                let tasks_lock = tasks_arc.lock().unwrap();
                if let Some(task) = tasks_lock.get(&id_string) {
                    !is_paused.load(Ordering::Relaxed)
                        && all_succeeded
                        && task.downloaded_bytes >= task.total_bytes
                        && task.total_bytes > 0
                } else {
                    false
                }
            };

            if is_complete {
                let sha = compute_file_sha256(&part_path).await.unwrap_or_else(|_| "N/A".to_string());
                let dest_opt = {
                    let tasks_lock = tasks_arc.lock().unwrap();
                    tasks_lock.get(&id_string).map(|t| t.destination_path.clone())
                };

                if let Some(dest) = dest_opt {
                    let _ = tokio::fs::rename(&part_path, &dest).await;
                }

                let mut tasks_lock = tasks_arc.lock().unwrap();
                if let Some(task) = tasks_lock.get_mut(&id_string) {
                    task.status = DownloadStatus::Completed;
                    task.speed_bps = 0.0;
                    task.checksum_sha256 = Some(sha.clone());
                    task.completed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64);

                    let db_rec = DbDownloadRecord {
                        id: task.id.clone(),
                        url: task.url.clone(),
                        filename: task.filename.clone(),
                        destination_path: task.destination_path.clone(),
                        total_bytes: task.total_bytes,
                        downloaded_bytes: task.downloaded_bytes,
                        status: "completed".to_string(),
                        etag: task.etag.clone(),
                        last_modified: task.last_modified.clone(),
                        checksum_sha256: Some(sha),
                        error_message: None,
                        concurrency: task.concurrency as u32,
                        created_at: task.created_at,
                        completed_at: task.completed_at,
                    };
                    let _ = db_arc.save_download(&db_rec);
                }
            } else {
                let mut tasks_lock = tasks_arc.lock().unwrap();
                if let Some(task) = tasks_lock.get_mut(&id_string) {
                    if is_paused.load(Ordering::Relaxed) {
                        task.status = DownloadStatus::Paused;
                    } else if !all_succeeded {
                        task.status = DownloadStatus::Failed;
                        if task.error_message.is_none() {
                            task.error_message = Some("Connection interrupted by remote server. Click Retry (↺) to resume from where it stopped.".to_string());
                        }
                    }
                    task.speed_bps = 0.0;
                }
            }
        });
    }

    /// Pause an active download
    pub fn pause_download(&self, id: &str) -> Result<(), String> {
        if let Some(ctrl) = self.active_controls.lock().unwrap().get(id) {
            ctrl.is_paused.store(true, Ordering::Relaxed);
            let _ = ctrl.cancel_sender.send(());
        }

        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = DownloadStatus::Paused;
            task.speed_bps = 0.0;

            let db_rec = DbDownloadRecord {
                id: task.id.clone(),
                url: task.url.clone(),
                filename: task.filename.clone(),
                destination_path: task.destination_path.clone(),
                total_bytes: task.total_bytes,
                downloaded_bytes: task.downloaded_bytes,
                status: "paused".to_string(),
                etag: task.etag.clone(),
                last_modified: task.last_modified.clone(),
                checksum_sha256: None,
                error_message: None,
                concurrency: task.concurrency as u32,
                created_at: task.created_at,
                completed_at: None,
            };
            let _ = self.db.save_download(&db_rec);
            return Ok(());
        }

        Err("Download task not found".to_string())
    }

    /// Resume a paused download with ETag and Last-Modified validation
    pub async fn resume_download(
        &self,
        id: &str,
        bandwidth_mgr: Arc<Mutex<BandwidthManager>>,
    ) -> Result<(), String> {
        let (url, prev_etag, prev_lm, part_path) = {
            let tasks = self.tasks.lock().unwrap();
            if let Some(t) = tasks.get(id) {
                (
                    t.url.clone(),
                    t.etag.clone(),
                    t.last_modified.clone(),
                    format!("{}.part", t.destination_path),
                )
            } else {
                return Err("Download task not found".to_string());
            }
        };

        // Re-probe URL to validate ETag and Last-Modified
        let probe = self.probe_url(&url).await?;

        // Check if remote file changed
        let etag_changed = prev_etag.is_some() && probe.etag.is_some() && prev_etag != probe.etag;
        let lm_changed = prev_lm.is_some() && probe.last_modified.is_some() && prev_lm != probe.last_modified;

        if etag_changed || lm_changed {
            // File changed on server! Reset .part file and restart from byte 0 to prevent corruption
            let _ = tokio::fs::remove_file(&part_path).await;
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(task) = tasks.get_mut(id) {
                task.downloaded_bytes = 0;
                task.etag = probe.etag;
                task.last_modified = probe.last_modified;
                for s in &mut task.segments {
                    s.downloaded_bytes = 0;
                    s.status = SegmentStatus::Pending;
                }
            }
        }

        {
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(task) = tasks.get_mut(id) {
                task.status = DownloadStatus::Downloading;
            }
        }

        self.spawn_workers(id, bandwidth_mgr);
        Ok(())
    }

    /// Cancel download and delete partial file
    pub fn cancel_download(&self, id: &str) -> Result<(), String> {
        if let Some(ctrl) = self.active_controls.lock().unwrap().remove(id) {
            ctrl.is_paused.store(true, Ordering::Relaxed);
            let _ = ctrl.cancel_sender.send(());
        }

        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = DownloadStatus::Cancelled;
            task.speed_bps = 0.0;

            let part_path = format!("{}.part", task.destination_path);
            let _ = std::fs::remove_file(&part_path);

            let _ = self.db.delete_download(id);
            return Ok(());
        }

        Err("Download task not found".to_string())
    }

    /// Retry a failed download
    pub async fn retry_download(
        &self,
        id: &str,
        bandwidth_mgr: Arc<Mutex<BandwidthManager>>,
    ) -> Result<(), String> {
        self.resume_download(id, bandwidth_mgr).await
    }

    /// Get snapshot of all downloads
    pub fn get_all_tasks(&self) -> Vec<DownloadTask> {
        let tasks = self.tasks.lock().unwrap();
        let mut list: Vec<DownloadTask> = tasks.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }
}

/// Single segment download worker writing directly to designated byte range in target file
/// Supports self-healing auto-reconnects against transient TCP connection resets from rate-limited servers
async fn run_segment_worker(
    task_id: String,
    seg_idx: usize,
    url: String,
    part_path: String,
    start_byte: u64,
    end_byte: u64,
    current_downloaded: u64,
    supports_range: bool,
    client: reqwest::Client,
    tasks_arc: Arc<Mutex<HashMap<String, DownloadTask>>>,
    is_paused: Arc<AtomicBool>,
    cancel_rx: &mut broadcast::Receiver<()>,
    bandwidth_mgr: Arc<Mutex<BandwidthManager>>,
) -> Result<(), String> {
    let mut worker_written = current_downloaded;

    // Open target file with async write access
    let mut file = OpenOptions::new()
        .write(true)
        .open(&part_path)
        .await
        .map_err(|e| format!("Failed to open destination file: {}", e))?;

    let max_retries = 5;
    let mut attempt = 0;

    loop {
        let current_offset = start_byte + worker_written;
        if end_byte > 0 && current_offset > end_byte {
            // Segment already completed
            let mut tasks = tasks_arc.lock().unwrap();
            if let Some(task) = tasks.get_mut(&task_id) {
                if seg_idx < task.segments.len() {
                    task.segments[seg_idx].status = SegmentStatus::Completed;
                    task.segments[seg_idx].speed_bps = 0.0;
                }
            }
            break;
        }

        if is_paused.load(Ordering::Relaxed) {
            break;
        }

        file.seek(SeekFrom::Start(current_offset))
            .await
            .map_err(|e| format!("Failed to seek destination file: {}", e))?;

        let mut req = client.get(&url);
        if supports_range && end_byte > 0 {
            req = req.header(RANGE, format!("bytes={}-{}", current_offset, end_byte));
        }

        let resp_res = req.send().await;
        let mut resp = match resp_res {
            Ok(r) => {
                if !r.status().is_success() && r.status() != StatusCode::PARTIAL_CONTENT {
                    attempt += 1;
                    if attempt >= max_retries {
                        return Err(format!("Worker #{} received HTTP status: {}", seg_idx + 1, r.status()));
                    }
                    tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                    continue;
                }
                r
            }
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(format!("Worker #{} request failed after {} attempts: {}", seg_idx + 1, max_retries, e));
                }
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                continue;
            }
        };

        // Reset attempt counter on successful socket connection
        attempt = 0;

        let mut last_speed_calc = Instant::now();
        let mut bytes_since_last_calc = 0u64;
        let mut stream_interrupted = false;

        loop {
            if is_paused.load(Ordering::Relaxed) {
                break;
            }

            tokio::select! {
                _ = cancel_rx.recv() => {
                    break;
                }
                chunk_res = resp.chunk() => {
                    match chunk_res {
                        Ok(Some(chunk)) => {
                            // Strict segment boundary enforcement: prevent writing past assigned segment end byte
                            let chunk_data = if supports_range && end_byte > 0 {
                                let current_pos = start_byte + worker_written;
                                if current_pos > end_byte {
                                    break;
                                }
                                let max_allowed = (end_byte - current_pos + 1) as usize;
                                if chunk.len() > max_allowed {
                                    &chunk[..max_allowed]
                                } else {
                                    &chunk[..]
                                }
                            } else {
                                &chunk[..]
                            };

                            if chunk_data.is_empty() {
                                break;
                            }

                            let chunk_len = chunk_data.len() as u64;
                            file.write_all(chunk_data).await.map_err(|e| format!("Write failed: {}", e))?;
                            worker_written += chunk_len;

                            bytes_since_last_calc += chunk_len;

                            // Calculate speed every 250ms
                            let now = Instant::now();
                            let dt = now.duration_since(last_speed_calc).as_secs_f64();
                            let mut worker_speed = 0.0;
                            if dt >= 0.25 {
                                worker_speed = (bytes_since_last_calc as f64 * 8.0) / dt;
                                bytes_since_last_calc = 0;
                                last_speed_calc = now;
                            }

                            // Update shared task state
                            {
                                let mut tasks = tasks_arc.lock().unwrap();
                                if let Some(task) = tasks.get_mut(&task_id) {
                                    task.downloaded_bytes += chunk_len;
                                    if seg_idx < task.segments.len() {
                                        task.segments[seg_idx].downloaded_bytes += chunk_len;
                                        task.segments[seg_idx].status = SegmentStatus::Downloading;
                                        if worker_speed > 0.0 {
                                            task.segments[seg_idx].speed_bps = worker_speed;
                                        }
                                    }

                                    // Calculate overall task speed
                                    let total_speed: f64 = task.segments.iter().map(|s| s.speed_bps).sum();
                                    task.speed_bps = total_speed;

                                    // ETA
                                    if total_speed > 0.0 && task.total_bytes > task.downloaded_bytes {
                                        let remaining_bytes = task.total_bytes - task.downloaded_bytes;
                                        task.eta_seconds = ((remaining_bytes as f64 * 8.0) / total_speed).round() as u64;
                                    }
                                }
                            }

                            // Bandwidth Coordination Adaptive Pacing:
                            // Workers dynamically query the Bandwidth Coordinator for calculated pacing delay
                            // inserted between TCP chunk reads to prevent bufferbloat and buffer starvation
                            let pacing_delay_ms = {
                                let mgr = bandwidth_mgr.lock().unwrap();
                                mgr.get_pacing_delay_ms()
                            };

                            if pacing_delay_ms > 0 {
                                tokio::time::sleep(Duration::from_millis(pacing_delay_ms)).await;
                            }
                        }
                        Ok(None) => {
                            // Segment finished
                            let mut tasks = tasks_arc.lock().unwrap();
                            if let Some(task) = tasks.get_mut(&task_id) {
                                if seg_idx < task.segments.len() {
                                    task.segments[seg_idx].status = SegmentStatus::Completed;
                                    task.segments[seg_idx].speed_bps = 0.0;
                                }
                            }
                            break;
                        }
                        Err(e) => {
                            // Transient socket disconnect or timeout from server
                            log::warn!("Worker #{} chunk stream interrupted: {}. Auto-reconnecting...", seg_idx + 1, e);
                            stream_interrupted = true;
                            break;
                        }
                    }
                }
            }
        }

        if stream_interrupted && !is_paused.load(Ordering::Relaxed) {
            // Check if we still have remaining bytes to fetch
            if supports_range && end_byte > 0 && (start_byte + worker_written) <= end_byte {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(format!("Worker #{} stream interrupted and exceeded {} retries", seg_idx + 1, max_retries));
                }
                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                continue;
            }
        }

        break;
    }

    let _ = file.flush().await;
    Ok(())
}

/// Compute SHA-256 hash of completed file
async fn compute_file_sha256(path: &str) -> Result<String, String> {
    let mut file = File::open(path)
        .await
        .map_err(|e| format!("Failed to open file for checksum: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let n = file
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file for checksum: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

fn derive_filename(url: &str, headers: &reqwest::header::HeaderMap) -> String {
    // Check Content-Disposition: attachment; filename="example.zip"
    if let Some(cd) = headers.get(reqwest::header::CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
        if let Some(pos) = cd.find("filename=") {
            let sub = &cd[pos + 9..];
            let clean = sub.trim_matches(|c| c == '"' || c == ';' || c == ' ');
            if !clean.is_empty() {
                return clean.to_string();
            }
        }
    }

    // Extract from URL path
    if let Ok(parsed) = reqwest::Url::parse(url) {
        if let Some(segments) = parsed.path_segments() {
            if let Some(last) = segments.last() {
                let trimmed = last.trim();
                if !trimmed.is_empty() && trimmed.contains('.') {
                    return trimmed.to_string();
                }
            }
        }
    }

    format!("download-{}.bin", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis())
}

