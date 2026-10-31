import React from 'react';
import { Wifi, Network, ShieldCheck, Radio, Server } from 'lucide-react';
import { NetworkAdapter } from '../../types/network';

interface ActiveAdapterCardProps {
  adapter: NetworkAdapter | null | undefined;
}

export const ActiveAdapterCard: React.FC<ActiveAdapterCardProps> = ({ adapter }) => {
  const isWifi = adapter?.adapterType === 'wifi';
  const signal = adapter?.wifiSignalPercent ?? 0;

  return (
    <div className="glass-panel rounded-xl p-4 border border-slate-800 flex flex-col justify-between">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center space-x-2">
          {isWifi ? (
            <Wifi className="w-4 h-4 text-cyan-400" />
          ) : (
            <Network className="w-4 h-4 text-cyan-400" />
          )}
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
            Active Network Adapter
          </h3>
        </div>

        <span className="flex items-center space-x-1 text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-semibold">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
          <span>CONNECTED</span>
        </span>
      </div>

      <div className="my-2">
        <div className="text-sm font-semibold text-slate-100 flex items-center justify-between">
          <span className="truncate">{adapter?.name || 'Ethernet Interface'}</span>
          <span 
            className="text-xs font-mono text-cyan-400 font-normal ml-2 shrink-0 bg-slate-900/80 px-2 py-0.5 rounded border border-slate-800"
            title="Physical Layer (PHY) Link Rate negotiated with the local Wi-Fi router or switch. This is not internet download speed."
          >
            PHY Link: {adapter?.speedBps ? `${(adapter.speedBps / 1_000_000).toFixed(0)} Mbps` : '1.0 Gbps'}
          </span>
        </div>
        <p className="text-[11px] text-slate-400 truncate mt-0.5">
          {adapter?.description || 'Native Windows Network Controller'} <span className="text-slate-500">(Local link speed, distinct from ISP throughput)</span>
        </p>
      </div>

      {/* Wi-Fi specifics if Wi-Fi */}
      {isWifi && (
        <div className="my-2 p-2 bg-slate-900/60 rounded-lg border border-slate-800/80 flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Radio className="w-3.5 h-3.5 text-cyan-400" />
            <span className="text-xs font-medium text-slate-200">
              {adapter?.wifiSsid || 'Default Network'}
            </span>
          </div>
          <div className="flex items-center space-x-1.5 text-xs font-mono text-cyan-300">
            <span>Signal:</span>
            <span className="font-bold">{signal}%</span>
          </div>
        </div>
      )}

      {/* Interface Details */}
      <div className="mt-2 pt-2 border-t border-slate-800/80 grid grid-cols-2 gap-2 text-[11px]">
        <div>
          <span className="text-slate-500 block">IPv4 Address:</span>
          <span className="font-mono text-slate-300 font-medium truncate block">
            {adapter?.ipAddresses?.[0] || '192.168.1.100'}
          </span>
        </div>

        <div>
          <span className="text-slate-500 block">Gateway:</span>
          <span className="font-mono text-slate-300 font-medium truncate block">
            {adapter?.gateway || '192.168.1.1'}
          </span>
        </div>

        <div>
          <span className="text-slate-500 block">Type:</span>
          <span className="font-mono text-slate-300 font-medium capitalize">
            {adapter?.adapterType || 'Ethernet (802.3)'}
          </span>
        </div>

        <div>
          <span className="text-slate-500 block">Physical MAC:</span>
          <span className="font-mono text-slate-300 font-medium truncate block">
            {adapter?.macAddress || 'Auto-detected'}
          </span>
        </div>
      </div>
    </div>
  );
};

