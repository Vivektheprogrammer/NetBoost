import { invoke } from '@tauri-apps/api/core';
import { 
  RealtimeMetrics, 
  NetworkAdapter, 
  ProcessNetworkReport, 
  OptimizationState,
  DownloadTask,
  OperatingMode,
  DbBenchmarkRecord,
  BenchmarkComparison,
  StreamingConfig,
  DnsBenchmarkReport,
  DnsBackupRecord,
  AppSettings
} from '../types/network';

export const isTauriEnvironment = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// --- Network & Health ---
export async function getRealtimeMetrics(): Promise<RealtimeMetrics> {
  if (isTauriEnvironment()) {
    return await invoke<RealtimeMetrics>('get_realtime_metrics');
  }
  return generatePreviewMetrics();
}

export async function getNetworkAdapters(): Promise<NetworkAdapter[]> {
  if (isTauriEnvironment()) {
    return await invoke<NetworkAdapter[]>('get_network_adapters');
  }
  return [];
}

// --- Application Network Monitoring ---
export async function getActiveProcesses(): Promise<ProcessNetworkReport[]> {
  if (isTauriEnvironment()) {
    return await invoke<ProcessNetworkReport[]>('get_active_processes');
  }
  return generatePreviewProcesses();
}

// --- Bandwidth Optimization Modes ---
export async function getOptimizationState(): Promise<OptimizationState> {
  if (isTauriEnvironment()) {
    return await invoke<OptimizationState>('get_optimization_state');
  }
  return {
    enabled: true,
    mode: 'streaming',
    congestionDetected: false,
    activeStreamingSessions: 0,
    activeDownloads: 0,
    policyDescription: 'Streaming Priority: Protecting video buffer and reserving responsive latency.'
  };
}

export async function setOptimizationMode(mode: OperatingMode): Promise<OptimizationState> {
  if (isTauriEnvironment()) {
    return await invoke<OptimizationState>('set_optimization_mode', { mode });
  }
  return {
    enabled: true,
    mode,
    congestionDetected: false,
    activeStreamingSessions: 0,
    activeDownloads: 0,
    policyDescription: `Mode switched to ${mode}.`
  };
}

export async function toggleOptimization(enabled: boolean): Promise<OptimizationState> {
  if (isTauriEnvironment()) {
    return await invoke<OptimizationState>('toggle_optimization', { enabled });
  }
  return {
    enabled,
    mode: 'streaming',
    congestionDetected: false,
    activeStreamingSessions: 0,
    activeDownloads: 0,
    policyDescription: enabled ? 'Optimization enabled' : 'Optimization paused'
  };
}

// --- Download Accelerator Commands ---
export async function startDownload(
  url: string,
  customFilename?: string,
  concurrency?: number
): Promise<string> {
  if (isTauriEnvironment()) {
    return await invoke<string>('start_download', {
      url,
      customFilename,
      concurrency
    });
  }
  return `mock-${Date.now()}`;
}

export async function pauseDownload(id: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('pause_download', { id });
  }
}

export async function resumeDownload(id: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('resume_download', { id });
  }
}

export async function cancelDownload(id: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('cancel_download', { id });
  }
}

export async function retryDownload(id: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('retry_download', { id });
  }
}

export async function getDownloads(): Promise<DownloadTask[]> {
  if (isTauriEnvironment()) {
    return await invoke<DownloadTask[]>('get_downloads');
  }
  return [];
}

export async function openDownloadedFile(path: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('open_downloaded_file', { path });
  }
}

export async function openDownloadFolder(path: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('open_download_folder', { path });
  }
}

export async function openExternalUrl(url: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('open_external_url', { url });
  } else {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}

export async function openExtensionFolder(): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('open_extension_folder');
  }
}

// --- Benchmarking Engine Commands ---
export async function runBenchmark(
  scenario: string,
  targetUrl?: string
): Promise<DbBenchmarkRecord> {
  if (isTauriEnvironment()) {
    return await invoke<DbBenchmarkRecord>('run_benchmark', {
      scenario,
      targetUrl
    });
  }
  return {
    id: `bm-${Date.now()}`,
    timestamp: Date.now(),
    mode: 'streaming',
    unloadedLatencyMs: 14.5,
    loadedLatencyMs: 22.8,
    jitterMs: 2.1,
    packetLossPercent: 0,
    downloadSpeedBps: 45000000,
    completionTimeSecs: 4.2,
    optimizationEnabled: true,
    scenario,
    notes: 'Preview benchmark measurement.'
  };
}

export async function getBenchmarkHistory(): Promise<DbBenchmarkRecord[]> {
  if (isTauriEnvironment()) {
    return await invoke<DbBenchmarkRecord[]>('get_benchmark_history');
  }
  return [];
}

