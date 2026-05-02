import { useMemo } from "react";
import type { HttpSession } from "../../types";

interface GrpcMessage {
  compressed: boolean;
  length: number;
  payload: Uint8Array | null;
}

function isGrpcWebRequest(session: HttpSession): boolean {
  const ct = session.request.content_type || "";
  const respCt = session.response?.content_type || "";
  return ct.startsWith("application/grpc") || respCt.startsWith("application/grpc");
}

function parseGrpcFrame(data: Uint8Array): GrpcMessage[] {
  const frames: GrpcMessage[] = [];
  let offset = 0;
  while (offset + 5 <= data.length) {
    const compressed = data[offset] === 1;
    const length = (data[offset + 1] << 24) | (data[offset + 2] << 16) | (data[offset + 3] << 8) | data[offset + 4];
    offset += 5;
    if (offset + length > data.length) break;
    const payload = data.slice(offset, offset + length);
    frames.push({ compressed, length, payload });
    offset += length;
  }
  return frames;
}

function tryDecodeProtobuf(payload: Uint8Array): string {
  const lines: string[] = [];
  let offset = 0;
  while (offset < payload.length) {
    const byte = payload[offset];
    const fieldNumber = byte >> 3;
    const wireType = byte & 0x07;
    offset++;
    if (fieldNumber === 0) break;
    const wireTypeLabel = ["varint", "64bit", "length-delimited", "start_group", "end_group", "32bit"][wireType] || "unknown";
    switch (wireType) {
      case 0: {
        let value = 0n;
        let shift = 0n;
        while (offset < payload.length) {
          const b = payload[offset++];
          value |= BigInt(b & 0x7f) << shift;
          if ((b & 0x80) === 0) break;
          shift += 7n;
        }
        lines.push(`  field ${fieldNumber} (${wireTypeLabel}): ${value}`);
        break;
      }
      case 2: {
        let len = 0;
        let shift = 0;
        while (offset < payload.length) {
          const b = payload[offset++];
          len |= (b & 0x7f) << shift;
          if ((b & 0x80) === 0) break;
          shift += 7;
        }
        if (offset + len <= payload.length) {
          const bytes = payload.slice(offset, offset + len);
          offset += len;
          try {
            const text = new TextDecoder().decode(bytes);
            if (/^[\x20-\x7E\u0080-\uFFFF]*$/.test(text) && text.length > 0) {
              lines.push(`  field ${fieldNumber} (${wireTypeLabel}): "${text}"`);
            } else {
              lines.push(`  field ${fieldNumber} (${wireTypeLabel}): [${len} bytes]`);
            }
          } catch {
            lines.push(`  field ${fieldNumber} (${wireTypeLabel}): [${len} bytes]`);
          }
        }
        break;
      }
      case 1: {
        offset += 8;
        lines.push(`  field ${fieldNumber} (${wireTypeLabel}): [64-bit]`);
        break;
      }
      case 5: {
        offset += 4;
        lines.push(`  field ${fieldNumber} (${wireTypeLabel}): [32-bit]`);
        break;
      }
      default: {
        lines.push(`  field ${fieldNumber} (${wireTypeLabel})`);
        break;
      }
    }
    if (lines.length > 50) break;
  }
  return lines.join("\n");
}

function extractGrpcPath(session: HttpSession): string {
  const path = session.request.url || "";
  return path;
}

export function GrpcView({ session }: { session: HttpSession }) {
  const grpcPath = extractGrpcPath(session);

  const requestFrames = useMemo(() => {
    if (!session.request.body) return [];
    return parseGrpcFrame(new Uint8Array(session.request.body));
  }, [session.request.body]);

  const responseFrames = useMemo(() => {
    if (!session.response?.body) return [];
    return parseGrpcFrame(new Uint8Array(session.response.body));
  }, [session.response?.body]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-3 py-1.5 bg-[var(--color-bg-secondary)] border-b border-[var(--color-border)]">
        <span className="px-1.5 py-0.5 text-[10px] rounded font-bold text-white bg-[var(--color-accent)]">gRPC</span>
        <span className="text-xs font-mono text-[var(--color-text-primary)] truncate">{grpcPath}</span>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        <div>
          <h4 className="text-[11px] font-semibold text-[var(--color-text-secondary)] mb-1">
            Request Frames ({requestFrames.length})
          </h4>
          {requestFrames.length === 0 ? (
            <p className="text-[11px] text-[var(--color-text-secondary)]">No request body</p>
          ) : (
            requestFrames.map((frame, i) => (
              <div key={i} className="mb-2 p-2 bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded">
                <div className="flex items-center gap-2 text-[10px] text-[var(--color-text-secondary)]">
                  <span>Frame #{i}</span>
                  <span>{frame.compressed ? "compressed" : "uncompressed"}</span>
                  <span>{frame.length} bytes</span>
                </div>
                {frame.payload && frame.payload.length > 0 && (
                  <pre className="mt-1 text-[10px] font-mono whitespace-pre-wrap break-all text-[var(--color-text-primary)]">
                    {tryDecodeProtobuf(frame.payload)}
                  </pre>
                )}
              </div>
            ))
          )}
        </div>

        <div>
          <h4 className="text-[11px] font-semibold text-[var(--color-text-secondary)] mb-1">
            Response Frames ({responseFrames.length})
          </h4>
          {responseFrames.length === 0 ? (
            <p className="text-[11px] text-[var(--color-text-secondary)]">No response body</p>
          ) : (
            responseFrames.map((frame, i) => (
              <div key={i} className="mb-2 p-2 bg-[var(--color-bg-secondary)] border border-[var(--color-border)] rounded">
                <div className="flex items-center gap-2 text-[10px] text-[var(--color-text-secondary)]">
                  <span>Frame #{i}</span>
                  <span>{frame.compressed ? "compressed" : "uncompressed"}</span>
                  <span>{frame.length} bytes</span>
                </div>
                {frame.payload && frame.payload.length > 0 && (
                  <pre className="mt-1 text-[10px] font-mono whitespace-pre-wrap break-all text-[var(--color-text-primary)]">
                    {tryDecodeProtobuf(frame.payload)}
                  </pre>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

export { isGrpcWebRequest };
