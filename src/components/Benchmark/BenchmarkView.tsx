import React, { useState, useEffect } from 'react';
import { 
  Gauge, 
  Play, 
  CheckCircle2, 
  AlertCircle, 
  ExternalLink, 
  Trash2, 
  History, 
  RefreshCw,
  TrendingDown,
  TrendingUp,
  Activity
} from 'lucide-react';
import { DbBenchmarkRecord, BenchmarkComparison } from '../../types/network';
import { 
  runBenchmark, 
  getBenchmarkHistory, 
  clearBenchmarkHistory, 
  getBenchmarkComparison,
  openExternalUrl
} from '../../services/tauriCommands';

export const BenchmarkView: React.FC = () => {
  const [running, setRunning] = useState(false);
  const [history, setHistory] = useState<DbBenchmarkRecord[]>([]);
  const [latestRun, setLatestRun] = useState<DbBenchmarkRecord | null>(null);
  const [comparison, setComparison] = useState<BenchmarkComparison | null>(null);
  const [scenario, setScenario] = useState<string>('streaming_priority');
  const [targetUrl, setTargetUrl] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  const loadHistory = async () => {
    try {
      const records = await getBenchmarkHistory();
      setHistory(records);
      if (records.length > 0 && !latestRun) {
        setLatestRun(records[0]);
        loadComparison(records[0].id);
      }
    } catch (e: any) {
      console.error('Failed to load benchmark history:', e);
    }
  };

  const loadComparison = async (id: string) => {
    try {
      const comp = await getBenchmarkComparison(id);
      setComparison(comp);
    } catch (e) {
      console.warn('Comparison failed:', e);
    }
  };

  useEffect(() => {
    loadHistory();
  }, []);

  const handleRunBenchmark = async () => {
    setRunning(true);
    setError(null);
    try {
      const result = await runBenchmark(scenario, targetUrl.trim() || undefined);
      setLatestRun(result);
      await loadHistory();
      await loadComparison(result.id);
    } catch (e: any) {
      setError(e?.toString() || 'Benchmark execution failed');
    } finally {
      setRunning(false);
    }
  };

  const handleClearHistory = async () => {
    if (window.confirm('Clear all local benchmark records from SQLite?')) {
      await clearBenchmarkHistory();
      setHistory([]);
      setLatestRun(null);
      setComparison(null);
    }
  };

  return (
    <div className="space-y-4">
      {/* Benchmark Control Panel */}
      <div className="glass-panel p-4 rounded-xl border border-slate-800 space-y-4">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
          <div>
            <div className="flex items-center space-x-2">
              <Gauge className="w-4 h-4 text-cyan-400" />
              <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
                Network Performance & Bufferbloat Benchmarking
              </h3>
            </div>
            <p className="text-xs text-slate-400 mt-1">
              Executes controlled network loads and statistically samples loaded latency, jitter variance, and segmented throughput.
            </p>
          </div>

          <button
            onClick={handleRunBenchmark}
            disabled={running}
            className="px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-white font-semibold text-xs rounded-lg flex items-center space-x-2 transition-all shadow-md shadow-cyan-500/20 disabled:opacity-50 shrink-0"
          >
            {running ? (
              <>
                <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                <span>Running Test (Sampling Probes)...</span>
              </>
            ) : (
              <>
                <Play className="w-3.5 h-3.5 fill-current" />
                <span>Execute Benchmark</span>
              </>
            )}
          </button>
        </div>

        {/* Configuration Options */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-2 border-t border-slate-800/80">
          <div>
            <label className="block text-[11px] font-mono uppercase text-slate-400 mb-1">
              Test Scenario & Concurrency Mode
            </label>
            <select
              value={scenario}
              onChange={(e) => setScenario(e.target.value)}
              disabled={running}
              className="w-full bg-slate-900 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
            >
              <option value="streaming_priority">Streaming Priority (Protected Latency Pacing)</option>
              <option value="balanced">Balanced Mode (Fair Bandwidth Distribution)</option>
              <option value="download_priority">Download Priority (Max Segmented Throughput)</option>
              <option value="single_download">Single-Worker Download Load</option>
              <option value="idle_only">Idle Baseline (Zero Download Load)</option>
            </select>
          </div>

          <div>
            <label className="block text-[11px] font-mono uppercase text-slate-400 mb-1">
              Target Download Test URL (Optional)
            </label>
            <input
              type="text"
              placeholder="Leave empty for Cloudflare Anycast test (5 MB)"
              value={targetUrl}
              onChange={(e) => setTargetUrl(e.target.value)}
              disabled={running}
              className="w-full bg-slate-900 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 placeholder-slate-600 focus:outline-none focus:border-cyan-500"
            />
          </div>
        </div>

        {error && (
          <div className="p-3 bg-rose-500/10 border border-rose-500/30 rounded-lg text-xs text-rose-400 flex items-center space-x-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{error}</span>
          </div>
        )}
      </div>

      {/* Latest Run Results Cards */}
      {latestRun && (
        <div className="space-y-3">
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <div className="glass-panel p-3 rounded-xl border border-slate-800">
              <span className="text-[10px] font-mono uppercase text-slate-400">Idle Latency</span>
              <div className="text-lg font-mono font-bold text-slate-200 mt-0.5">
                {latestRun.unloadedLatencyMs.toFixed(1)} <span className="text-xs font-normal text-slate-400">ms</span>
              </div>
              <span className="text-[10px] text-slate-500">Unloaded network baseline</span>
            </div>

            <div className="glass-panel p-3 rounded-xl border border-slate-800">
              <span className="text-[10px] font-mono uppercase text-slate-400">Loaded Latency</span>
              <div className="text-lg font-mono font-bold text-cyan-400 mt-0.5">
                {latestRun.loadedLatencyMs.toFixed(1)} <span className="text-xs font-normal text-slate-400">ms</span>
              </div>
              <div className="text-[10px] font-mono flex items-center space-x-1 mt-0.5">
                <span className={latestRun.loadedLatencyMs - latestRun.unloadedLatencyMs < 20 ? 'text-emerald-400' : 'text-amber-400'}>
                  Bufferbloat: +{(latestRun.loadedLatencyMs - latestRun.unloadedLatencyMs).toFixed(1)} ms
                </span>
              </div>
            </div>

            <div className="glass-panel p-3 rounded-xl border border-slate-800">
              <span className="text-[10px] font-mono uppercase text-slate-400">Throughput</span>
              <div className="text-lg font-mono font-bold text-emerald-400 mt-0.5">
                {(latestRun.downloadSpeedBps / 1_000_000).toFixed(1)} <span className="text-xs font-normal text-slate-400">Mbps</span>
              </div>
              <span className="text-[10px] text-slate-500">{latestRun.completionTimeSecs.toFixed(2)}s test duration</span>
            </div>

            <div className="glass-panel p-3 rounded-xl border border-slate-800">
              <span className="text-[10px] font-mono uppercase text-slate-400">Jitter / Loss</span>
              <div className="text-lg font-mono font-bold text-slate-200 mt-0.5">
                {latestRun.jitterMs.toFixed(1)} <span className="text-xs font-normal text-slate-400">ms</span>
              </div>
              <span className="text-[10px] text-slate-500">{latestRun.packetLossPercent.toFixed(1)}% packet drop</span>
            </div>
          </div>

          {/* Comparative Assessment Card */}
          {comparison && (
            <div className="glass-panel p-3.5 rounded-xl border border-cyan-500/30 bg-gradient-to-r from-cyan-950/20 to-slate-900 flex items-start justify-between gap-3">
              <div className="flex items-start space-x-2.5">
                <Activity className="w-4 h-4 text-cyan-400 mt-0.5 shrink-0" />
                <div>
                  <h4 className="text-xs font-semibold text-slate-200">
                    Comparative Performance Summary
                  </h4>
                  <p className="text-xs text-slate-300 mt-0.5 font-mono">
                    {comparison.summary}
                  </p>
                  {comparison.baselineRun && (
                    <div className="text-[11px] text-slate-400 mt-1 flex items-center space-x-3">
                      <span>Baseline: {comparison.baselineRun.scenario} ({comparison.baselineRun.loadedLatencyMs.toFixed(1)} ms)</span>
                      <span>•</span>
                      <span>Current: {latestRun.scenario} ({latestRun.loadedLatencyMs.toFixed(1)} ms)</span>
                    </div>
                  )}
                </div>
              </div>

              {comparison.latencyImprovementPercent > 0 && (
                <div className="px-2.5 py-1 bg-emerald-500/10 border border-emerald-500/30 rounded-lg text-emerald-400 text-xs font-mono font-bold flex items-center space-x-1 shrink-0">
                  <TrendingDown className="w-3.5 h-3.5" />
                  <span>{comparison.latencyImprovementPercent.toFixed(1)}% Lower Lag</span>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Historical SQLite Benchmark Runs Table */}
      <div className="glass-panel rounded-xl border border-slate-800 overflow-hidden">
        <div className="p-3.5 bg-slate-900/80 border-b border-slate-800 flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <History className="w-3.5 h-3.5 text-slate-400" />
            <span className="text-xs font-semibold uppercase tracking-wider text-slate-300">
              Benchmark Run History ({history.length} runs stored in SQLite)
            </span>
          </div>

          {history.length > 0 && (
            <button
              onClick={handleClearHistory}
              className="text-[11px] text-slate-400 hover:text-rose-400 flex items-center space-x-1 transition-colors"
            >
              <Trash2 className="w-3 h-3" />
              <span>Clear History</span>
            </button>
          )}
        </div>

        {history.length === 0 ? (
          <div className="p-6 text-center text-xs text-slate-500 font-mono">
            No benchmarks recorded yet. Click "Execute Benchmark" to start your first network audit.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse text-xs">
              <thead>
                <tr className="border-b border-slate-800/80 bg-slate-900/40 text-slate-400 text-[10px] uppercase font-mono tracking-wider">
                  <th className="py-2.5 px-3">Date & Time</th>
                  <th className="py-2.5 px-3">Scenario</th>
                  <th className="py-2.5 px-3">Mode</th>
                  <th className="py-2.5 px-3">Idle Ping</th>
                  <th className="py-2.5 px-3">Loaded Ping</th>
                  <th className="py-2.5 px-3">Throughput</th>
                  <th className="py-2.5 px-3">Jitter</th>
                  <th className="py-2.5 px-3 text-right">Action</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono">
                {history.map((r) => {
                  const dateStr = new Date(r.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
                  const isSelected = latestRun?.id === r.id;
                  return (
                    <tr 
                      key={r.id} 
                      className={`hover:bg-slate-800/30 transition-colors ${isSelected ? 'bg-cyan-500/5' : ''}`}
                    >
                      <td className="py-2.5 px-3 text-slate-400">{dateStr}</td>
                      <td className="py-2.5 px-3 font-medium text-slate-200 capitalize">{r.scenario.replace('_', ' ')}</td>
                      <td className="py-2.5 px-3">
                        <span className={`px-2 py-0.5 rounded text-[10px] uppercase font-bold ${
                          r.mode === 'streaming' ? 'bg-purple-500/20 text-purple-300' :
                          r.mode === 'balanced' ? 'bg-blue-500/20 text-blue-300' :
                          r.mode === 'download' ? 'bg-amber-500/20 text-amber-300' :
                          'bg-slate-700 text-slate-300'
                        }`}>
                          {r.mode}
                        </span>
                      </td>
                      <td className="py-2.5 px-3 text-slate-400">{r.unloadedLatencyMs.toFixed(1)} ms</td>
                      <td className="py-2.5 px-3 text-cyan-300 font-bold">{r.loadedLatencyMs.toFixed(1)} ms</td>
                      <td className="py-2.5 px-3 text-emerald-400">{(r.downloadSpeedBps / 1_000_000).toFixed(1)} Mbps</td>
                      <td className="py-2.5 px-3 text-slate-400">{r.jitterMs.toFixed(1)} ms</td>
                      <td className="py-2.5 px-3 text-right">
                        <button
                          onClick={() => {
                            setLatestRun(r);
                            loadComparison(r.id);
                          }}
                          className="text-[10px] text-cyan-400 hover:text-cyan-300 font-sans font-semibold underline"
                        >
                          View Delta
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* External Benchmarking Links */}
      <div className="glass-panel p-4 rounded-xl border border-slate-800 space-y-2">
        <h4 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
          Independent External Speed & Bufferbloat Tests
        </h4>
        <p className="text-xs text-slate-400">
          Cross-verify NetBoost's latency pacing and queue management against independent web testing platforms:
        </p>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 pt-2">
          <button
            type="button"
            onClick={() => openExternalUrl("https://www.waveform.com/tools/bufferbloat")}
            className="p-2.5 bg-slate-900/80 hover:bg-slate-800/80 border border-slate-800 rounded-lg flex items-center justify-between text-xs text-slate-300 transition-colors text-left cursor-pointer"
          >
            <span>Waveform Bufferbloat Test</span>
            <ExternalLink className="w-3.5 h-3.5 text-slate-500" />
          </button>

          <button
            type="button"
            onClick={() => openExternalUrl("https://speed.cloudflare.com")}
            className="p-2.5 bg-slate-900/80 hover:bg-slate-800/80 border border-slate-800 rounded-lg flex items-center justify-between text-xs text-slate-300 transition-colors text-left cursor-pointer"
          >
            <span>Cloudflare Speed & Jitter</span>
            <ExternalLink className="w-3.5 h-3.5 text-slate-500" />
          </button>

          <button
            type="button"
            onClick={() => openExternalUrl("https://fast.com")}
            className="p-2.5 bg-slate-900/80 hover:bg-slate-800/80 border border-slate-800 rounded-lg flex items-center justify-between text-xs text-slate-300 transition-colors text-left cursor-pointer"
          >
            <span>FAST.com (Netflix CDN)</span>
            <ExternalLink className="w-3.5 h-3.5 text-slate-500" />
          </button>
        </div>
      </div>
    </div>
  );
};
