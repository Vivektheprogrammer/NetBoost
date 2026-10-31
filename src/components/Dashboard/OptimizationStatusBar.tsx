import React from 'react';
import { ShieldCheck, Video, DownloadCloud, AlertTriangle, Cpu, Radio } from 'lucide-react';
import { OptimizationState } from '../../types/network';

interface OptimizationStatusBarProps {
  optState: OptimizationState | null;
}

export const OptimizationStatusBar: React.FC<OptimizationStatusBarProps> = ({ optState }) => {
  const isOptimized = optState?.enabled ?? true;
  const mode = optState?.mode ?? 'streaming';
  const congestion = optState?.congestionDetected ?? false;
  const streamSessions = optState?.activeStreamingSessions ?? 0;
  const activeDownloads = optState?.activeDownloads ?? 0;

  const modeDescriptions: Record<string, { title: string; desc: string; icon: any; color: string }> = {
    streaming: {
      title: 'Streaming Priority Active',
      desc: 'Protects video buffer integrity, suppresses queue delay, and limits background download bursts.',
      icon: Video,
      color: 'text-cyan-400 bg-cyan-500/10 border-cyan-500/30'
    },
    balanced: {
      title: 'Balanced Network Allocation',
      desc: 'Dynamically balances bandwidth between downloads, web browsing, and active streaming without starvation.',
      icon: Cpu,
      color: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/30'
    },
    download: {
      title: 'Download Acceleration Priority',
      desc: 'Maximizes segmented HTTP download concurrency while preserving basic UI and latency responsiveness.',
      icon: DownloadCloud,
      color: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/30'
    }
  };

  const currentModeInfo = modeDescriptions[mode] || modeDescriptions.streaming;
  const ModeIcon = currentModeInfo.icon;

  return (
    <div className="glass-panel rounded-xl p-3.5 border border-slate-800 flex flex-col md:flex-row items-center justify-between gap-3">
      {/* Mode & Policy */}
      <div className="flex items-center space-x-3 w-full md:w-auto">
        <div className={`p-2 rounded-lg border ${currentModeInfo.color} shrink-0`}>
          <ModeIcon className="w-4 h-4" />
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <h4 className="text-xs font-semibold text-slate-100">{currentModeInfo.title}</h4>
            {congestion ? (
              <span className="flex items-center space-x-1 text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-rose-500/10 text-rose-400 border border-rose-500/30">
                <AlertTriangle className="w-3 h-3" />
                <span>Congestion Mitigating</span>
              </span>
            ) : (
              <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                No Bufferbloat
              </span>
            )}
          </div>
          <p className="text-[11px] text-slate-400 leading-tight mt-0.5 max-w-xl">
            {optState?.policyDescription || currentModeInfo.desc}
          </p>
        </div>
      </div>

      {/* Stats counter badges */}
      <div className="flex items-center space-x-3 text-xs w-full md:w-auto justify-end">
        <div className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-slate-900/80 border border-slate-800 text-slate-300">
          <Video className="w-3.5 h-3.5 text-cyan-400" />
          <span className="text-[11px] text-slate-400">Streams:</span>
          <span className="font-mono font-semibold text-slate-100">{streamSessions}</span>
        </div>

        <div className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-slate-900/80 border border-slate-800 text-slate-300">
          <DownloadCloud className="w-3.5 h-3.5 text-indigo-400" />
          <span className="text-[11px] text-slate-400">Downloads:</span>
          <span className="font-mono font-semibold text-slate-100">{activeDownloads}</span>
        </div>
      </div>
    </div>
  );
};

