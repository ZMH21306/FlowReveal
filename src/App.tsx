import { AppShell } from "./components/layout/AppShell";
import { useTraffic } from "./hooks/useTraffic";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import "./styles/globals.css";

export default function App() {
  useTraffic();
  useKeyboardShortcuts();
  return <AppShell />;
}
