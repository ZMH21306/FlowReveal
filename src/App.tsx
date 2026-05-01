import { AppShell } from "./components/layout/AppShell";
import { useTraffic } from "./hooks/useTraffic";
import "./styles/globals.css";

export default function App() {
  useTraffic();
  return <AppShell />;
}
