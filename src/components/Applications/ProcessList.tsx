import React, { useState, useEffect } from 'react';
import { 
  Search, 
  Tv, 
  RefreshCw, 
  ChevronDown, 
  ChevronRight, 
  Info, 
  Radio,
  Settings,
  Plus,
  Trash2,
  ShieldCheck
} from 'lucide-react';
import { ProcessNetworkReport, StreamingConfig } from '../../types/network';
import { formatSpeed, formatBytes } from '../../utils/formatters';
import { 
  getStreamingConfig, 
  addUserStreamingApp, 
  removeUserStreamingApp 
} from '../../services/tauriCommands';

interface ProcessListProps {
  processes: ProcessNetworkReport[];
  onRefresh: () => void;
}

export const ProcessList: React.FC<ProcessListProps> = ({ processes, onRefresh }) => {
  const [filterQuery, setFilterQuery] = useState('');
  const [onlyStreaming, setOnlyStreaming] = useState(false);
  const [expandedPid, setExpandedPid] = useState<number | null>(null);
  const [showConfig, setShowConfig] = useState(false);
  const [streamingConfig, setStreamingConfig] = useState<StreamingConfig | null>(null);
  const [newAppName, setNewAppName] = useState('');

  const loadConfig = async () => {
    try {
      const cfg = await getStreamingConfig();
      setStreamingConfig(cfg);
    } catch (e) {
      console.warn('Failed to load streaming config:', e);
    }
  };

  useEffect(() => {
    loadConfig();
  }, []);

  const handleAddApp = async () => {
    const clean = newAppName.trim();
    if (!clean) return;
    try {
      const updated = await addUserStreamingApp(clean);
      setStreamingConfig(updated);
      setNewAppName('');
      onRefresh();
    } catch (e) {
      console.error(e);
    }
  };

  const handleRemoveApp = async (app: string) => {
    try {
      const updated = await removeUserStreamingApp(app);
      setStreamingConfig(updated);
      onRefresh();
    } catch (e) {
      console.error(e);
    }
  };

  const filtered = processes.filter((p) => {
    const matchesSearch =
      p.name.toLowerCase().includes(filterQuery.toLowerCase()) ||
      p.pid.toString().includes(filterQuery) ||
      p.path.toLowerCase().includes(filterQuery.toLowerCase());
    const matchesStream = onlyStreaming ? p.isStreaming : true;
    return matchesSearch && matchesStream;
  });

  const toggleExpand = (pid: number) => {
    setExpandedPid(expandedPid === pid ? null : pid);
  };

  return (
    <div className="space-y-4">
      {/* Search & Filter Toolbar */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-3 glass-panel p-3 rounded-xl border border-slate-800">
        <div className="relative w-full sm:w-80">
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-2.5" />
          <input
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Search by executable, PID or path..."
            className="w-full pl-9 pr-3 py-1.5 bg-dark-900/80 border border-slate-700/80 rounded-lg text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 font-mono"
          />
        </div>

        <div className="flex items-center space-x-2 w-full sm:w-auto justify-end">
          <button
            onClick={() => setShowConfig(!showConfig)}
            className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors ${
              showConfig
                ? 'bg-purple-500/20 border-purple-500/40 text-purple-300'
                : 'bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200'
            }`}
          >
            <Settings className="w-3.5 h-3.5" />
            <span>Streaming Config</span>
          </button>

          <button
            onClick={() => setOnlyStreaming(!onlyStreaming)}
            className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors ${
              onlyStreaming
                ? 'bg-cyan-500/20 border-cyan-500/40 text-cyan-300'
                : 'bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200'
            }`}
          >
            <Tv className="w-3.5 h-3.5" />
            <span>Streaming Apps Only</span>
          </button>

          <button
            onClick={onRefresh}
            className="p-1.5 bg-slate-900 border border-slate-800 rounded-lg text-slate-400 hover:text-slate-200 hover:border-slate-700 transition-colors"
            title="Refresh process socket table"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Streaming Configuration & Privacy Disclosure Drawer */}
      {showConfig && streamingConfig && (
        <div className="glass-panel p-4 rounded-xl border border-purple-500/30 bg-purple-950/10 space-y-3">
          <div className="flex items-start justify-between">
            <div>
              <h4 className="text-xs font-semibold text-purple-300 flex items-center space-x-2">
                <ShieldCheck className="w-4 h-4 text-purple-400" />
                <span>Configured Streaming Heuristics & Privacy Safeguards</span>
              </h4>
              <p className="text-[11px] text-slate-400 mt-1 max-w-2xl">
                NetBoost identifies streaming using process binaries, HTTPS socket count, DASH/HLS transfer cadence, and CDN domain matching. NetBoost does <strong className="text-slate-200">never decrypt HTTPS payloads</strong> or inspect video content.
              </p>
            </div>
          </div>

          {/* Add custom process name */}
          <div className="flex items-center space-x-2 max-w-md pt-1">
            <input
              type="text"
              placeholder="e.g. obs64.exe, twitch.exe, potplayer.exe"
              value={newAppName}
              onChange={(e) => setNewAppName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleAddApp()}
              className="flex-1 bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-slate-200 placeholder-slate-600 focus:outline-none focus:border-purple-400 font-mono"
            />
            <button
              onClick={handleAddApp}
              className="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 text-white rounded-lg text-xs font-semibold flex items-center space-x-1 transition-colors shrink-0"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Add App</span>
            </button>
          </div>

          {/* User configured custom apps */}
          {streamingConfig.userConfiguredApps.length > 0 && (
            <div className="space-y-1 pt-1">
              <span className="text-[10px] font-mono uppercase text-purple-300">User Custom Streaming Apps:</span>
              <div className="flex flex-wrap gap-1.5">
                {streamingConfig.userConfiguredApps.map((app) => (
                  <span
                    key={app}
                    className="inline-flex items-center space-x-1.5 px-2 py-0.5 rounded bg-purple-500/20 border border-purple-500/40 text-purple-200 text-xs font-mono"
                  >
                    <span>{app}</span>
                    <button
                      onClick={() => handleRemoveApp(app)}
                      className="text-purple-400 hover:text-rose-400"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Default streaming apps list */}
          <div className="space-y-1 pt-1">
            <span className="text-[10px] font-mono uppercase text-slate-400">Built-in Monitored Applications & Browsers:</span>
            <div className="flex flex-wrap gap-1 max-h-20 overflow-y-auto">
              {streamingConfig.defaultApps.map((app) => (
                <span
                  key={app}
                  className="px-2 py-0.5 rounded bg-slate-900/80 border border-slate-800 text-slate-400 text-[10px] font-mono"
                >
                  {app}
                </span>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Technical Attribution Disclosure (Fulfilling Part A requirement) */}
      <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800/80 flex items-start space-x-2.5 text-xs text-slate-400">
        <Info className="w-4 h-4 text-cyan-400 shrink-0 mt-0.5" />
        <div>
          <span className="font-semibold text-slate-200">Hardware & Process Attribution Disclosure: </span>
          Active TCP connections and owner PIDs are mapped directly via Windows <code className="font-mono text-cyan-300">GetExtendedTcpTable</code>. Data rates represent <strong className="text-slate-200">Combined Process I/O</strong> (disk operations plus socket traffic) measured via native <code className="font-mono text-cyan-300">GetProcessIoCounters</code>. NetBoost does not divide adapter bandwidth across applications, and never decrypts private HTTPS traffic.
        </div>
      </div>

      {/* Main Table */}
      <div className="glass-panel rounded-xl border border-slate-800 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse text-xs">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80 text-slate-400 font-semibold uppercase text-[10px] tracking-wider">
                <th className="py-2.5 px-3 w-8"></th>
                <th className="py-2.5 px-3">Application</th>
                <th className="py-2.5 px-3">PID</th>
                <th className="py-2.5 px-3">Active Sockets</th>
                <th className="py-2.5 px-3" title="Total process read operations (socket downloads + disk reads) measured via Win32 GetProcessIoCounters">
                  Combined Process Read
                </th>
                <th className="py-2.5 px-3" title="Total process write operations (socket uploads + disk writes) measured via Win32 GetProcessIoCounters">
                  Combined Process Write
                </th>
                <th className="py-2.5 px-3" title="Heuristic streaming assessment with uncertainty/confidence level (High, Medium, Low, Unknown)">
                  Streaming Classification
                </th>
                <th className="py-2.5 px-4 text-right">Lifetime Total I/O</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {filtered.length === 0 ? (
                <tr>
                  <td colSpan={8} className="py-8 text-center text-slate-500">
                    No active network-consuming applications matching your filter.
                  </td>
                </tr>
              ) : (
                filtered.map((proc) => {
                  const isExpanded = expandedPid === proc.pid;

                  return (
                    <React.Fragment key={proc.pid}>
                      <tr 
                        onClick={() => toggleExpand(proc.pid)}
                        className={`hover:bg-slate-800/40 transition-colors cursor-pointer ${
                          isExpanded ? 'bg-slate-800/30' : ''
                        }`}
                      >
                        {/* Expand Icon */}
                        <td className="py-2.5 px-3 text-slate-500">
                          {isExpanded ? (
                            <ChevronDown className="w-3.5 h-3.5 text-cyan-400" />
                          ) : (
                            <ChevronRight className="w-3.5 h-3.5 text-slate-600" />
                          )}
                        </td>

                        {/* Name & Path */}
                        <td className="py-2.5 px-3">
                          <div className="font-medium text-slate-200 flex items-center space-x-2">
                            <span className="w-2 h-2 rounded-full bg-slate-500 shrink-0" />
                            <span className="truncate max-w-xs">{proc.name}</span>
                          </div>
                          {proc.path && (
                            <span className="text-[10px] text-slate-500 truncate block font-mono max-w-sm mt-0.5">
                              {proc.path}
                            </span>
                          )}
                        </td>

                        {/* PID */}
                        <td className="py-2.5 px-3 font-mono text-slate-400">
                          {proc.pid}
                        </td>

                        {/* Sockets */}
                        <td className="py-2.5 px-3 font-mono text-slate-300">
                          <span className="px-2 py-0.5 rounded bg-slate-800/80 border border-slate-700/60 font-semibold text-cyan-400">
                            {proc.activeConnections}
                          </span>
                        </td>

                        {/* Read Rate */}
                        <td className="py-2.5 px-3 font-mono font-semibold text-cyan-300">
                          {formatSpeed(proc.ioReadRateBps / 8)}
                        </td>

                        {/* Write Rate */}
                        <td className="py-2.5 px-3 font-mono font-semibold text-indigo-300">
                          {formatSpeed(proc.ioWriteRateBps / 8)}
                        </td>

                        {/* Streaming Detection Badge */}
                        <td className="py-2.5 px-3">
                          {proc.isStreaming ? (
                            <div className="flex flex-col space-y-0.5">
                              <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-semibold bg-cyan-500/10 text-cyan-300 border border-cyan-500/30 w-fit">
                                <Tv className="w-3 h-3 text-cyan-400" />
                                <span className="capitalize">Streaming ({proc.streamingConfidence})</span>
                              </span>
                              {proc.streamingReason && (
                                <span className="text-[9px] text-slate-400 truncate max-w-xs">
                                  {proc.streamingReason}
                                </span>
                              )}
                            </div>
                          ) : (
                            <span className="text-slate-600 text-[11px]">—</span>
                          )}
                        </td>

                        {/* Lifetime I/O */}
                        <td className="py-2.5 px-4 text-right font-mono text-slate-300">
                          {formatBytes(proc.ioReadTransferBytes + proc.ioWriteTransferBytes)}
                        </td>
                      </tr>

                      {/* Expanded Sockets Subtable */}
                      {isExpanded && (
                        <tr className="bg-slate-950/60">
                          <td colSpan={8} className="py-3 px-6 border-b border-slate-800">
                            <div className="space-y-2">
                              <div className="flex items-center justify-between text-[11px] font-mono text-slate-400">
                                <span className="font-semibold text-slate-300 flex items-center space-x-1.5">
                                  <Radio className="w-3.5 h-3.5 text-cyan-400" />
                                  <span>Active Network Endpoints ({proc.connections.length} Sockets)</span>
                                </span>
                                <span className="text-[10px] text-slate-500">
                                  Query method: GetExtendedTcpTable (TCP_TABLE_OWNER_PID_ALL)
                                </span>
                              </div>

                              <div className="rounded-lg border border-slate-800/80 overflow-hidden bg-dark-900/90">
                                <table className="w-full text-left border-collapse text-[11px] font-mono">
                                  <thead>
                                    <tr className="bg-slate-900/60 text-slate-400 text-[10px] uppercase border-b border-slate-800">
                                      <th className="py-1.5 px-3">Protocol</th>
                                      <th className="py-1.5 px-3">Local Endpoint</th>
                                      <th className="py-1.5 px-3">Remote Endpoint</th>
                                      <th className="py-1.5 px-3">TCP State</th>
                                      <th className="py-1.5 px-3 text-right">Transition Event</th>
                                    </tr>
                                  </thead>
                                  <tbody className="divide-y divide-slate-800/40">
                                    {proc.connections.map((c, i) => (
                                      <tr key={i} className="hover:bg-slate-800/30">
                                        <td className="py-1.5 px-3 text-slate-400">{c.ip_version}</td>
                                        <td className="py-1.5 px-3 text-slate-300">{c.local_addr}</td>
                                        <td className="py-1.5 px-3 text-cyan-300">{c.remote_addr}</td>
                                        <td className="py-1.5 px-3">
                                          <span className={`px-1.5 py-0.5 rounded text-[10px] ${
                                            c.state === 'ESTABLISHED' 
                                              ? 'bg-emerald-500/10 text-emerald-400 font-semibold'
                                              : 'bg-slate-800 text-slate-400'
                                          }`}>
                                            {c.state}
                                          </span>
                                        </td>
                                        <td className="py-1.5 px-3 text-right">
                                          {c.is_new ? (
                                            <span className="px-1.5 py-0.5 rounded text-[10px] bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 animate-pulse font-semibold">
                                              NEW (Established)
                                            </span>
                                          ) : (
                                            <span className="text-slate-600 text-[10px]">Active</span>
                                          )}
                                        </td>
                                      </tr>
                                    ))}
                                  </tbody>
                                </table>
                              </div>
                            </div>
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
