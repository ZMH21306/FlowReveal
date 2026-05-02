import { Toolbar } from "./Toolbar";
import { StatusBar } from "./StatusBar";
import { TrafficList } from "../traffic/TrafficList";
import { FilterBar } from "../traffic/FilterBar";
import { RequestDetail } from "../detail/RequestDetail";

export function AppShell() {
  return (
    <div className="flex flex-col h-screen w-screen bg-[var(--color-bg-primary)]">
      <Toolbar />
      <FilterBar />
      <div className="flex flex-1 overflow-hidden">
        <div className="w-[55%] border-r border-[var(--color-border)] overflow-hidden">
          <TrafficList />
        </div>
        <div className="flex-1 overflow-hidden">
          <RequestDetail />
        </div>
      </div>
      <StatusBar />
    </div>
  );
}
