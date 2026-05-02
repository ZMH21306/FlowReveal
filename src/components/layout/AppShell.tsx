import { useState } from "react";
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
      {showAi && <AiAssistant onClose={() => setShowAi(false)} />}
      {showDiff && <TrafficDiff onClose={() => setShowDiff(false)} />}
      {showVuln && <VulnScanner onClose={() => setShowVuln(false)} />}
      {showPlugins && <PluginManager onClose={() => setShowPlugins(false)} />}
    </div>
  );
}
