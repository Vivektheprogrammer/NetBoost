// Format bytes per second to human-readable network speeds (e.g. 85.4 Mbps)
export function formatSpeed(bytesPerSec: number): string {
  const bitsPerSec = bytesPerSec * 8;
  if (bitsPerSec >= 1_000_000_000) {
    return `${(bitsPerSec / 1_000_000_000).toFixed(1)} Gbps`;
  }
  if (bitsPerSec >= 1_000_000) {
    return `${(bitsPerSec / 1_000_000).toFixed(1)} Mbps`;
  }
  if (bitsPerSec >= 1_000) {
    return `${(bitsPerSec / 1_000).toFixed(0)} Kbps`;
  }
  return `${bitsPerSec.toFixed(0)} bps`;
}

// Format byte amounts (e.g. 1.42 GB)
export function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  }
  if (bytes >= 1_048_576) {
    return `${(bytes / 1_048_576).toFixed(1)} MB`;
  }
  if (bytes >= 1_024) {
    return `${(bytes / 1_024).toFixed(0)} KB`;
  }
  return `${bytes} B`;
}

export function formatLatency(ms: number): string {
  if (ms < 0) return '—';
  return `${Math.round(ms)} ms`;
}

export function formatJitter(ms: number): string {
  if (ms < 0) return '—';
  return `${ms.toFixed(1)} ms`;
}

// Format duration / ETA into human-readable hours, minutes, and seconds (e.g. "3m 23s", "1h 12m")
export function formatEta(seconds: number): string {
  if (!seconds || seconds <= 0 || !isFinite(seconds)) return '—';
  const totalSecs = Math.round(seconds);
  if (totalSecs < 60) {
    return `${totalSecs}s`;
  }
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  if (mins < 60) {
    return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
  }
  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  return remMins > 0 ? `${hours}h ${remMins}m` : `${hours}h`;
}

