import { useState } from "react";
import { requestElevation } from "../../lib/tauri-bindings";

interface ElevationDialogProps {
  open: boolean;
  onClose: () => void;
}

export function ElevationDialog({ open, onClose }: ElevationDialogProps) {
  const [requesting, setRequesting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const handleElevate = async () => {
    setRequesting(true);
    setError(null);
    try {
      await requestElevation();
    } catch (e) {
      setError(String(e));
    } finally {
      setRequesting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg shadow-xl max-w-md w-full mx-4 p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-base font-semibold text-[var(--color-text-primary)] mb-3">
          🔐 需要管理员权限
        </h3>
        <p className="text-sm text-[var(--color-text-secondary)] mb-4">
          全局捕获模式需要管理员权限才能启动 WinDivert 驱动进行网络数据包重定向。
          没有管理员权限时，将自动回退到仅代理模式（只能捕获配置了代理的应用）。
        </p>
        <div className="bg-[var(--color-bg-tertiary)] rounded-md p-3 mb-4 text-xs text-[var(--color-text-secondary)]">
          <p className="font-medium text-[var(--color-text-primary)] mb-1">权限说明：</p>
          <ul className="list-disc list-inside space-y-0.5">
            <li>WinDivert 驱动需要管理员权限加载</li>
            <li>系统代理设置需要管理员权限</li>
            <li>CA 证书安装到系统存储需要管理员权限</li>
          </ul>
        </div>
        {error && (
          <p className="text-xs text-[var(--color-error)] mb-3">{error}</p>
        )}
        <div className="flex items-center justify-end gap-2">
          <button
            onClick={onClose}
            className="px-3 py-1.5 text-xs font-medium rounded-[var(--radius-md)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-tertiary)] transition-all"
          >
            取消
          </button>
          <button
            onClick={handleElevate}
            disabled={requesting}
            className="px-3 py-1.5 text-xs font-medium rounded-[var(--radius-md)] bg-[var(--color-accent)] text-white hover:opacity-90 disabled:opacity-50 transition-all"
          >
            {requesting ? "请求中..." : "以管理员身份运行"}
          </button>
        </div>
      </div>
    </div>
  );
}