export async function clearBenchmarkHistory(): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('clear_benchmark_history');
  }
}

export async function getBenchmarkComparison(currentId: string): Promise<BenchmarkComparison> {
  if (isTauriEnvironment()) {
    return await invoke<BenchmarkComparison>('get_benchmark_comparison', { currentId });
  }
  return {
    currentRun: {
      id: currentId,
      timestamp: Date.now(),
      mode: 'streaming',
      unloadedLatencyMs: 14.5,
      loadedLatencyMs: 22.8,
      jitterMs: 2.1,
      packetLossPercent: 0,
      downloadSpeedBps: 45000000,
      completionTimeSecs: 4.2,
      optimizationEnabled: true,
      scenario: 'Streaming Priority Test'
    },
    latencyDeltaMs: -34.2,
    latencyImprovementPercent: 60.0,
    speedDeltaBps: -4000000,
    speedChangePercent: -8.1,
    summary: 'Bufferbloat reduced by 34.2 ms (60% latency improvement) compared to unoptimized baseline.'
  };
}

// --- Streaming Heuristics & App Configuration ---
export async function getStreamingConfig(): Promise<StreamingConfig> {
  if (isTauriEnvironment()) {
    return await invoke<StreamingConfig>('get_streaming_config');
  }
  return {
    defaultApps: ['chrome.exe', 'msedge.exe', 'firefox.exe', 'netflix.exe', 'spotify.exe', 'vlc.exe'],
    userConfiguredApps: []
  };
}

export async function addUserStreamingApp(appName: string): Promise<StreamingConfig> {
  if (isTauriEnvironment()) {
    return await invoke<StreamingConfig>('add_user_streaming_app', { appName });
  }
  return {
    defaultApps: ['chrome.exe', 'msedge.exe', 'firefox.exe'],
    userConfiguredApps: [appName.toLowerCase()]
  };
}

export async function removeUserStreamingApp(appName: string): Promise<StreamingConfig> {
  if (isTauriEnvironment()) {
    return await invoke<StreamingConfig>('remove_user_streaming_app', { appName });
  }
  return {
    defaultApps: ['chrome.exe', 'msedge.exe', 'firefox.exe'],
    userConfiguredApps: []
  };
}

// --- DNS Optimizer Commands ---
export async function benchmarkDns(): Promise<DnsBenchmarkReport> {
  if (isTauriEnvironment()) {
    return await invoke<DnsBenchmarkReport>('benchmark_dns');
  }
  return {
    providers: [
      {
        name: 'Cloudflare DNS',
        primaryIp: '1.1.1.1',
        secondaryIp: '1.0.0.1',
        description: 'Fastest consumer DNS with strict privacy and zero logging.',
        medianLatencyMs: 11.2,
        minLatencyMs: 9.8,
        maxLatencyMs: 14.5,
        successRatePercent: 100,
        reliabilityPercent: 100,
        isCurrent: false,
        isRecommended: true
      },
      {
        name: 'Google Public DNS',
        primaryIp: '8.8.8.8',
        secondaryIp: '8.8.4.4',
        description: 'Global distributed anycast routing with high cache hit rates.',
        medianLatencyMs: 14.8,
        minLatencyMs: 12.1,
        maxLatencyMs: 18.2,
        successRatePercent: 100,
        reliabilityPercent: 100,
        isCurrent: false,
        isRecommended: false
      },
      {
        name: 'Quad9 DNS',
        primaryIp: '9.9.9.9',
        secondaryIp: '149.112.112.112',
        description: 'Threat-blocking security DNS with zero malware redirection.',
        medianLatencyMs: 16.5,
        minLatencyMs: 14.0,
        maxLatencyMs: 21.0,
        successRatePercent: 100,
        reliabilityPercent: 100,
        isCurrent: false,
        isRecommended: false
      },
      {
        name: 'OpenDNS (Cisco)',
        primaryIp: '208.67.222.222',
        secondaryIp: '208.67.220.220',
        description: 'PhishTank protected enterprise recursive resolver.',
        medianLatencyMs: 18.2,
        minLatencyMs: 15.4,
        maxLatencyMs: 25.0,
        successRatePercent: 100,
        reliabilityPercent: 100,
        isCurrent: false,
        isRecommended: false
      }
    ],
    recommendedProvider: 'Cloudflare DNS',
    currentDns: '192.168.1.1',
    activeInterface: 'Wi-Fi',
    disclaimer: 'DNS optimization reduces domain lookup latency (Time To First Byte). It does NOT increase maximum ISP download/upload bandwidth.'
  };
}

export async function applyDnsProvider(
  interfaceName: string,
  primaryIp: string,
  secondaryIp?: string
): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('apply_dns_provider', {
      interfaceName,
      primaryIp,
      secondaryIp
    });
  }
}

