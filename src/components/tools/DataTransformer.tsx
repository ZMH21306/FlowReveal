import { useState } from "react";

type TransformType = "url-encode" | "url-decode" | "base64-encode" | "base64-decode" | "json-format" | "json-minify" | "unicode-escape" | "unicode-unescape" | "timestamp" | "hash-md5" | "hash-sha256";

const TRANSFORMS: { key: TransformType; label: string }[] = [
  { key: "url-encode", label: "URL编码" },
  { key: "url-decode", label: "URL解码" },
  { key: "base64-encode", label: "Base64编码" },
  { key: "base64-decode", label: "Base64解码" },
  { key: "json-format", label: "JSON格式化" },
  { key: "json-minify", label: "JSON压缩" },
  { key: "unicode-escape", label: "Unicode转义" },
  { key: "unicode-unescape", label: "Unicode反转义" },
  { key: "timestamp", label: "时间戳转换" },
  { key: "hash-md5", label: "MD5" },
  { key: "hash-sha256", label: "SHA256" },
];

function transform(input: string, type: TransformType): string {
  try {
    switch (type) {
      case "url-encode": return encodeURIComponent(input);
      case "url-decode": return decodeURIComponent(input);
      case "base64-encode": return btoa(unescape(encodeURIComponent(input)));
      case "base64-decode": return decodeURIComponent(escape(atob(input)));
      case "json-format": return JSON.stringify(JSON.parse(input), null, 2);
      case "json-minify": return JSON.stringify(JSON.parse(input));
      case "unicode-escape": return input.replace(/[\u0080-\uffff]/g, (ch) => {
        const code = ch.charCodeAt(0);
        return code > 0xffff ? `\\u{${code.toString(16)}}` : `\\u${code.toString(16).padStart(4, "0")}`;
      });
      case "unicode-unescape": return input.replace(/\\u\{?([0-9a-fA-F]+)\}?/g, (_, hex) => String.fromCodePoint(parseInt(hex, 16)));
      case "timestamp": {
        const num = Number(input);
        if (isNaN(num)) return "无效时间戳";
        const ms = num > 1e12 ? num : num * 1000;
        return new Date(ms).toLocaleString();
      }
      case "hash-md5":
      case "hash-sha256": {
        return "(需要Web Crypto API，将在异步模式运行)";
      }
    }
  } catch (e) {
    return `错误: ${(e as Error).message}`;
  }
}

interface DataTransformerProps {
  onClose: () => void;
}

export function DataTransformer({ onClose }: DataTransformerProps) {
  const [input, setInput] = useState("");
  const [transformType, setTransformType] = useState<TransformType>("url-encode");
  const [output, setOutput] = useState("");

  const handleTransform = () => {
    setOutput(transform(input, transformType));
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(output);
  };

  return (
    <div className="rule-edit-overlay" onClick={onClose}>
      <div className="rule-edit-dialog" onClick={(e) => e.stopPropagation()} style={{ width: 560 }}>
        <h3>🔧 数据转换器</h3>
        <div className="form-group">
          <label>输入</label>
          <textarea value={input} onChange={(e) => setInput(e.target.value)} rows={4} style={{ fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12 }} />
        </div>
        <div className="form-group">
          <label>转换类型</label>
          <div className="flex flex-wrap gap-1">
            {TRANSFORMS.map((t) => (
              <button key={t.key} className={`btn-small ${transformType === t.key ? "bg-[var(--color-accent)] text-white border-[var(--color-accent)]" : ""}`} onClick={() => setTransformType(t.key)}>
                {t.label}
              </button>
            ))}
          </div>
        </div>
        <div className="flex gap-2 mb-3">
          <button className="btn-primary" onClick={handleTransform}>转换</button>
        </div>
        <div className="form-group">
          <label>输出 <button className="btn-small ml-2" onClick={handleCopy}>📋 复制</button></label>
          <textarea value={output} readOnly rows={4} style={{ fontFamily: "'Cascadia Code', 'Fira Code', monospace", fontSize: 12, background: "var(--color-bg-primary)" }} />
        </div>
        <div className="dialog-actions">
          <button className="btn-secondary" onClick={onClose}>关闭</button>
        </div>
      </div>
    </div>
  );
}
