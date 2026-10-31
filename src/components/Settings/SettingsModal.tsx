import React, { useState, useEffect } from 'react';
import { 
  X, 
  Sliders, 
  RefreshCw, 
  Terminal, 
  Check, 
  AlertCircle 
} from 'lucide-react';
import { OperatingMode, AppSettings } from '../../types/network';
import { 
  getAppSettings, 
  saveAppSettings, 
  getApplicationLogs, 
  restoreOriginalDns 
} from '../../services/tauriCommands';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  mode: OperatingMode;
  onModeChange: (m: OperatingMode) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  onClose,
  mode,
  onModeChange
}) => {
  const [settings, setSettings] = useState<AppSettings>({
    minimizeToTray: true,
    launchOnStartup: false,
    bufferbloatWarning: true,
    defaultMode: 'streaming'
  });
  const [logs, setLogs] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [savedNotice, setSavedNotice] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
    }
  }, [isOpen]);

  const loadSettings = async () => {
    try {
      const s = await getAppSettings();
      setSettings(s);
    } catch (e) {
      console.warn('Failed to load settings:', e);
    }
  };

  const handleToggle = async (key: keyof AppSettings, val: any) => {
    const updated = { ...settings, [key]: val };
    setSettings(updated);
    try {
      await saveAppSettings(updated);
      setSavedNotice('Setting updated.');
      setTimeout(() => setSavedNotice(null), 1500);
    } catch (e) {
      console.error('Failed to save setting:', e);
    }
  };

  const handleLoadLogs = async () => {
    try {
      const l = await getApplicationLogs();
      setLogs(l);
      setShowLogs(!showLogs);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRestoreDefaults = async () => {
    const confirmed = window.confirm(
      'Reset all NetBoost optimization settings, policies, and DNS overrides to Windows defaults?'
    );
    if (!confirmed) return;

    try {
      await restoreOriginalDns('Wi-Fi');
      const def: AppSettings = {
        minimizeToTray: true,
        launchOnStartup: false,
        bufferbloatWarning: true,
        defaultMode: 'streaming'
      };
      setSettings(def);
      await saveAppSettings(def);
      onModeChange('streaming');
      alert('Network settings and policies restored to default.');
    } catch (e) {
      alert('Restoration error: ' + e);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="glass-panel w-full max-w-lg rounded-2xl border border-slate-700 shadow-2xl p-5 space-y-4 max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-800 pb-3">
          <div className="flex items-center space-x-2">
            <Sliders className="w-5 h-5 text-cyan-400" />
            <h2 className="text-sm font-semibold text-slate-100">NetBoost Configuration</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Settings Body */}
        <div className="space-y-4 text-xs">
          {/* Operating Mode */}
          <div>
            <label className="font-semibold text-slate-300 block mb-1.5">
              Default Optimization Policy
            </label>
            <div className="grid grid-cols-3 gap-2">
              <button
                onClick={() => {
                  onModeChange('streaming');
                  handleToggle('defaultMode', 'streaming');
                }}
                className={`p-2.5 rounded-lg border text-left transition-all ${
                  mode === 'streaming'
                    ? 'border-cyan-500 bg-cyan-950/30 text-cyan-300 font-semibold'
                    : 'border-slate-800 bg-slate-900/60 text-slate-400 hover:border-slate-700'
                }`}
              >
                <div className="text-xs">Streaming</div>
                <div className="text-[10px] text-slate-500 font-normal mt-0.5">Protect video jitter</div>
              </button>

              <button
                onClick={() => {
                  onModeChange('balanced');
                  handleToggle('defaultMode', 'balanced');
                }}
                className={`p-2.5 rounded-lg border text-left transition-all ${
                  mode === 'balanced'
                    ? 'border-cyan-500 bg-cyan-950/30 text-cyan-300 font-semibold'
                    : 'border-slate-800 bg-slate-900/60 text-slate-400 hover:border-slate-700'
                }`}
              >
                <div className="text-xs">Balanced</div>
                <div className="text-[10px] text-slate-500 font-normal mt-0.5">Fair multi-tasking</div>
              </button>

              <button
                onClick={() => {
                  onModeChange('download');
                  handleToggle('defaultMode', 'download');
                }}
                className={`p-2.5 rounded-lg border text-left transition-all ${
                  mode === 'download'
                    ? 'border-cyan-500 bg-cyan-950/30 text-cyan-300 font-semibold'
                    : 'border-slate-800 bg-slate-900/60 text-slate-400 hover:border-slate-700'
                }`}
              >
                <div className="text-xs">Download</div>
                <div className="text-[10px] text-slate-500 font-normal mt-0.5">Max segmented rate</div>
              </button>
            </div>
          </div>

          {/* Toggles */}
          <div className="space-y-3 pt-2 border-t border-slate-800">
            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium text-slate-200 block">Launch on Windows Startup</span>
                <span className="text-[11px] text-slate-400">Configures HKCU Run registry entry to monitor on boot</span>
              </div>
              <input 
                type="checkbox" 
                checked={settings.launchOnStartup}
                onChange={(e) => handleToggle('launchOnStartup', e.target.checked)}
                className="accent-cyan-400 w-4 h-4 cursor-pointer" 
              />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium text-slate-200 block">Minimize to System Tray</span>
                <span className="text-[11px] text-slate-400">Keep active background protection running when closed</span>
              </div>
              <input 
                type="checkbox" 
                checked={settings.minimizeToTray}
                onChange={(e) => handleToggle('minimizeToTray', e.target.checked)}
                className="accent-cyan-400 w-4 h-4 cursor-pointer" 
              />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium text-slate-200 block">Bufferbloat Mitigation Warning</span>
                <span className="text-[11px] text-slate-400">Notify if background transfers degrade stream buffer</span>
              </div>
              <input 
                type="checkbox" 
                checked={settings.bufferbloatWarning}
                onChange={(e) => handleToggle('bufferbloatWarning', e.target.checked)}
                className="accent-cyan-400 w-4 h-4 cursor-pointer" 
              />
            </div>
          </div>

          {/* Diagnostic Logs Drawer */}
          <div className="pt-2 border-t border-slate-800 space-y-2">
            <button
              onClick={handleLoadLogs}
              className="text-xs text-cyan-400 hover:text-cyan-300 font-mono flex items-center space-x-1.5 transition-colors"
            >
              <Terminal className="w-3.5 h-3.5" />
              <span>{showLogs ? 'Hide Diagnostic Logs' : 'View Local Engine Logs (%APPDATA%\\NetBoost\\logs)'}</span>
            </button>

            {showLogs && (
              <div className="bg-slate-950 p-3 rounded-lg border border-slate-800 font-mono text-[10px] text-slate-300 max-h-36 overflow-y-auto space-y-0.5">
                {logs.length === 0 ? (
                  <span className="text-slate-500">No log entries recorded yet.</span>
                ) : (
                  logs.map((l, i) => (
                    <div key={i} className="truncate">{l}</div>
                  ))
                )}
              </div>
            )}
          </div>

          {/* Restore Original Network Settings */}
          <div className="pt-2 border-t border-slate-800">
            <button
              onClick={handleRestoreDefaults}
              className="w-full py-2 bg-slate-900 hover:bg-slate-800 border border-slate-700 rounded-lg text-slate-300 font-medium text-xs flex items-center justify-center space-x-2 transition-colors"
            >
              <RefreshCw className="w-3.5 h-3.5" />
              <span>Restore Factory Windows Network Defaults</span>
            </button>
          </div>
        </div>

        {/* Footer */}
        <div className="pt-3 border-t border-slate-800 flex items-center justify-between">
          <span className="text-[11px] text-emerald-400 font-mono">
            {savedNotice}
          </span>
          <button
            onClick={onClose}
            className="px-4 py-1.5 bg-cyan-500 text-dark-900 font-bold rounded-lg text-xs hover:bg-cyan-400 transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
