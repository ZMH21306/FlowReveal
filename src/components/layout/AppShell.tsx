import { useState, useCallback } from "react";
import { Toolbar } from "./Toolbar";
import { StatusBar } from "./StatusBar";
import { TrafficList } from "../traffic/TrafficList";
import { DslFilterBar } from "../traffic/DslFilterBar";
import { RequestDetail } from "../detail/RequestDetail";
import { RulesPanel } from "../rules/RulesPanel";
import { StatsPanel } from "../stats/StatsPanel";
import { DataTransformer } from "../tools/DataTransformer";
import { AiAssistant } from "../ai/AiAssistant";
import { TrafficDiff } from "../tools/TrafficDiff";
import { VulnScanner } from "../tools/VulnScanner";
import { PluginManager } from "../tools/PluginManager";

export function AppShell() {
  const [showRules, setShowRules] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [showTransformer, setShowTransformer] = useState(false);
  const [showAi, setShowAi] = useState(false);
  const [showDiff, setShowDiff] = useState(false);
  const [showVuln, setShowVuln] = useState(false);
  const [showPlugins, setShowPlugins] = useState(false);
  const [leftWidth, setLeftWidth] = useState(50);

  const handleDividerDrag = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const startX = e.clientX;
      const startWidth = leftWidth;
      const container = (e.target as HTMLElement).closest(".main-split") as HTMLElement;
      if (!container) return;
      const containerWidth = container.offsetWidth;

      const onMouseMove = (ev: MouseEvent) => {
        const delta = ev.clientX - startX;
        const pct = startWidth + (delta / containerWidth) * 100;
        setLeftWidth(Math.max(25, Math.min(75, pct)));
      };
      const onMouseUp = () => {
        document.removeEventListener("mousemove", onMouseMove);
        document.removeEventListener("mouseup", onMouseUp);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      };
      document.addEventListener("mousemove", onMouseMove);
      document.addEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [leftWidth]
  );

  return (
    <div className="flex flex-col h-screen w-screen bg-[var(--color-bg-primary)]">
      <Toolbar
        onToggleRules={() => setShowRules(!showRules)}
        showRules={showRules}
        onToggleStats={() => setShowStats(!showStats)}
        showStats={showStats}
        onOpenTransformer={() => setShowTransformer(true)}
        onOpenAi={() => setShowAi(true)}
        onOpenDiff={() => setShowDiff(true)}
        onOpenVuln={() => setShowVuln(true)}
        onOpenPlugins={() => setShowPlugins(true)}
      />
      <DslFilterBar />
      <div className="main-split flex flex-1 overflow-hidden">
        <div
          className="overflow-hidden flex flex-col"
          style={{ width: `${leftWidth}%` }}
        >
          <TrafficList />
        </div>
        <div
          className="w-[5px] shrink-0 cursor-col-resize bg-[var(--color-border-subtle)] hover:bg-[var(--color-accent)] transition-colors duration-150 active:bg-[var(--color-accent-hover)]"
          onMouseDown={handleDividerDrag}
          title="拖拽调整面板宽度"
        />
        <div className="flex-1 overflow-hidden">
          <RequestDetail />
        </div>
      </div>
      <StatusBar />
      {showRules && <RulesPanel onClose={() => setShowRules(false)} />}
      {showStats && <StatsPanel onClose={() => setShowStats(false)} />}
      {showTransformer && <DataTransformer onClose={() => setShowTransformer(false)} />}
      {showAi && <AiAssistant onClose={() => setShowAi(false)} />}
      {showDiff && <TrafficDiff onClose={() => setShowDiff(false)} />}
      {showVuln && <VulnScanner onClose={() => setShowVuln(false)} />}
      {showPlugins && <PluginManager onClose={() => setShowPlugins(false)} />}
    </div>
  );
}
