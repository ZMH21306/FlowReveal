import { useState, useMemo } from "react";
import type { WebSocketFrame, WsOpcode } from "../../types";

interface WebSocketViewProps {
  frames: WebSocketFrame[];
}

function opcodeLabel(op: WsOpcode): string {
  switch (op) {
    case "Text": return "TXT";
    case "Binary": return "BIN";
    case "Close": return "CLS";
    case "Ping": return "PNG";
    case "Pong": return "POG";
    case "Continuation": return "CNT";
    default: return "???";
  }
}

function opcodeColor(op: WsOpcode): string {
  switch (op) {
    case "Text": return "var(--color-success)";
    case "Binary": return "var(--color-accent)";
    case "Close": return "var(--color-error)";
    case "Ping": return "var(--color-warning)";
    case "Pong": return "var(--color-warning)";
    default: return "var(--color-text-secondary)";
  }
}

function formatTimestamp(ts: number): string {
  const d = new Date(ts / 1000);
  return d.toLocaleTimeString("zh-CN", { hour12: false }) + "." + String(d.getMilliseconds()).padStart(3, "0");
}

function formatPayloadSize(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

function decodePayload(payload: number[] | null, opcode: WsOpcode): string {
  if (!payload || payload.length === 0) return "(empty)";
  try {
    const bytes = new Uint8Array(payload);
    if (opcode === "Text") {
      return new TextDecoder().decode(bytes);
    }
    const hex = Array.from(bytes.slice(0, 256))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(" ");
    return payload.length > 256 ? hex + " ..." : hex;
  } catch {
    return "(decode error)";
  }
}

type FilterMode = "all" | "sent" | "received";

export function WebSocketView({ frames }: WebSocketViewProps) {
  const [filterMode, setFilterMode] = useState<FilterMode>("all");
  const [selectedFrameId, setSelectedFrameId] = useState<number | null>(null);
  const [viewMode, setViewMode] = useState<"hex" | "text">("text");

  const filtered = useMemo(() => {
    switch (filterMode) {
      case "sent": return frames.filter((f) => f.direction === "Request");
      case "received": return frames.filter((f) => f.direction === "Response");
      default: return frames;
    }
  }, [frames, filterMode]);

  const selectedFrame = useMemo(
    () => frames.find((f) => f.id === selectedFrameId) ?? null,
    [frames, selectedFrameId]
  );

  const sentCount = frames.filter((f) => f.direction === "Request").length;
  const recvCount = frames.filter((f) => f.direction === "Response").length;

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <div className="flex items-center gap-2 px-3 py-1.5 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)] shrink-0">
        <span className="text-[11px] font-semibold text-[var(--color-text-primary)]">WebSocket Frames</span>
        <span className="text-[10px] text-[var(--color-text-secondary)]">{frames.length} frames</span>
        <span className="text-[10px] text-[var(--color-success)]">↑{sentCount}</span>
        <span className="text-[10px] text-[var(--color-accent)]">↓{recvCount}</span>
        <div className="flex-1" />
        <div className="flex gap-1">
          {(["all", "sent", "received"] as FilterMode[]).map((mode) => (
            <button
              key={mode}
              onClick={() => setFilterMode(mode)}
              className={`px-2 py-0.5 text-[10px] rounded ${
                filterMode === mode
                  ? "bg-[var(--color-accent)] text-white"
                  : "bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-hover)]"
              }`}
            >
              {mode === "all" ? "全部" : mode === "sent" ? "发送" : "接收"}
            </button>
          ))}
        </div>
      </div>

      <div className="flex flex-1 overflow-hidden">
        <div className="w-[340px] shrink-0 overflow-y-auto border-r border-[var(--color-border)]">
          {filtered.length === 0 ? (
            <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-xs">
              暂无 WebSocket 帧
            </div>
          ) : (
            filtered.map((frame) => (
              <div
                key={frame.id}
                onClick={() => setSelectedFrameId(frame.id)}
                className={`flex items-center gap-2 px-3 py-1 text-[11px] cursor-pointer border-b border-[var(--color-border)] hover:bg-[var(--color-bg-tertiary)] ${
                  selectedFrameId === frame.id ? "bg-[var(--color-bg-tertiary)] border-l-2 border-l-[var(--color-accent)]" : ""
                }`}
              >
                <span className="w-4 text-center" style={{ color: frame.direction === "Request" ? "var(--color-success)" : "var(--color-accent)" }}>
                  {frame.direction === "Request" ? "↑" : "↓"}
                </span>
                <span className="font-mono font-bold text-[10px] w-8" style={{ color: opcodeColor(frame.opcode) }}>
                  {opcodeLabel(frame.opcode)}
                </span>
                <span className="text-[var(--color-text-secondary)] flex-1 truncate">
                  {frame.opcode === "Text" && frame.payload
                    ? decodePayload(frame.payload, "Text").slice(0, 40)
                    : formatPayloadSize(frame.payload_size)}
                </span>
                <span className="text-[var(--color-text-secondary)] text-[10px] whitespace-nowrap">
                  {formatTimestamp(frame.timestamp)}
                </span>
              </div>
            ))
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-3">
          {selectedFrame ? (
            <div className="space-y-2">
              <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[11px]">
                <span className="text-[var(--color-text-secondary)]">方向:</span>
                <span style={{ color: selectedFrame.direction === "Request" ? "var(--color-success)" : "var(--color-accent)" }}>
                  {selectedFrame.direction === "Request" ? "客户端 → 服务器" : "服务器 → 客户端"}
                </span>
                <span className="text-[var(--color-text-secondary)]">操作码:</span>
                <span style={{ color: opcodeColor(selectedFrame.opcode) }}>{selectedFrame.opcode}</span>
                <span className="text-[var(--color-text-secondary)]">大小:</span>
                <span>{formatPayloadSize(selectedFrame.payload_size)}{selectedFrame.payload_truncated ? " (截断)" : ""}</span>
                <span className="text-[var(--color-text-secondary)]">时间:</span>
                <span>{formatTimestamp(selectedFrame.timestamp)}</span>
              </div>
              <div className="flex gap-1 pt-1">
                <button
                  onClick={() => setViewMode("text")}
                  className={`px-2 py-0.5 text-[10px] rounded ${
                    viewMode === "text" ? "bg-[var(--color-accent)] text-white" : "bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)]"
                  }`}
                >
                  文本
                </button>
                <button
                  onClick={() => setViewMode("hex")}
                  className={`px-2 py-0.5 text-[10px] rounded ${
                    viewMode === "hex" ? "bg-[var(--color-accent)] text-white" : "bg-[var(--color-bg-tertiary)] text-[var(--color-text-secondary)]"
                  }`}
                >
                  十六进制
                </button>
              </div>
              <pre className="bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded p-2 text-[11px] font-mono whitespace-pre-wrap break-all max-h-[400px] overflow-y-auto text-[var(--color-text-primary)]">
                {viewMode === "text"
                  ? decodePayload(selectedFrame.payload, selectedFrame.opcode)
                  : selectedFrame.payload
                    ? Array.from(new Uint8Array(selectedFrame.payload))
                        .map((b) => b.toString(16).padStart(2, "0"))
                        .join(" ")
                    : "(empty)"}
              </pre>
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-[var(--color-text-secondary)] text-xs">
              选择一个帧查看详情
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
