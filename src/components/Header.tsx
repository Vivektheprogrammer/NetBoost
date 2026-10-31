import React from 'react';
import { 
  Zap, 
  Activity, 
  Cpu, 
  DownloadCloud, 
  Gauge, 
  Globe, 
  Settings, 
  Power,
  ShieldCheck
} from 'lucide-react';
import { OperatingMode, OptimizationState, RealtimeMetrics } from '../types/network';

interface HeaderProps {
  currentTab: string;
  onTabChange: (tab: string) => void;
  metrics: RealtimeMetrics | null;
  optState: OptimizationState | null;
  onModeChange: (mode: OperatingMode) => void;
  onToggleOptimization: () => void;
  onOpenSettings: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  currentTab,
  onTabChange,
  metrics,
  optState,
  onModeChange,
  onToggleOptimization,
  onOpenSettings
}) => {
  const isOptimized = optState?.enabled ?? true;
  const currentMode = optState?.mode ?? 'streaming';

  return (
    <header className="h-16 border-b border-slate-800/80 bg-dark-900/90 backdrop-blur-md px-5 flex items-center justify-between z-20 shrink-0">
      {/* Brand */}
      <div className="flex items-center space-x-3">
        <div className="relative flex items-center justify-center w-9 h-9 rounded-xl bg-slate-950 border border-cyan-500/40 shadow-sm shadow-cyan-500/20 overflow-hidden p-1">
          <img src="/icon.png" alt="NetBoost Logo" className="w-full h-full object-contain" />
          {isOptimized && (
            <span className="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-emerald-400 border-2 border-slate-900 animate-pulse" />
          )}
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <h1 className="text-base font-semibold tracking-tight text-white">NetBoost</h1>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-cyan-950 text-cyan-400 border border-cyan-800/60 font-medium">
              v1.0
            </span>
          </div>
          <p className="text-[11px] text-slate-400 font-medium">Windows Network Intelligence</p>
        </div>
      </div>

      {/* Navigation tabs */}
      <nav className="flex items-center space-x-1 bg-slate-900/80 p-1 rounded-xl border border-slate-800">
        <button
          onClick={() => onTabChange('dashboard')}
          className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            currentTab === 'dashboard'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shadow-sm'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
          }`}
        >
          <Activity className="w-3.5 h-3.5" />
          <span>Dashboard</span>
        </button>

        <button
          onClick={() => onTabChange('applications')}
          className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            currentTab === 'applications'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shadow-sm'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
          }`}
        >
          <Cpu className="w-3.5 h-3.5" />
          <span>Applications</span>
        </button>

        <button
          onClick={() => onTabChange('downloads')}
          className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            currentTab === 'downloads'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shadow-sm'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
          }`}
        >
          <DownloadCloud className="w-3.5 h-3.5" />
          <span>Accelerator</span>
        </button>

        <button
          onClick={() => onTabChange('dns')}
          className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            currentTab === 'dns'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shadow-sm'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
          }`}
        >
          <Globe className="w-3.5 h-3.5" />
          <span>DNS Tuning</span>
        </button>

        <button
          onClick={() => onTabChange('benchmark')}
          className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            currentTab === 'benchmark'
              ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shadow-sm'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
          }`}
        >
          <Gauge className="w-3.5 h-3.5" />
          <span>Benchmark</span>
        </button>
      </nav>

      {/* Mode Switcher & Controls */}
      <div className="flex items-center space-x-3">
        {/* Operating Mode Selector */}
        <div className="flex items-center bg-dark-800/90 rounded-lg p-0.5 border border-slate-700/80 text-[11px] font-medium">
          <button
            onClick={() => onModeChange('streaming')}
            title="Streaming Priority: Protects video buffer and keeps queue latency low"
            className={`px-2.5 py-1 rounded transition-colors ${
              currentMode === 'streaming'
                ? 'bg-cyan-500/30 text-cyan-300 font-semibold shadow-sm'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Streaming
          </button>
          <button
            onClick={() => onModeChange('balanced')}
            title="Balanced: Fair dynamic allocation among all apps"
            className={`px-2.5 py-1 rounded transition-colors ${
              currentMode === 'balanced'
                ? 'bg-cyan-500/30 text-cyan-300 font-semibold shadow-sm'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Balanced
          </button>
          <button
            onClick={() => onModeChange('download')}
            title="Download Priority: Maximizes segmented download throughput"
            className={`px-2.5 py-1 rounded transition-colors ${
              currentMode === 'download'
                ? 'bg-cyan-500/30 text-cyan-300 font-semibold shadow-sm'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Download
          </button>
        </div>

        {/* Master Optimization Toggle */}
        <button
          onClick={onToggleOptimization}
          className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg border text-xs font-medium transition-all ${
            isOptimized
              ? 'bg-emerald-500/10 border-emerald-500/40 text-emerald-300 hover:bg-emerald-500/20'
              : 'bg-slate-800/60 border-slate-700 text-slate-400 hover:text-slate-200'
          }`}
          title={isOptimized ? 'Click to disable optimization' : 'Click to enable optimization'}
        >
          <Power className={`w-3.5 h-3.5 ${isOptimized ? 'text-emerald-400' : 'text-slate-500'}`} />
          <span>{isOptimized ? 'Optimizing' : 'Paused'}</span>
        </button>

        {/* Settings button */}
        <button
          onClick={onOpenSettings}
          className="p-2 text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 rounded-lg transition-colors border border-transparent hover:border-slate-700"
          title="NetBoost Settings"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};