export async function restoreOriginalDns(interfaceName: string): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('restore_original_dns', { interfaceName });
  }
}

export async function getDnsBackupStatus(interfaceName: string): Promise<DnsBackupRecord | null> {
  if (isTauriEnvironment()) {
    return await invoke<DnsBackupRecord | null>('get_dns_backup_status', { interfaceName });
  }
  return null;
}

// --- Application Settings & Production Controls ---
export async function getAppSettings(): Promise<AppSettings> {
  if (isTauriEnvironment()) {
    return await invoke<AppSettings>('get_app_settings');
  }
  return {
    minimizeToTray: true,
    launchOnStartup: false,
    bufferbloatWarning: true,
    defaultMode: 'streaming'
  };
}

export async function saveAppSettings(settings: AppSettings): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('save_app_settings', { settings });
  }
}

export async function getApplicationLogs(): Promise<string[]> {
  if (isTauriEnvironment()) {
    return await invoke<string[]>('get_application_logs');
  }
  return ['[INFO] Development fallback environment'];
}

// --- Mock / Development Fallbacks ---
let lastDown = 2500000;
let lastUp = 320000;
function generatePreviewMetrics(): RealtimeMetrics {
  lastDown = Math.max(100000, lastDown + (Math.random() - 0.48) * 800000);
  lastUp = Math.max(50000, lastUp + (Math.random() - 0.5) * 150000);
  return {
    downloadSpeedBps: lastDown,
    uploadSpeedBps: lastUp,
    totalBytesReceived: 1420500120,
    totalBytesSent: 135800420,
    latencyMs: 14 + Math.round(Math.random() * 4),
    jitterMs: 1.8 + Math.random() * 0.8,
    packetLossPercent: 0,
    activeAdapter: {
      id: '{PREVIEW}',
      name: 'Wi-Fi 6',
      description: 'Intel(R) Wi-Fi 6E AX211 160MHz',
      adapterType: 'wifi',
      isUp: true,
      speedBps: 1200000000,
      ipAddresses: ['192.168.0.142'],
      gateway: '192.168.0.1',
      macAddress: 'F0:D5:BF:73:9A:12',
      wifiSsid: 'HomeFiber_5G',
      wifiSignalPercent: 92,
      bytesReceived: 1420500120,
      bytesSent: 135800420,
    },
    health: {
      score: 'excellent',
      scoreNumber: 96,
      reasons: ['Low unloaded latency (15ms)', 'Minimal jitter (<2ms)', 'Zero packet loss'],
      latencyGrade: 'A+',
      jitterGrade: 'A+',
      stabilityGrade: '100%'
    },
    timestamp: Date.now()
  };
}

function generatePreviewProcesses(): ProcessNetworkReport[] {
  return [
    {
      pid: 14208,
      name: 'chrome.exe',
      path: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
      activeConnections: 8,
      connections: [
        { local_addr: '192.168.1.105:54210', remote_addr: '142.250.190.46:443', state: 'ESTABLISHED', ip_version: 'IPv4', is_new: false },
        { local_addr: '192.168.1.105:54211', remote_addr: '142.250.190.46:443', state: 'ESTABLISHED', ip_version: 'IPv4', is_new: false },
        { local_addr: '192.168.1.105:54212', remote_addr: '172.217.16.206:443', state: 'ESTABLISHED', ip_version: 'IPv4', is_new: true },
      ],
      ioReadTransferBytes: 154210000,
      ioWriteTransferBytes: 12400000,
      ioReadRateBps: 2450000,
      ioWriteRateBps: 120000,
      isStreaming: true,
      streamingConfidence: 'high',
      streamingReason: 'Active video stream (2.5 Mbps sustained read across HTTPS media streams).',
      bandwidthMeasurementType: 'Process I/O Transfer Rate (Measured via Win32 GetProcessIoCounters)'
    },
    {
      pid: 8940,
      name: 'spotify.exe',
      path: 'C:\\Users\\user\\AppData\\Roaming\\Spotify\\Spotify.exe',
      activeConnections: 4,
      connections: [
        { local_addr: '192.168.1.105:51200', remote_addr: '104.199.65.124:443', state: 'ESTABLISHED', ip_version: 'IPv4', is_new: false }
      ],
      ioReadTransferBytes: 42100000,
      ioWriteTransferBytes: 2400000,
      ioReadRateBps: 320000,
      ioWriteRateBps: 15000,
      isStreaming: true,
      streamingConfidence: 'high',
      streamingReason: 'Dedicated audio streaming media process with active sockets.',
      bandwidthMeasurementType: 'Process I/O Transfer Rate (Measured via Win32 GetProcessIoCounters)'
    }
  ];
}
