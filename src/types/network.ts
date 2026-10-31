export type AdapterType = 'ethernet' | 'wifi' | 'cellular' | 'loopback' | 'other';

export interface NetworkAdapter {
  id: string;
  name: string;
  description: string;
  adapterType: AdapterType;
  isUp: boolean;
  speedBps: number;
  ipAddresses: string[];
  gateway?: string;
  macAddress: string;
  wifiSsid?: string;
  wifiSignalPercent?: number;
  bytesReceived: number;
  bytesSent: number;
}

export type HealthScore = 'excellent' | 'good' | 'moderate' | 'poor';

export interface NetworkHealth {
  score: HealthScore;
  scoreNumber: number; // 0 - 100
  reasons: string[];
  latencyGrade: string;
  jitterGrade: string;
  stabilityGrade: string;
}

export interface RealtimeMetrics {
  downloadSpeedBps: number;
  uploadSpeedBps: number;
  totalBytesReceived: number;
  totalBytesSent: number;
  latencyMs: number;
  jitterMs: number;
  packetLossPercent: number;
  activeAdapter: NetworkAdapter | null;
  health: NetworkHealth;
  timestamp: number;
}

export type OperatingMode = 'streaming' | 'balanced' | 'download';

export interface OptimizationState {
  enabled: boolean;
  mode: OperatingMode;
  congestionDetected: boolean;
  congestionReason?: string;
  activeStreamingSessions: number;
  activeDownloads: number;
  policyDescription: string;
  baselineLatencyMs?: number;
  medianLatencyMs?: number;
  pacingDelayMs?: number;
  effectiveConcurrencyFactor?: number;
}

export interface ProcessSocketConnection {
  local_addr: string;
  remote_addr: string;
  state: string;
  ip_version: string;
  is_new: boolean;
}

export interface ProcessNetworkReport {
  pid: number;
  name: string;
  path: string;
  activeConnections: number;
  connections: ProcessSocketConnection[];
  ioReadTransferBytes: number;
  ioWriteTransferBytes: number;
  ioReadRateBps: number;
  ioWriteRateBps: number;
  isStreaming: boolean;
  streamingConfidence: 'high' | 'medium' | 'low' | 'none';
  streamingReason?: string;
  bandwidthMeasurementType: string;
}

export type SegmentStatus = 'pending' | 'downloading' | 'paused' | 'completed' | 'failed';

export interface DownloadSegment {
  index: number;
  startByte: number;
  endByte: number;
  downloadedBytes: number;
  speedBps: number;
  status: SegmentStatus;
}

export type DownloadStatus =
  | 'queued'
  | 'probing'
  | 'downloading'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface DownloadTask {
  id: string;
  url: string;
  filename: string;
  destinationPath: string;
  totalBytes: number;
  downloadedBytes: number;
  speedBps: number;
  etaSeconds: number;
  status: DownloadStatus;
  supportsRange: boolean;
  etag?: string;
  lastModified?: string;
  checksumSha256?: string;
  errorMessage?: string;
  concurrency: number;
  segments: DownloadSegment[];
  createdAt: number;
  completedAt?: number;
}

export interface DbBenchmarkRecord {
  id: string;
  timestamp: number;
  mode: string;
  unloadedLatencyMs: number;
  loadedLatencyMs: number;
  jitterMs: number;
  packetLossPercent: number;
  downloadSpeedBps: number;
  completionTimeSecs: number;
  optimizationEnabled: boolean;
  scenario: string;
  notes?: string;
}

export interface BenchmarkComparison {
  baselineRun?: DbBenchmarkRecord;
  currentRun: DbBenchmarkRecord;
  latencyDeltaMs: number;
  latencyImprovementPercent: number;
  speedDeltaBps: number;
  speedChangePercent: number;
  summary: string;
}

export interface StreamingConfig {
  defaultApps: string[];
  userConfiguredApps: string[];
}

export interface DnsBenchmarkResult {
  name: string;
  primaryIp: string;
  secondaryIp: string;
  description: string;
  medianLatencyMs: number;
  minLatencyMs: number;
  maxLatencyMs: number;
  successRatePercent: number;
  reliabilityPercent: number;
  isCurrent: boolean;
  isRecommended: boolean;
}

export interface DnsBenchmarkReport {
  providers: DnsBenchmarkResult[];
  recommendedProvider?: string;
  currentDns?: string;
  activeInterface?: string;
  disclaimer: string;
}

export interface DnsBackupRecord {
  interfaceName: string;
  isDhcp: boolean;
  primaryDns?: string;
  secondaryDns?: string;
  backedUpAt: number;
}

export interface AppSettings {
  minimizeToTray: boolean;
  launchOnStartup: boolean;
  bufferbloatWarning: boolean;
  defaultMode: string;
}


