import { invoke } from "@tauri-apps/api/core";
import type { Rule, PresetRuleType } from "../types";

export async function addRule(rule: Rule): Promise<number> {
  return invoke<number>("add_rule", { rule });
}

export async function removeRule(id: number): Promise<boolean> {
  return invoke<boolean>("remove_rule", { id });
}

export async function toggleRule(id: number, enabled: boolean): Promise<boolean> {
  return invoke<boolean>("toggle_rule", { id, enabled });
}

export async function updateRule(id: number, rule: Rule): Promise<boolean> {
  return invoke<boolean>("update_rule", { id, rule });
}

export async function getRules(): Promise<Rule[]> {
  return invoke<Rule[]>("get_rules");
}

export async function enablePresetRule(preset: PresetRuleType): Promise<number> {
  return invoke<number>("enable_preset_rule", { preset });
}

export async function exportRules(): Promise<string> {
  return invoke<string>("export_rules");
}

export async function importRules(json: string): Promise<number> {
  return invoke<number>("import_rules", { json });
}

export async function clearRules(): Promise<void> {
  return invoke<void>("clear_rules");
}
