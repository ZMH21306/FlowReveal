import { useState, useEffect, useRef } from "react";
import type { HttpSession } from "../../types";

interface ContextMenuProps {
  x: number;
  y: number;
  session: HttpSession;
  onClose: () => void;
  onCopyUrl: (session: HttpSession) => void;
  onCopyResponseBody: (session: HttpSession) => void;
  onCopyRequestBody: (session: HttpSession) => void;
  onCopyHeaders: (session: HttpSession) => void;
  onCopyCurl: (session: HttpSession) => void;
  onFilterByHost: (session: HttpSession) => void;
  onFilterByUrl: (session: HttpSession) => void;
  onBookmark: (session: HttpSession) => void;
  onReplay: (session: HttpSession) => void;
  onOpenInBrowser: (session: HttpSession) => void;
}

interface SubMenu {
  label: string;
  items: { label: string; action: () => void }[];
}

export function ContextMenu({
  x, y, session, onClose,
  onCopyUrl, onCopyResponseBody, onCopyRequestBody, onCopyHeaders, onCopyCurl,
  onFilterByHost, onFilterByUrl,
  onBookmark, onReplay, onOpenInBrowser,
}: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeSubmenu, setActiveSubmenu] = useState<string | null>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  const copySubmenu: SubMenu = {
    label: "📋 复制",
    items: [
      { label: "复制 URL", action: () => { onCopyUrl(session); onClose(); } },
      { label: "复制 响应内容", action: () => { onCopyResponseBody(session); onClose(); } },
      { label: "复制 请求内容", action: () => { onCopyRequestBody(session); onClose(); } },
      { label: "复制 请求标头", action: () => { onCopyHeaders(session); onClose(); } },
      { label: "复制为 cURL", action: () => { onCopyCurl(session); onClose(); } },
    ],
  };

  const filterSubmenu: SubMenu = {
    label: "🔍 过滤",
    items: [
      { label: "按此URL过滤", action: () => { onFilterByUrl(session); onClose(); } },
      { label: "按此域名过滤", action: () => { onFilterByHost(session); onClose(); } },
    ],
  };

  const menuItems = [
    { type: "submenu" as const, data: copySubmenu },
    { type: "submenu" as const, data: filterSubmenu },
    { type: "separator" as const },
    { type: "item" as const, label: "⭐ 添加书签", action: () => { onBookmark(session); onClose(); } },
    { type: "item" as const, label: "↻ 重放请求", action: () => { onReplay(session); onClose(); } },
    { type: "item" as const, label: "🌐 在浏览器打开", action: () => { onOpenInBrowser(session); onClose(); } },
  ];

  const adjustedX = Math.min(x, window.innerWidth - 220);
  const adjustedY = Math.min(y, window.innerHeight - 300);

  return (
    <div
      ref={menuRef}
      className="context-menu"
      style={{ left: adjustedX, top: adjustedY }}
    >
      {menuItems.map((item, idx) => {
        if (item.type === "separator") {
          return <div key={idx} className="context-menu-separator" />;
        }
        if (item.type === "submenu") {
          return (
            <div
              key={idx}
              className="context-menu-item has-submenu"
              onMouseEnter={() => setActiveSubmenu(item.data.label)}
              onMouseLeave={() => setActiveSubmenu(null)}
            >
              <span>{item.data.label}</span>
              <span className="submenu-arrow">▸</span>
              {activeSubmenu === item.data.label && (
                <div className="context-submenu">
                  {item.data.items.map((sub, si) => (
                    <button key={si} className="context-menu-item" onClick={sub.action}>
                      {sub.label}
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        }
        return (
          <button key={idx} className="context-menu-item" onClick={item.action}>
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
