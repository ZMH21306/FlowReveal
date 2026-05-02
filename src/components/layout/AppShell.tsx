import { useState } from "react";
import { Toolbar } from "./Toolbar";
import { StatusBar } from "./StatusBar";
import { TrafficList } from "../traffic/TrafficList";
import { DslFilterBar } from "../traffic/DslFilterBar";
import { RequestDetail } from "../detail/RequestDetail";
import { RulesPanel } from "../rules/RulesPanel";
import { StatsPanel } from "../stats/StatsPanel";
import { DataTransformer } from "../tools/DataTransformer";

export function AppShell() {
  const [showRules, setShowRules] = useState(false);
  const [showStats, setShowStats] = useState(false);
  const [showTransformer, setShowTransformer] = useState(false);

  return (
    <div className="flex flex-col h-screen w-screen bg-[var(--color-bg-primary)]">
      <Toolbar
        onToggleRules={() => setShowRules(!showRules)}
        showRules={showRules}
        onToggleStats={() => setShowStats(!showStats)}
        showStats={showStats}
        onOpenTransformer={() => setShowTransformer(true)}
      />
      <DslFilterBar />
      <div className="flex flex-1 overflow-hidden">
        <div className={`border-r border-[var(--color-border)] overflow-hidden transition-all ${showRules ? "w-[calc(55%-380px)]" : "w-[55%]"}`}>
          <TrafficList />
        </div>
        <div className="flex-1 overflow-hidden">
          <RequestDetail />
        </div>
      </div>
      <StatusBar />
      {showRules && <RulesPanel onClose={() => setShowRules(false)} />}
      {showStats && <StatsPanel onClose={() => setShowStats(false)} />}
      {showTransformer && <DataTransformer onClose={() => setShowTransformer(false)} />}
    </div>
  );
}
