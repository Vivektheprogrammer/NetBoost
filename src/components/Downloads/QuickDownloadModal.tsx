import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Zap, Download, X, Layers, Globe, Copy, ShieldCheck } from 'lucide-react';
import { startDownload } from '../../services/tauriCommands';

interface InterceptedDownload {
  id?: number;
  url: string;
  filename?: string;
  referrer?: string;
  cookies?: string;
  source: string; // "browser_extension" | "clipboard"
}

interface QuickDownloadModalProps {
  onDownloadStarted?: () => void;
}

export const QuickDownloadModal: React.FC<QuickDownloadModalProps> = ({ onDownloadStarted }) => {
  const [activeDownload, setActiveDownload] = useState<InterceptedDownload | null>(null);
  const [concurrency, setConcurrency] = useState(6);
  const [isStarting, setIsStarting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    // Listen for intercepted downloads from local HTTP server or Windows clipboard
    const unlistenPromise = listen<InterceptedDownload>('netboost-download-detected', (event) => {
      setActiveDownload(event.payload);
      setErrorMsg(null);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Listen for Escape key to quickly yield back to browser
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && activeDownload) {
        handleDownloadInBrowser();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeDownload]);

  if (!activeDownload) return null;

  let rawFilename = activeDownload.filename || activeDownload.url.split('?')[0].split('/').pop() || 'download.bin';
  try {
    rawFilename = decodeURIComponent(rawFilename);
  } catch (e) {}
  const derivedFilename = rawFilename;

  // Option 1: Accelerate with NetBoost
  const handleStart = async () => {
    setIsStarting(true);
    setErrorMsg(null);
    try {
      // Notify extension to cancel Chrome's internal download
      if (activeDownload.id) {
        await invoke('resolve_intercepted_download', { id: activeDownload.id, accept: true });
      }

      await startDownload(
        activeDownload.url,
        derivedFilename,
        concurrency
      );
      setActiveDownload(null);
      if (onDownloadStarted) {
        onDownloadStarted();
      }
    } catch (err: any) {
      setErrorMsg(err?.toString() || 'Failed to start accelerated download.');
    } finally {
      setIsStarting(false);
    }
  };

  // Option 2: Decline and yield immediately back to Chrome / Edge native "Save As" screen!
  const handleDownloadInBrowser = async () => {
    try {
      if (activeDownload?.id) {
        await invoke('resolve_intercepted_download', { id: activeDownload.id, accept: false });
      } else {
        await invoke('hide_window');
      }
    } catch (err) {
      console.warn('Error resolving browser download:', err);
      // Fallback direct hide
      try {
        await invoke('hide_window');
      } catch (e) {}
    }
    setActiveDownload(null);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-in fade-in duration-200">
      <div className="glass-panel w-full max-w-lg p-6 rounded-2xl border border-cyan-500/40 shadow-2xl bg-slate-900/95 space-y-4 relative">
        {/* Close Button */}
        <button
          onClick={handleDownloadInBrowser}
          className="absolute top-4 right-4 p-1.5 text-slate-400 hover:text-slate-100 hover:bg-slate-800 rounded-lg transition-colors"
          title="Download in Browser"
        >
          <X className="w-4 h-4" />
        </button>

        {/* Header */}
        <div className="flex items-center space-x-3">
          <div className="w-12 h-12 rounded-xl bg-slate-950 border border-cyan-500/40 p-1.5 shadow-lg shadow-cyan-500/20 shrink-0 overflow-hidden">
            <img src="/icon.png" alt="NetBoost Logo" className="w-full h-full object-contain" />
          </div>
          <div>
            <div className="flex items-center space-x-2">
              <h3 className="text-base font-bold text-slate-100 tracking-tight">NetBoost says</h3>
              <span className="px-2 py-0.5 text-[10px] font-mono font-bold bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 rounded-full flex items-center space-x-1">
                <span>⚡ Download Interception</span>
              </span>
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              Choose whether to accelerate with NetBoost multi-worker engine or download normally in your browser.
            </p>
          </div>
        </div>

        {/* File & Source Card */}
        <div className="p-4 rounded-xl bg-slate-950/70 border border-slate-800/80 space-y-2">
          <div className="flex items-start justify-between">
            <div className="flex items-center space-x-2">
              <Download className="w-4 h-4 text-cyan-400 shrink-0" />
              <span className="text-sm font-semibold text-slate-100 font-mono break-all">
                {derivedFilename}
              </span>
            </div>
            <span className="text-[10px] font-mono px-2 py-0.5 bg-slate-800 text-slate-300 rounded border border-slate-700 shrink-0 ml-2">
              Multi-Channel
            </span>
          </div>

          <div className="text-[11px] text-slate-500 font-mono break-all line-clamp-2">
            {activeDownload.url}
          </div>
        </div>

        {/* Concurrency Channel Selector */}
        <div className="flex items-center justify-between p-3 rounded-xl bg-slate-900/60 border border-slate-800/80 text-xs text-slate-300">
          <div className="flex items-center space-x-2">
            <Layers className="w-4 h-4 text-slate-400" />
            <span>Parallel HTTP Range Channels:</span>
          </div>
          <div className="flex items-center space-x-1">
            {[4, 6, 8, 12].map((num) => (
              <button
                key={num}
                type="button"
                onClick={() => setConcurrency(num)}
                className={`px-2.5 py-1 rounded text-xs font-mono font-semibold transition-all ${
                  concurrency === num
                    ? 'bg-cyan-500 text-slate-950 font-bold shadow-md shadow-cyan-500/20'
                    : 'bg-slate-800 text-slate-400 hover:text-slate-200'
                }`}
              >
                {num}x
              </button>
            ))}
          </div>
        </div>

        {errorMsg && (
          <div className="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/20 text-rose-400 text-xs font-mono">
            {errorMsg}
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex items-center justify-between pt-2">
          <button
            type="button"
            onClick={handleDownloadInBrowser}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-slate-100 rounded-xl text-xs font-semibold transition-colors"
          >
            Download Normally in Browser
          </button>
          <button
            type="button"
            disabled={isStarting}
            onClick={handleStart}
            className="px-5 py-2.5 bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-slate-950 rounded-xl text-xs font-bold transition-all shadow-lg shadow-cyan-500/25 flex items-center space-x-2 disabled:opacity-50"
          >
            <Zap className="w-4 h-4 fill-current" />
            <span>{isStarting ? 'Connecting...' : `Accelerate (${concurrency}x Channels)`}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
