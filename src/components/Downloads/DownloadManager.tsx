import React, { useState, useEffect } from 'react';
import { 
  Download, 
  Plus, 
  Play, 
  Pause, 
  X, 
  FolderOpen, 
  FileText, 
  Layers, 
  AlertCircle, 
  RotateCcw, 
  CheckCircle2, 
  Clock, 
  Zap, 
  ShieldCheck,
  Globe 
} from 'lucide-react';
import { DownloadTask } from '../../types/network';
import { 
  getDownloads, 
  startDownload, 
  pauseDownload, 
  resumeDownload, 
  cancelDownload, 
  retryDownload, 
  openDownloadedFile, 
  openDownloadFolder,
  openExtensionFolder 
} from '../../services/tauriCommands';
import { formatSpeed, formatBytes, formatEta } from '../../utils/formatters';

export const DownloadManager: React.FC = () => {
  const [downloadUrl, setDownloadUrl] = useState('');
  const [customFilename, setCustomFilename] = useState('');
  const [concurrency, setConcurrency] = useState(4);
  const [downloads, setDownloads] = useState<DownloadTask[]>([]);
  const [isStarting, setIsStarting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Poll real download tasks from Rust core engine
  const fetchTasks = async () => {
    try {
      const list = await getDownloads();
      setDownloads(list);
    } catch (err) {
      console.error('Error fetching download queue:', err);
    }
  };

  useEffect(() => {
    fetchTasks();
    const interval = setInterval(fetchTasks, 750);
    return () => clearInterval(interval);
  }, []);

  const handleStart = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!downloadUrl.trim()) return;

    setErrorMsg(null);
    setIsStarting(true);

    try {
      await startDownload(
        downloadUrl.trim(),
        customFilename.trim() ? customFilename.trim() : undefined,
        concurrency
      );
      setDownloadUrl('');
      setCustomFilename('');
      await fetchTasks();
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : 'Failed to probe or start download.');
    } finally {
      setIsStarting(false);
    }
  };

  const handlePause = async (id: string) => {
    await pauseDownload(id);
    await fetchTasks();
  };

  const handleResume = async (id: string) => {
    await resumeDownload(id);
    await fetchTasks();
  };

  const handleCancel = async (id: string) => {
    await cancelDownload(id);
    await fetchTasks();
  };

  const handleRetry = async (id: string) => {
    await retryDownload(id);
    await fetchTasks();
  };

  const handleOpenFile = async (path: string) => {
    await openDownloadedFile(path);
  };

  const handleOpenFolder = async (path: string) => {
    await openDownloadFolder(path);
  };

  return (
    <div className="space-y-4">
      {/* Add New Download Form */}
      <form onSubmit={handleStart} className="glass-panel p-4 rounded-xl border border-slate-800 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Download className="w-4 h-4 text-cyan-400" />
            <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
              Parallel Segmented Download Accelerator
            </h3>
          </div>
          <span className="text-[11px] text-slate-400 font-mono flex items-center space-x-1.5">
            <Zap className="w-3.5 h-3.5 text-cyan-400" />
            <span>Multi-channel HTTP Byte-Range Engine</span>
          </span>
        </div>

        {errorMsg && (
          <div className="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/30 flex items-center space-x-2 text-xs text-rose-300">
            <AlertCircle className="w-4 h-4 shrink-0 text-rose-400" />
            <span>{errorMsg}</span>
          </div>
        )}

        <div className="flex flex-col sm:flex-row items-center gap-2">
          <input
            type="url"
            value={downloadUrl}
            onChange={(e) => setDownloadUrl(e.target.value)}
            placeholder="Direct download URL (e.g. https://.../file.iso)"
            required
            className="flex-1 w-full px-3 py-2 bg-dark-900 border border-slate-700/80 rounded-lg text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500 font-mono"
          />

          <input
            type="text"
            value={customFilename}
            onChange={(e) => setCustomFilename(e.target.value)}
            placeholder="Custom filename (optional)"
            className="w-full sm:w-48 px-3 py-2 bg-dark-900 border border-slate-700/80 rounded-lg text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-cyan-500"
          />

          <div className="flex items-center space-x-2 w-full sm:w-auto">
            <div className="flex items-center space-x-1 px-2.5 py-1.5 bg-dark-900 border border-slate-700/80 rounded-lg text-xs text-slate-300">
              <Layers className="w-3.5 h-3.5 text-cyan-400" />
              <span className="text-slate-400">Workers:</span>
              <select
                value={concurrency}
                onChange={(e) => setConcurrency(Number(e.target.value))}
                className="bg-transparent text-cyan-300 font-semibold focus:outline-none cursor-pointer"
              >
                <option value={1} className="bg-slate-900 text-slate-200">1 (Single Stream)</option>
                <option value={2} className="bg-slate-900 text-slate-200">2 Channels</option>
                <option value={4} className="bg-slate-900 text-slate-200">4 Channels</option>
                <option value={8} className="bg-slate-900 text-slate-200">8 Channels</option>
              </select>
            </div>

            <button
              type="submit"
              disabled={isStarting}
              className="px-4 py-2 bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-500/40 font-medium text-xs rounded-lg flex items-center space-x-1.5 transition-colors shrink-0 shadow-sm disabled:opacity-50"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>{isStarting ? 'Probing URL...' : 'Start Download'}</span>
            </button>
          </div>
        </div>
      </form>

      {/* Browser Integration & Clipboard Banner */}
      <div className="p-3.5 rounded-xl bg-slate-900/60 border border-cyan-500/20 flex flex-col sm:flex-row sm:items-center justify-between gap-2.5 text-xs text-slate-400">
        <div className="flex items-center space-x-2.5">
          <div className="p-1.5 rounded-lg bg-cyan-500/10 border border-cyan-500/20 text-cyan-400 shrink-0">
            <Zap className="w-4 h-4" />
          </div>
          <div>
            <div className="font-semibold text-slate-200 flex items-center space-x-2">
              <span>Automatic Download Interception Active</span>
              <span className="px-1.5 py-0.2 text-[9px] font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded">
                Live Sniffer
              </span>
            </div>
            <div className="text-[11px] text-slate-400 mt-0.5">
              NetBoost automatically pops up when download buttons are clicked in Chrome/Edge, or when download links are copied to your clipboard.
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={() => openExtensionFolder()}
          className="px-3 py-1.5 bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-500/30 rounded-lg text-xs font-semibold shrink-0 transition-colors cursor-pointer"
        >
          Open Extension Folder
        </button>
      </div>

      {/* Downloads List */}
      <div className="space-y-3">
        {downloads.length === 0 ? (
          <div className="glass-panel p-8 rounded-xl border border-slate-800 text-center text-slate-500 text-xs">
            No downloads currently active or in history. Paste a direct HTTP/HTTPS link above to begin.
          </div>
        ) : (
          downloads.map((item) => {
            const percent = item.totalBytes > 0
              ? Math.min(100, Math.round((item.downloadedBytes / item.totalBytes) * 100))
              : 0;

            const isDone = item.status === 'completed';
            const isPaused = item.status === 'paused';
            const isFailed = item.status === 'failed';

            return (
              <div key={item.id} className="glass-panel p-4 rounded-xl border border-slate-800 space-y-3">
                {/* Header row */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3 overflow-hidden">
                    <div className={`p-2 rounded-lg border ${
                      isDone 
                        ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400' 
                        : isFailed
                        ? 'bg-rose-500/10 border-rose-500/30 text-rose-400'
                        : 'bg-cyan-500/10 border-cyan-500/30 text-cyan-400'
                    }`}>
                      {isDone ? <CheckCircle2 className="w-4 h-4" /> : <Download className="w-4 h-4" />}
                    </div>
                    <div className="truncate">
                      <div className="flex items-center space-x-2">
                        <h4 className="text-xs font-semibold text-slate-100 truncate">{item.filename}</h4>
                        <span className={`px-2 py-0.2 rounded text-[10px] font-mono font-semibold uppercase ${
                          isDone 
                            ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' 
                            : isPaused
                            ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                            : isFailed
                            ? 'bg-rose-500/10 text-rose-400 border border-rose-500/20'
                            : 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 animate-pulse'
                        }`}>
                          {item.status}
                        </span>
                      </div>
                      <span className="text-[10px] text-slate-500 truncate block font-mono max-w-lg mt-0.5">
                        {item.url}
                      </span>
                      {item.errorMessage && (
                        <span className="text-[10px] text-rose-400 font-mono block mt-1 bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 rounded w-fit">
                          {item.errorMessage}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Actions & Live Telemetry */}
                  <div className="flex items-center space-x-2 shrink-0">
                    {!isDone && (
                      <div className="text-right font-mono text-xs mr-2">
                        <div className="font-bold text-cyan-300">{formatSpeed(item.speedBps / 8)}</div>
                        <div className="text-[10px] text-slate-500">
                          {item.etaSeconds > 0 ? `ETA: ${formatEta(item.etaSeconds)}` : '—'}
                        </div>
                      </div>
                    )}

                    {/* Pause / Resume */}
                    {!isDone && !isFailed && (
                      <button
                        onClick={() => isPaused ? handleResume(item.id) : handlePause(item.id)}
                        className="p-1.5 bg-slate-800 hover:bg-slate-700 rounded text-slate-300 transition-colors"
                        title={isPaused ? 'Resume Download' : 'Pause Download'}
                      >
                        {isPaused ? <Play className="w-3.5 h-3.5" /> : <Pause className="w-3.5 h-3.5" />}
                      </button>
                    )}

                    {/* Retry if failed */}
                    {isFailed && (
                      <button
                        onClick={() => handleRetry(item.id)}
                        className="p-1.5 bg-slate-800 hover:bg-slate-700 rounded text-cyan-400 transition-colors"
                        title="Retry Download"
                      >
                        <RotateCcw className="w-3.5 h-3.5" />
                      </button>
                    )}

                    {/* Open file / Open folder on completion */}
                    {isDone && (
                      <>
                        <button
                          onClick={() => handleOpenFile(item.destinationPath)}
                          className="px-2.5 py-1 bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 border border-cyan-500/40 rounded text-[11px] font-medium flex items-center space-x-1"
                          title="Open completed file"
                        >
                          <FileText className="w-3 h-3" />
                          <span>Open File</span>
                        </button>
                        <button
                          onClick={() => handleOpenFolder(item.destinationPath)}
                          className="p-1.5 bg-slate-800 hover:bg-slate-700 rounded text-slate-300"
                          title="Open download folder"
                        >
                          <FolderOpen className="w-3.5 h-3.5" />
                        </button>
                      </>
                    )}

                    {/* Cancel / Remove */}
                    <button
                      onClick={() => handleCancel(item.id)}
                      className="p-1.5 bg-slate-800 hover:bg-slate-700 rounded text-slate-400 hover:text-rose-400 transition-colors"
                      title="Cancel and remove file"
                    >
                      <X className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                {/* Progress bar */}
                <div>
                  <div className="flex items-center justify-between text-[11px] text-slate-400 mb-1 font-mono">
                    <span>
                      {formatBytes(item.downloadedBytes)} / {item.totalBytes > 0 ? formatBytes(item.totalBytes) : 'Stream'}
                    </span>
                    <span className="font-semibold text-slate-200">{percent}%</span>
                  </div>
                  <div className="w-full h-2 bg-slate-800 rounded-full overflow-hidden">
                    <div
                      className={`h-full transition-all duration-300 ${
                        isDone 
                          ? 'bg-emerald-400' 
                          : isFailed
                          ? 'bg-rose-500'
                          : 'bg-gradient-to-r from-cyan-500 to-blue-500'
                      }`}
                      style={{ width: `${percent}%` }}
                    />
                  </div>
                </div>

                {/* Multi-worker parallel segments display */}
                {item.segments && item.segments.length > 0 && (
                  <div className="pt-2 border-t border-slate-800/80">
                    <div className="flex items-center justify-between text-[10px] text-slate-400 mb-1.5">
                      <span className="uppercase font-semibold tracking-wider">
                        Parallel Range Workers ({item.segments.length} Channels)
                      </span>
                      {item.checksumSha256 && (
                        <span className="font-mono text-emerald-400 truncate max-w-xs flex items-center space-x-1">
                          <ShieldCheck className="w-3 h-3 shrink-0" />
                          <span>SHA-256: {item.checksumSha256.substring(0, 16)}...</span>
                        </span>
                      )}
                    </div>

                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                      {item.segments.map((seg) => {
                        const segTotal = seg.endByte > seg.startByte ? seg.endByte - seg.startByte + 1 : 1;
                        const segPct = Math.min(100, Math.round((seg.downloadedBytes / segTotal) * 100));

                        return (
                          <div key={seg.index} className="p-2 bg-slate-900/60 rounded border border-slate-800/80 text-[10px] font-mono">
                            <div className="flex items-center justify-between text-slate-400 mb-1">
                              <span>Worker #{seg.index + 1}</span>
                              <span className="text-slate-300 font-bold">{segPct}%</span>
                            </div>
                            <div className="w-full h-1 bg-slate-800 rounded-full overflow-hidden mb-1">
                              <div
                                className="h-full bg-cyan-400 transition-all duration-200"
                                style={{ width: `${segPct}%` }}
                              />
                            </div>
                            <div className="flex items-center justify-between text-[9px] text-slate-500">
                              <span>{formatBytes(seg.downloadedBytes)}</span>
                              <span>{seg.speedBps > 0 ? formatSpeed(seg.speedBps / 8) : '—'}</span>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
