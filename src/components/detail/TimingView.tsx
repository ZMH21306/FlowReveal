import { formatDuration } from "../../lib/utils";

export function TimingView({ req, resp }: { req: { timestamp: number }; resp: { timestamp: number; duration_us: number | null } | null }) {
  const reqTime = new Date(req.timestamp / 1000);
  const duration = resp?.duration_us;

  return (
    <div className="space-y-2 text-xs">
      <div className="flex justify-between">
        <span className="text-[var(--color-text-secondary)]">请求发起时间</span>
        <span className="text-[var(--color-text-primary)] font-mono">
          {reqTime.toLocaleTimeString("zh-CN", { hour12: false, fractionalSecondDigits: 3 })}
        </span>
      </div>
      {duration != null && (
        <div className="flex justify-between">
          <span className="text-[var(--color-text-secondary)]">总耗时</span>
          <span className="text-[var(--color-text-primary)] font-mono">{formatDuration(duration)}</span>
        </div>
      )}
      {duration != null && (
        <div className="mt-2">
          <div className="h-2 bg-[var(--color-bg-tertiary)] rounded overflow-hidden">
            <div
              className="h-full bg-[var(--color-accent)] rounded"
              style={{ width: `${Math.min(100, (duration / 1_000_000) * 100)}%` }}
            />
          </div>
          <div className="flex justify-between mt-1 text-[10px] text-[var(--color-text-secondary)]">
            <span>0ms</span>
            <span>{(duration / 1_000_000).toFixed(2)}s</span>
          </div>
        </div>
      )}
    </div>
  );
}
