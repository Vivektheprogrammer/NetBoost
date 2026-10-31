import React, { useState, useEffect } from 'react';
import { 
  Globe, 
  Check, 
  AlertCircle, 
  Play, 
  Shield, 
  RefreshCw, 
  Undo2, 
  Zap,
  Server,
  Info
} from 'lucide-react';
import { DnsBenchmarkReport, DnsBenchmarkResult, DnsBackupRecord } from '../../types/network';
import { 
  benchmarkDns, 
  applyDnsProvider, 
  restoreOriginalDns, 
  getDnsBackupStatus 
} from '../../services/tauriCommands';

export const DnsOptimizer: React.FC = () => {
  const [testing, setTesting] = useState(false);
  const [report, setReport] = useState<DnsBenchmarkReport | null>(null);
  const [backup, setBackup] = useState<DnsBackupRecord | null>(null);
  const [applying, setApplying] = useState(false);
  const [statusMessage, setStatusMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [confirmModal, setConfirmModal] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    confirmText: string;
    onConfirm: () => void;
  } | null>(null);

  const checkBackup = async (ifaceName?: string) => {
    try {
      const iface = ifaceName || report?.activeInterface || 'Wi-Fi';
      const b = await getDnsBackupStatus(iface);
      setBackup(b);
    } catch (e) {
      console.warn('Backup check error:', e);
    }
  };

  const runBenchmark = async () => {
    setTesting(true);
    setStatusMessage(null);
    try {
      const res = await benchmarkDns();
      setReport(res);
      await checkBackup(res.activeInterface);
    } catch (e: any) {
      setStatusMessage({ type: 'error', text: e?.toString() || 'DNS benchmark failed.' });
    } finally {
      setTesting(false);
    }
  };

  useEffect(() => {
    runBenchmark();
  }, []);

  const executeApply = async (provider: DnsBenchmarkResult) => {
    const iface = report?.activeInterface || 'Wi-Fi';
    setApplying(true);
    setStatusMessage(null);
    try {
      await applyDnsProvider(iface, provider.primaryIp, provider.secondaryIp);
      setStatusMessage({
        type: 'success',
        text: `Successfully applied ${provider.name} to ${iface}. Original configuration backed up.`
      });
      await runBenchmark();
    } catch (e: any) {
      setStatusMessage({
        type: 'error',
        text: e?.toString() || 'Failed to apply DNS. Please ensure NetBoost has Administrator permissions if required by Windows.'
      });
    } finally {
      setApplying(false);
    }
  };

  const handleApply = (provider: DnsBenchmarkResult) => {
    const iface = report?.activeInterface || 'Wi-Fi';
    setConfirmModal({
      isOpen: true,
      title: 'NetBoost System Confirmation',
      message: `Apply ${provider.name} (${provider.primaryIp}, ${provider.secondaryIp}) to network adapter "${iface}"?\n\nYour current configuration will be automatically backed up for 1-click restoration.`,
      confirmText: `Apply ${provider.name}`,
      onConfirm: () => {
        setConfirmModal(null);
        executeApply(provider);
      }
    });
  };

  const executeRestore = async () => {
    const iface = report?.activeInterface || 'Wi-Fi';
    setApplying(true);
    setStatusMessage(null);
    try {
      await restoreOriginalDns(iface);
      setStatusMessage({
        type: 'success',
        text: `Restored original DNS configuration for ${iface}.`
      });
      await runBenchmark();
    } catch (e: any) {
      setStatusMessage({
        type: 'error',
        text: e?.toString() || 'Failed to restore DNS.'
      });
    } finally {
      setApplying(false);
    }
  };

  const handleRestore = () => {
    const iface = report?.activeInterface || 'Wi-Fi';
    setConfirmModal({
      isOpen: true,
      title: 'NetBoost System Confirmation',
      message: `Restore original DNS configuration for adapter "${iface}"?`,
      confirmText: 'Restore Original DNS',
      onConfirm: () => {
        setConfirmModal(null);
        executeRestore();
      }
    });
  };

  return (
    <div className="space-y-4">
      {/* Header Card */}
      <div className="glass-panel p-4 rounded-xl border border-slate-800 flex flex-col sm:flex-row items-center justify-between gap-3">
        <div>
          <div className="flex items-center space-x-2">
            <Globe className="w-4 h-4 text-cyan-400" />
            <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
              DNS Resolution Latency Optimizer
            </h3>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Sends real UDP queries to major anycast resolvers to reduce hostname lookup latency (Time To First Byte).
          </p>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          {backup && (
            <button
              onClick={handleRestore}
              disabled={applying || testing}
              className="px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 rounded-lg text-xs font-medium flex items-center space-x-1.5 transition-colors disabled:opacity-50"
            >
              <Undo2 className="w-3.5 h-3.5 text-amber-400" />
              <span>Restore Original DNS</span>
            </button>
          )}

          <button
            onClick={runBenchmark}
            disabled={testing || applying}
            className="px-4 py-2 bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-white rounded-lg text-xs font-semibold flex items-center space-x-1.5 transition-all shadow-md shadow-cyan-500/20 disabled:opacity-50"
          >
            {testing ? <RefreshCw className="w-3.5 h-3.5 animate-spin" /> : <Play className="w-3.5 h-3.5 fill-current" />}
            <span>{testing ? 'Benchmarking UDP Queries...' : 'Run DNS Benchmark'}</span>
          </button>
        </div>
      </div>

      {/* Backup Active Alert */}
      {backup && (
        <div className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between text-xs text-amber-300">
          <div className="flex items-center space-x-2">
            <Shield className="w-4 h-4 text-amber-400 shrink-0" />
            <span>
              <strong>Custom DNS Active on "{backup.interfaceName}": </strong>
              Original setting ({backup.isDhcp ? 'DHCP Automatic' : `Static: ${backup.primaryDns || 'N/A'}`}) is backed up in SQLite.
            </span>
          </div>
          <button
            onClick={handleRestore}
            className="text-[11px] underline hover:text-amber-200 shrink-0 ml-2"
          >
            Restore Defaults
          </button>
        </div>
      )}

      {/* Status Notifications */}
      {statusMessage && (
        <div className={`p-3 rounded-xl text-xs flex items-center space-x-2 ${
          statusMessage.type === 'success'
            ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-300'
            : 'bg-rose-500/10 border border-rose-500/30 text-rose-400'
        }`}>
          {statusMessage.type === 'success' ? <Check className="w-4 h-4 shrink-0" /> : <AlertCircle className="w-4 h-4 shrink-0" />}
          <span>{statusMessage.text}</span>
        </div>
      )}

      {/* Provider Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {report?.providers.map((p, idx) => (
          <div
            key={idx}
            className={`glass-panel p-4 rounded-xl border transition-all flex flex-col justify-between ${
              p.isRecommended
                ? 'border-cyan-500/50 bg-cyan-950/20 shadow-lg shadow-cyan-500/5'
                : p.isCurrent
                ? 'border-emerald-500/40 bg-emerald-950/10'
                : 'border-slate-800 hover:border-slate-700'
            }`}
          >
            <div>
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center space-x-2">
                  <span className="font-semibold text-sm text-slate-100">{p.name}</span>
                  {p.isRecommended && (
                    <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 font-bold flex items-center space-x-1">
                      <Zap className="w-3 h-3 text-cyan-400" />
                      <span>RECOMMENDED</span>
                    </span>
                  )}
                  {p.isCurrent && (
                    <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 font-bold">
                      ACTIVE
                    </span>
                  )}
                </div>

                <div className="font-mono text-xs font-bold">
                  {p.medianLatencyMs < 900 ? (
                    <span className={p.medianLatencyMs < 15 ? 'text-emerald-400' : p.medianLatencyMs < 25 ? 'text-cyan-400' : 'text-amber-400'}>
                      {p.medianLatencyMs.toFixed(1)} ms
                    </span>
                  ) : (
                    <span className="text-slate-500">Timeout</span>
                  )}
                </div>
              </div>

              <p className="text-[11px] text-slate-400 mb-3">{p.description}</p>
            </div>

            <div>
              <div className="grid grid-cols-2 gap-2 py-2 border-t border-slate-800/80 text-[11px] font-mono">
                <div>
                  <span className="text-slate-500 block">Primary / Secondary:</span>
                  <span className="text-slate-300 font-medium">{p.primaryIp} {p.secondaryIp !== '—' && `/ ${p.secondaryIp}`}</span>
                </div>
                <div>
                  <span className="text-slate-500 block">Reliability / Range:</span>
                  <span className="text-slate-300 font-medium">
                    {p.reliabilityPercent.toFixed(0)}% ({p.minLatencyMs < 900 ? `${p.minLatencyMs.toFixed(0)}-${p.maxLatencyMs.toFixed(0)}ms` : 'N/A'})
                  </span>
                </div>
              </div>

              <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between">
                <span className="text-[10px] text-slate-500 font-mono">
                  {p.isCurrent ? 'Currently handling DNS queries' : `Target: ${report?.activeInterface || 'Wi-Fi'}`}
                </span>

                {!p.isCurrent && (
                  <button
                    onClick={() => handleApply(p)}
                    disabled={applying || testing}
                    className="px-3 py-1 bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-500/40 rounded-lg text-xs font-semibold transition-colors disabled:opacity-50"
                  >
                    Apply Resolver
                  </button>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Technical Disclaimer (Fulfilling Phase 3 requirement) */}
      <div className="p-3.5 rounded-xl bg-slate-900/60 border border-slate-800/80 flex items-start space-x-2.5 text-xs text-slate-400">
        <Info className="w-4 h-4 text-cyan-400 shrink-0 mt-0.5" />
        <div>
          <span className="font-semibold text-slate-200">Engineering Reality Check: </span>
          DNS optimization accelerates domain lookup times (converting domain names to IP addresses), which reduces web page and media playback startup lag (Time To First Byte / TTFB). <strong className="text-slate-200">DNS optimization does NOT increase ISP bandwidth or maximum file download speed.</strong>
        </div>
      </div>

      {/* Branded Confirmation Modal - Replaces browser default "tauri.localhost says" */}
      {confirmModal && confirmModal.isOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-in fade-in duration-150">
          <div className="glass-panel w-full max-w-md p-6 rounded-2xl border border-cyan-500/30 shadow-2xl bg-slate-900/95 space-y-4">
            <div className="flex items-center space-x-3">
              <div className="p-2.5 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400">
                <Shield className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-slate-100 flex items-center space-x-2">
                  <span>NetBoost says</span>
                  <span className="px-1.5 py-0.5 text-[9px] font-mono bg-cyan-500/20 text-cyan-300 rounded border border-cyan-500/30">
                    System Security
                  </span>
                </h3>
                <p className="text-[11px] text-slate-400">Network Adapter Configuration</p>
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-slate-950/80 border border-slate-800/80 text-xs text-slate-300 leading-relaxed whitespace-pre-line font-sans">
              {confirmModal.message}
            </div>

            <div className="flex items-center justify-end space-x-2 pt-2">
              <button
                onClick={() => setConfirmModal(null)}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl text-xs font-semibold transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={confirmModal.onConfirm}
                className="px-4 py-2 bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 rounded-xl text-xs font-bold transition-all shadow-lg shadow-cyan-500/20"
              >
                {confirmModal.confirmText}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
