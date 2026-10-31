import React, { useMemo } from 'react';
import { formatSpeed } from '../../utils/formatters';

interface DataPoint {
  time: number;
  downloadSpeed: number;
  uploadSpeed: number;
}

interface LiveSpeedChartProps {
  history: DataPoint[];
  currentDown: number;
  currentUp: number;
}

export const LiveSpeedChart: React.FC<LiveSpeedChartProps> = ({
  history,
  currentDown,
  currentUp
}) => {
  const width = 800;
  const height = 180;
  const padding = { top: 15, right: 15, bottom: 25, left: 55 };

  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;

  // Compute peak speed for scaling
  const maxSpeed = useMemo(() => {
    let max = Math.max(currentDown, currentUp, 100_000); // minimum 100 KB/s scale
    for (const p of history) {
      if (p.downloadSpeed > max) max = p.downloadSpeed;
      if (p.uploadSpeed > max) max = p.uploadSpeed;
    }
    // Add 15% headroom
    return max * 1.15;
  }, [history, currentDown, currentUp]);

  // Generate SVG path points
  const points = useMemo(() => {
    if (history.length < 2) return { downPath: '', upPath: '', downArea: '', upArea: '' };

    const count = history.length;
    const stepX = chartWidth / (count - 1);

    const downCoords = history.map((pt, i) => {
      const x = padding.left + i * stepX;
      const y = padding.top + chartHeight - (pt.downloadSpeed / maxSpeed) * chartHeight;
      return { x, y: Math.max(padding.top, Math.min(padding.top + chartHeight, y)) };
    });

    const upCoords = history.map((pt, i) => {
      const x = padding.left + i * stepX;
      const y = padding.top + chartHeight - (pt.uploadSpeed / maxSpeed) * chartHeight;
      return { x, y: Math.max(padding.top, Math.min(padding.top + chartHeight, y)) };
    });

    const buildPath = (coords: { x: number; y: number }[]) => {
      return coords.reduce((acc, pt, i) => `${acc} ${i === 0 ? 'M' : 'L'} ${pt.x.toFixed(1)},${pt.y.toFixed(1)}`, '');
    };

    const buildArea = (coords: { x: number; y: number }[]) => {
      const basePath = buildPath(coords);
      const lastX = coords[coords.length - 1].x;
      const firstX = coords[0].x;
      const bottomY = padding.top + chartHeight;
      return `${basePath} L ${lastX},${bottomY} L ${firstX},${bottomY} Z`;
    };

    return {
      downPath: buildPath(downCoords),
      upPath: buildPath(upCoords),
      downArea: buildArea(downCoords),
      upArea: buildArea(upCoords),
    };
  }, [history, maxSpeed, chartWidth, chartHeight]);

  // Grid levels (0%, 25%, 50%, 75%, 100%)
  const yTicks = [
    { value: maxSpeed, y: padding.top },
    { value: maxSpeed * 0.66, y: padding.top + chartHeight * 0.33 },
    { value: maxSpeed * 0.33, y: padding.top + chartHeight * 0.66 },
    { value: 0, y: padding.top + chartHeight },
  ];

  return (
    <div className="w-full glass-panel rounded-xl p-4 border border-slate-800">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center space-x-2">
          <span className="w-2 h-2 rounded-full bg-cyan-400 animate-pulse" />
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300">
            Real-Time Network Throughput
          </h3>
          <span className="text-[10px] text-slate-500 font-mono">(Live 60-Second Window)</span>
        </div>

        {/* Legend */}
        <div className="flex items-center space-x-4 text-xs font-mono">
          <div className="flex items-center space-x-1.5">
            <span className="w-2.5 h-0.5 bg-cyan-400 rounded-full" />
            <span className="text-slate-400">Down:</span>
            <span className="text-cyan-300 font-semibold">{formatSpeed(currentDown)}</span>
          </div>
          <div className="flex items-center space-x-1.5">
            <span className="w-2.5 h-0.5 bg-indigo-400 rounded-full" />
            <span className="text-slate-400">Up:</span>
            <span className="text-indigo-300 font-semibold">{formatSpeed(currentUp)}</span>
          </div>
        </div>
      </div>

      {/* SVG Canvas */}
      <div className="w-full h-44 relative">
        <svg
          viewBox={`0 0 ${width} ${height}`}
          className="w-full h-full overflow-visible"
          preserveAspectRatio="none"
        >
          <defs>
            <linearGradient id="downGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#06b6d4" stopOpacity="0.3" />
              <stop offset="100%" stopColor="#06b6d4" stopOpacity="0.0" />
            </linearGradient>
            <linearGradient id="upGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#818cf8" stopOpacity="0.25" />
              <stop offset="100%" stopColor="#818cf8" stopOpacity="0.0" />
            </linearGradient>
          </defs>

          {/* Horizontal Grid lines */}
          {yTicks.map((tick, i) => (
            <g key={i}>
              <line
                x1={padding.left}
                y1={tick.y}
                x2={width - padding.right}
                y2={tick.y}
                stroke="#1e293b"
                strokeDasharray={i === yTicks.length - 1 ? 'none' : '3 3'}
                strokeWidth="1"
              />
              <text
                x={padding.left - 8}
                y={tick.y + 3}
                textAnchor="end"
                className="text-[9px] fill-slate-500 font-mono"
              >
                {formatSpeed(tick.value)}
              </text>
            </g>
          ))}

          {/* Area Fills */}
          {points.downArea && <path d={points.downArea} fill="url(#downGrad)" />}
          {points.upArea && <path d={points.upArea} fill="url(#upGrad)" />}

          {/* Lines */}
          {points.upPath && (
            <path
              d={points.upPath}
              fill="none"
              stroke="#818cf8"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          )}
          {points.downPath && (
            <path
              d={points.downPath}
              fill="none"
              stroke="#06b6d4"
              strokeWidth="2.2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          )}
        </svg>
      </div>
    </div>
  );
};

