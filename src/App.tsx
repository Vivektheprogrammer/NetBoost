import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { NetworkOverview } from './components/Dashboard/NetworkOverview';
import { LiveSpeedChart } from './components/Dashboard/LiveSpeedChart';
import { NetworkHealthIndicator } from './components/Dashboard/NetworkHealthIndicator';
import { ActiveAdapterCard } from './components/Dashboard/ActiveAdapterCard';
import { OptimizationStatusBar } from './components/Dashboard/OptimizationStatusBar';
import { ProcessList } from './components/Applications/ProcessList';
import { DownloadManager } from './components/Downloads/DownloadManager';
import { DnsOptimizer } from './components/Dns/DnsOptimizer';
import { BenchmarkView } from './components/Benchmark/BenchmarkView';
import { SettingsModal } from './components/Settings/SettingsModal';
import { QuickDownloadModal } from './components/Downloads/QuickDownloadModal';
import { 
  getRealtimeMetrics, 
  getOptimizationState, 
  getActiveProcesses, 
  setOptimizationMode, 
  toggleOptimization,
  openExternalUrl
} from './services/tauriCommands';
import { RealtimeMetrics, OptimizationState, ProcessNetworkReport, OperatingMode } from './types/network';

export const App: React.FC = () => {
  const [currentTab, setCurrentTab] = useState<'dashboard' | 'applications' | 'downloads' | 'dns' | 'benchmark'>('dashboard');
  const [metrics, setMetrics] = useState<RealtimeMetrics | null>(null);
  const [optState, setOptState] = useState<OptimizationState | null>(null);
  const [processes, setProcesses] = useState<ProcessNetworkReport[]>([]);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  // 60-second rolling history of throughput
  const [speedHistory, setSpeedHistory] = useState<{ time: number; downloadSpeed: number; uploadSpeed: number }[]>([]);

  // Global external link handler: ensures any http/https link clicked anywhere in NetBoost launches the user's default browser
  useEffect(() => {
    const handleGlobalClick = (e: MouseEvent) => {
      const target = (e.target as HTMLElement)?.closest('a');
      if (target) {
        const href = target.getAttribute('href');
        if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
          e.preventDefault();
          e.stopPropagation();
          openExternalUrl(href);
        }
      }
    };

    document.addEventListener('click', handleGlobalClick, true);
    return () => {
      document.removeEventListener('click', handleGlobalClick, true);
    };
  }, []);

  // Polling loop for live telemetry
  useEffect(() => {
    let mounted = true;

    const poll = async () => {
      try {
        const [m, opt] = await Promise.all([
          getRealtimeMetrics(),
          getOptimizationState()
        ]);

        if (!mounted) return;

        setMetrics(m);
        setOptState(opt);

        // Append to 60-second history
        setSpeedHistory((prev) => {
          const updated = [...prev, {
            time: m.timestamp,
            downloadSpeed: m.downloadSpeedBps,
            uploadSpeed: m.uploadSpeedBps
          }];
          if (updated.length > 60) {
            return updated.slice(updated.length - 60);
          }
          return updated;
        });
      } catch (err) {
        console.error('Error fetching live telemetry:', err);
      }
    };

    poll();
    const interval = setInterval(poll, 1000);
    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  // Fetch active processes on tab switch or manual refresh
  const loadProcesses = async () => {
    try {
      const procs = await getActiveProcesses();
      setProcesses(procs);
    } catch (err) {
      console.error('Error fetching process network table:', err);
    }
  };

  useEffect(() => {
    if (currentTab === 'applications') {
      loadProcesses();
      const interval = setInterval(loadProcesses, 2000);
      return () => clearInterval(interval);
    }
  }, [currentTab]);

  const handleModeChange = async (mode: OperatingMode) => {
    const updated = await setOptimizationMode(mode);
    setOptState(updated);
  };

  const handleToggleOptimization = async () => {
    if (!optState) return;
    const updated = await toggleOptimization(!optState.enabled);
    setOptState(updated);
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-dark-900 text-slate-100 font-sans overflow-hidden">
      {/* Top Application Bar */}
      <Header
        currentTab={currentTab}
        onTabChange={(tab) => setCurrentTab(tab as any)}
        metrics={metrics}
        optState={optState}
        onModeChange={handleModeChange}
        onToggleOptimization={handleToggleOptimization}
        onOpenSettings={() => setIsSettingsOpen(true)}
      />

      {/* Main Content Area */}
      <main className="flex-1 overflow-y-auto p-4 space-y-4 max-w-7xl w-full mx-auto">
        {currentTab === 'dashboard' && (
          <div className="space-y-4 animate-in fade-in duration-300">
            {/* Optimization Status Banner */}
            <OptimizationStatusBar optState={optState} />

            {/* Top Stat Cards */}
            <NetworkOverview metrics={metrics} />

            {/* Real-time Graph */}
            <LiveSpeedChart
              history={speedHistory}
              currentDown={metrics?.downloadSpeedBps ?? 0}
              currentUp={metrics?.uploadSpeedBps ?? 0}
            />

            {/* Health & Adapter row */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <NetworkHealthIndicator health={metrics?.health} />
              <ActiveAdapterCard adapter={metrics?.activeAdapter} />
            </div>
          </div>
        )}

        {currentTab === 'applications' && (
          <div className="animate-in fade-in duration-300">
            <ProcessList processes={processes} onRefresh={loadProcesses} />
          </div>
        )}

        {currentTab === 'downloads' && (
          <div className="animate-in fade-in duration-300">
            <DownloadManager />
          </div>
        )}

        {currentTab === 'dns' && (
          <div className="animate-in fade-in duration-300">
            <DnsOptimizer />
          </div>
        )}

        {currentTab === 'benchmark' && (
          <div className="animate-in fade-in duration-300">
            <BenchmarkView />
          </div>
        )}
      </main>

      {/* Settings Modal */}
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        mode={optState?.mode ?? 'streaming'}
        onModeChange={handleModeChange}
      />

      {/* Quick Download Pop-up Modal (Triggered by Browser Click or Clipboard) */}
      <QuickDownloadModal onDownloadStarted={() => setCurrentTab('downloads')} />
    </div>
  );
};

export default App;

