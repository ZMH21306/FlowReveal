import { useState, useEffect, useRef, useCallback, useMemo } from "react";

interface VirtualScrollOptions {
  itemCount: number;
  itemHeight: number;
  overscan?: number;
}

interface VirtualScrollResult {
  containerRef: React.RefObject<HTMLDivElement | null>;
  visibleItems: number[];
  offsetY: number;
  totalHeight: number;
  scrollToIndex: (index: number) => void;
}

export function useVirtualScroll({
  itemCount,
  itemHeight,
  overscan = 5,
}: VirtualScrollOptions): VirtualScrollResult {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(600);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setViewportHeight(entry.contentRect.height);
      }
    });
    observer.observe(el);

    const handleScroll = () => {
      setScrollTop(el.scrollTop);
    };
    el.addEventListener("scroll", handleScroll, { passive: true });

    setViewportHeight(el.clientHeight);

    return () => {
      observer.disconnect();
      el.removeEventListener("scroll", handleScroll);
    };
  }, []);

  const totalHeight = itemCount * itemHeight;

  const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  const endIndex = Math.min(
    itemCount - 1,
    Math.ceil((scrollTop + viewportHeight) / itemHeight) + overscan
  );

  const visibleItems = useMemo(() => {
    const items: number[] = [];
    for (let i = startIndex; i <= endIndex; i++) {
      items.push(i);
    }
    return items;
  }, [startIndex, endIndex]);

  const offsetY = startIndex * itemHeight;

  const scrollToIndex = useCallback(
    (index: number) => {
      const el = containerRef.current;
      if (!el) return;
      const top = index * itemHeight;
      el.scrollTo({ top, behavior: "smooth" });
    },
    [itemHeight]
  );

  return { containerRef, visibleItems, offsetY, totalHeight, scrollToIndex };
}
