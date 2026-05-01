import { invoke } from "@tauri-apps/api/core";
import type { CaptureConfig, CertificateAuthority, ProcessEntry } from "../types";

export async function startCapture(config: CaptureConfig): Promise<void> {
  return invoke("start_capture", { config });
}

export async function stopCapture(): Promise<void> {
  return invoke("stop_capture");
}

export async function getRequests(
  offset: number,
  limit: number
): Promise<void> {
  return invoke("get_requests", { offset, limit });
}

export async function installCert(): Promise<void> {
  return invoke("install_cert");
}

export async function uninstallCert(): Promise<void> {
  return invoke("uninstall_cert");
}

export async function getCaCertPem(): Promise<string> {
  return invoke("get_ca_cert_pem");
}

export async function getCaInfo(): Promise<CertificateAuthority> {
  return invoke("get_ca_info");
}

export async function exportHar(sessionIds: number[]): Promise<string> {
  return invoke("export_har", { sessionIds });
}

export async function replayRequest(sessionId: number): Promise<void> {
  return invoke("replay_request", { sessionId });
}

export async function listRunningProcesses(): Promise<ProcessEntry[]> {
  return invoke("list_running_processes");
}

export async function injectHook(pid: number): Promise<void> {
  return invoke("inject_hook", { pid });
}

export async function ejectHook(pid: number): Promise<void> {
  return invoke("eject_hook", { pid });
}

export async function getInjectedPids(): Promise<number[]> {
  return invoke("get_injected_pids");
}
