import { useState } from "react";

interface ProgressBarProps {
  currentTime: number;
  duration: number;
  onSeek: (time: number) => void;
  label?: string;
  className?: string;
  showTime?: boolean;
}

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function ProgressBar({
  currentTime,
  duration,
  onSeek,
  label = "播放进度，可拖动快进快退",
  className = "",
  showTime = true,
}: ProgressBarProps) {
  const [dragValue, setDragValue] = useState<number | null>(null);
  const max = duration > 0 ? duration : 0;
  const value = dragValue ?? Math.min(currentTime, max || currentTime);
  const percent = max > 0 ? (value / max) * 100 : 0;

  const commit = () => {
    if (dragValue !== null) {
      onSeek(dragValue);
      setDragValue(null);
    }
  };

  return (
    <div className={`progress-bar ${className}`.trim()}>
      <input
        type="range"
        className="progress-bar__range"
        min={0}
        max={max}
        step={0.1}
        value={value}
        disabled={max === 0}
        aria-label={label}
        style={{
          backgroundImage: `linear-gradient(to right, var(--text-primary) ${percent}%, var(--hairline-strong) ${percent}%)`,
        }}
        onChange={(event) => setDragValue(Number(event.target.value))}
        onPointerUp={commit}
        onKeyUp={commit}
        onBlur={commit}
      />
      {showTime && (
        <div className="progress-bar__time">
          <span>{formatTime(dragValue ?? currentTime)}</span>
          <span>{formatTime(duration)}</span>
        </div>
      )}
    </div>
  );
}
