import { useEffect } from "react";
import { useStore, type StoreState } from "../store";
import { getStoredTheme, setStoredTheme, type ThemeMode } from "./useTheme";

interface KeyBinding {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: () => void;
  description: string;
}

export function useKeyboardShortcuts() {
  const selectRequest = useStore((s: StoreState) => s.selectRequest);
  const selectedId = useStore((s: StoreState) => s.selectedId);
  const sessionList = useStore((s: StoreState) => s.filteredSessionList);
  const clearRequests = useStore((s: StoreState) => s.clearRequests);
  const setCaptureStatus = useStore((s: StoreState) => s.setCaptureStatus);
  const resetFilter = useStore((s: StoreState) => s.resetFilter);
  const toggleBookmark = useStore((s: StoreState) => s.toggleBookmark);

  useEffect(() => {
    const bindings: KeyBinding[] = [
      {
        key: "ArrowDown",
        action: () => {
          if (selectedId === null && sessionList.length > 0) {
            selectRequest(sessionList[0]);
          } else if (selectedId !== null) {
            const idx = sessionList.indexOf(selectedId);
            if (idx < sessionList.length - 1) {
              selectRequest(sessionList[idx + 1]);
            }
          }
        },
        description: "选择下一条请求",
      },
      {
        key: "ArrowUp",
        action: () => {
          if (selectedId !== null) {
            const idx = sessionList.indexOf(selectedId);
            if (idx > 0) {
              selectRequest(sessionList[idx - 1]);
            }
          }
        },
        description: "选择上一条请求",
      },
      {
        key: "b",
        ctrl: true,
        action: () => {
          if (selectedId !== null) {
            toggleBookmark(selectedId);
          }
        },
        description: "切换书签",
      },
      {
        key: "Delete",
        action: () => {
          clearRequests();
        },
        description: "清空所有请求",
      },
      {
        key: "l",
        ctrl: true,
        action: () => {
          resetFilter();
        },
        description: "重置过滤器",
      },
      {
        key: "d",
        ctrl: true,
        action: () => {
          const current = getStoredTheme();
          const next: ThemeMode = current === "dark" ? "light" : current === "light" ? "system" : "dark";
          setStoredTheme(next);
        },
        description: "切换主题",
      },
    ];

    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT") {
        return;
      }

      for (const binding of bindings) {
        const keyMatch = e.key === binding.key || e.key.toLowerCase() === binding.key.toLowerCase();
        const ctrlMatch = binding.ctrl ? (e.ctrlKey || e.metaKey) : !(e.ctrlKey || e.metaKey);
        const shiftMatch = binding.shift ? e.shiftKey : !e.shiftKey;
        const altMatch = binding.alt ? e.altKey : !e.altKey;

        if (keyMatch && ctrlMatch && shiftMatch && altMatch) {
          e.preventDefault();
          binding.action();
          return;
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedId, sessionList, selectRequest, clearRequests, setCaptureStatus, resetFilter, toggleBookmark]);
}
