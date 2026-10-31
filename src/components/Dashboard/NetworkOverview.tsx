import React from 'react';
import { 
  ArrowDownCircle, 
  ArrowUpCircle, 
  Clock, 
  Activity, 
  ShieldAlert, 
  Database 
} from 'lucide-react';
import { RealtimeMetrics } from '../../types/network';
import { formatSpeed, formatBytes, formatLatency, formatJitter } from '../../utils/formatters';

interface NetworkOverviewProps {
  metrics: RealtimeMetrics | null;
}

export const NetworkOverview: React.FC<NetworkOverviewProps> = ({ metrics }) => {
  const downSpeed = metrics?.downloadSpeedBps ?? 0;
  const upSpeed = metrics?.uploadSpeedBps ?? 0;
  const totalDown = metrics?.totalBytesReceived ?? 0;
  const totalUp = metrics?.totalBytesSent ?? 0;
  const latency = metrics?.latencyMs ?? 0;
  const jitter = metrics?.jitterMs ?? 0;
  const packetLoss = metrics?.packetLossPercent ?? 0;

  // Latency status color
  const latencyColor = latency <= 25 ? 'text-emerald-400' : latency <= 60 ? 'text-amber-400' : 'text-rose-400';
  const jitterColor = jitter <= 3 ? 'text-emerald-400' : jitter <= 10 ? 'text-amber-400' : 'text-rose-400';
  const lossColor = packetLoss === 0 ? 'text-emerald-400' : packetLoss <= 2 ? 'text-amber-400' : 'text-rose-400';

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-3">
      {/* Download Speed */}
      <div className="glass-panel rounded-xl p-3.5 border border-slate-800 relative overflow-hidden group">
        <div className="absolute top-0 right-0 w-24 h-24 bg-cyan-500/5 rounded-full blur-xl group-hover:bg-cyan-500/10 transition-all" />
        <div className="flex items-center justify-between text-slate-400 text-xs mb-1">
          <span className="font-medium">Download Rate</span>
          <ArrowDownCircle className="w-4 h-4 text-cyan-400" />
        </div>
        <div className="text-xl font-bold font-mono tracking-tight text-cyan-300">
          {formatSpeed(downSpeed)}
        </div>
        <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400">
          <span>Total In:</span>
          <span className="font-mono text-slate-300">{formatBytes(totalDown)}</span>
        </div>
      </div>

      {/* Upload Speed */}
      <div className="glass-panel rounded-xl p-3.5 border border-slate-800 relative overflow-hidden group">
        <div className="absolute top-0 right-0 w-24 h-24 bg-indigo-500/5 rounded-full blur-xl group-hover:bg-indigo-500/10 transition-all" />
        <div className="flex items-center justify-between text-slate-400 text-xs mb-1">
          <span className="font-medium">Upload Rate</span>
          <ArrowUpCircle className="w-4 h-4 text-indigo-400" />
        </div>
        <div className="text-xl font-bold font-mono tracking-tight text-indigo-300">
          {formatSpeed(upSpeed)}
        </div>
        <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400">
          <span>Total Out:</span>
          <span className="font-mono text-slate-300">{formatBytes(totalUp)}</span>
        </div>
      </div>

      {/* Latency */}
      <div className="glass-panel rounded-xl p-3.5 border border-slate-800 relative overflow-hidden">
        <div className="flex items-center justify-between text-slate-400 text-xs mb-1">
          <span className="font-medium" title="Round-trip time measured to anycast DNS infrastructure">Internet RTT (Ping)</span>
          <Clock className={`w-4 h-4 ${latencyColor}`} />
        </div>
        <div className={`text-xl font-bold font-mono tracking-tight ${latencyColor}`}>
          {formatLatency(latency)}
        </div>
        <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400">
          <span>Quality:</span>
          <span className="font-medium text-slate-300">
            {latency <= 25 ? 'Ultra-low' : latency <= 60 ? 'Optimal' : 'Elevated'}
          </span>
        </div>
      </div>

      {/* Jitter */}
      <div className="glass-panel rounded-xl p-3.5 border border-slate-800 relative overflow-hidden">
        <div className="flex items-center justify-between text-slate-400 text-xs mb-1">
          <span className="font-medium" title="Statistical variation in consecutive ping round-trip times">RTT Variation (Jitter)</span>
          <Activity className={`w-4 h-4 ${jitterColor}`} />
        </div>
        <div className={`text-xl font-bold font-mono tracking-tight ${jitterColor}`}>
          {formatJitter(jitter)}
        </div>
        <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400">
          <span>Stability:</span>
          <span className="font-medium text-slate-300">
            {jitter <= 2 ? 'Rock Solid' : jitter <= 8 ? 'Normal' : 'Buffer Fluctuations'}
          </span>
        </div>
      </div>

      {/* Packet Loss */}
      <div className="glass-panel rounded-xl p-3.5 border border-slate-800 relative overflow-hidden">
        <div className="flex items-center justify-between text-slate-400 text-xs mb-1">
          <span className="font-medium">Packet Loss</span>
          <ShieldAlert className={`w-4 h-4 ${lossColor}`} />
        </div>
        <div className={`text-xl font-bold font-mono tracking-tight ${lossColor}`}>
          {packetLoss.toFixed(1)}%
        </div>
        <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400">
          <span>Integrity:</span>
          <span className="font-medium text-slate-300">
            {packetLoss === 0 ? '100% Clean' : 'Retransmissions'}
          </span>
        </div>
      </div>

      {/* Total Data Transferred */}
      <div className="glass-panel rounded-xl p-3.5 border border-slate-800 relative overflow-hidden">
        <div className="flex items-center justify-between text-slate-400 text-xs mb-1">
          <span className="font-medium">Session Total</span>
          <Database className="w-4 h-4 text-cyan-400" />
        </div>
        <div className="text-xl font-bold font-mono tracking-tight text-slate-200">
          {formatBytes(totalDown + totalUp)}
        </div>
        <div className="mt-1 flex items-center justify-between text-[11px] text-slate-400">
          <span>Adapter:</span>
          <span className="font-mono text-slate-300 truncate max-w-[90px]">
            {metrics?.activeAdapter?.name ?? 'Detecting...'}
          </span>
        </div>
      </div>
    </div>
  );
};

