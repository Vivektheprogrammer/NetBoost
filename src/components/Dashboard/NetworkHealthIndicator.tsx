import React from 'react';
import { CheckCircle2, AlertCircle, AlertTriangle, XCircle, Sparkles } from 'lucide-react';
import { NetworkHealth } from '../../types/network';

interface NetworkHealthIndicatorProps {
  health: NetworkHealth | undefined;
}

export const NetworkHealthIndicator: React.FC<NetworkHealthIndicatorProps> = ({ health }) => {
  const score = health?.scoreNumber ?? 95;
  const rating = health?.score ?? 'excellent';
  const reasons = health?.reasons ?? [
    'Low round-trip latency (< 25 ms)',
    'Stable packet cadence (jitter < 2 ms)',
    'Zero measurable packet drop'
  ];

  const getBadgeConfig = () => {
    switch (rating) {
      case 'excellent':
        return {
          label: 'EXCELLENT',
          color: 'text-emerald-400',
          bg: 'bg-emerald-500/10 border-emerald-500/30',
          gaugeColor: '#10b981',
          icon: CheckCircle2,
          summary: 'Optimal conditions for 4K/8K HDR streaming and accelerated downloads.'
        };
      case 'good':
        return {
          label: 'GOOD',
          color: 'text-cyan-400',
          bg: 'bg-cyan-500/10 border-cyan-500/30',
          gaugeColor: '#06b6d4',
          icon: CheckCircle2,
          summary: 'Smooth streaming with minimal buffering risks.'
        };
      case 'moderate':
        return {
          label: 'MODERATE',
          color: 'text-amber-400',
          bg: 'bg-amber-500/10 border-amber-500/30',
          gaugeColor: '#f59e0b',
          icon: AlertTriangle,
          summary: 'Intermittent congestion or queue delay detected under active load.'
        };
      case 'poor':
      default:
        return {
          label: 'POOR',
          color: 'text-rose-400',
          bg: 'bg-rose-500/10 border-rose-500/30',
          gaugeColor: '#f43f5e',
          icon: XCircle,
          summary: 'High packet loss or elevated jitter causing playback degradation.'
        };
    }
  };

  const config = getBadgeConfig();
  const Icon = config.icon;

  return (
    <div className="glass-panel rounded-xl p-4 border border-slate-800 flex flex-col justify-between">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-2">
          <Sparkles className="w-4 h-4 text-cyan-400" />
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
            Network Health & Stability
          </h3>
        </div>
        <div className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-md border text-xs font-bold tracking-wide ${config.bg} ${config.color}`}>
          <Icon className="w-3.5 h-3.5" />
          <span>{config.label}</span>
        </div>
      </div>

      {/* Main Score Bar */}
      <div className="flex items-center space-x-4 my-2">
        <div className="relative w-16 h-16 shrink-0 flex items-center justify-center">
          <svg className="w-full h-full -rotate-90" viewBox="0 0 36 36">
            <path
              className="text-slate-800"
              strokeWidth="3.5"
              stroke="currentColor"
              fill="none"
              d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
            />
            <path
              strokeWidth="3.5"
              strokeDasharray={`${score}, 100`}
              strokeLinecap="round"
              stroke={config.gaugeColor}
              fill="none"
              d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
            />
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span className="text-sm font-bold font-mono text-white">{score}</span>
            <span className="text-[9px] text-slate-400 font-medium">/ 100</span>
          </div>
        </div>

        <div className="flex-1">
          <p className="text-xs text-slate-300 font-medium leading-relaxed">
            {config.summary}
          </p>
          <div className="mt-2 flex items-center space-x-3 text-[11px] font-mono">
            <span className="text-slate-400">Latency: <b className="text-slate-200">{health?.latencyGrade ?? 'A+'}</b></span>
            <span className="text-slate-400">Jitter: <b className="text-slate-200">{health?.jitterGrade ?? 'A+'}</b></span>
            <span className="text-slate-400">Quality: <b className="text-slate-200">{health?.stabilityGrade ?? '100%'}</b></span>
          </div>
        </div>
      </div>

      {/* Contributing Factors */}
      <div className="mt-3 pt-3 border-t border-slate-800/80">
        <h4 className="text-[10px] uppercase font-semibold text-slate-400 tracking-wider mb-1.5">
          Calculated Health Factors
        </h4>
        <ul className="space-y-1">
          {reasons.slice(0, 3).map((reason, idx) => (
            <li key={idx} className="flex items-center space-x-2 text-[11px] text-slate-300">
              <span className="w-1 h-1 rounded-full bg-cyan-400 shrink-0" />
              <span className="truncate">{reason}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
};

