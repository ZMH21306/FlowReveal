import { useState, useEffect, useCallback } from "react";
import type { Rule, PresetRuleType } from "../types";
import * as rulesApi from "../lib/rules-bindings";

export function useRules() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await rulesApi.getRules();
      setRules(result);
    } catch (e) {
      console.error("Failed to load rules:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const addRule = useCallback(async (rule: Rule) => {
    const id = await rulesApi.addRule(rule);
    await refresh();
    return id;
  }, [refresh]);

  const removeRule = useCallback(async (id: number) => {
    const ok = await rulesApi.removeRule(id);
    await refresh();
    return ok;
  }, [refresh]);

  const toggleRule = useCallback(async (id: number, enabled: boolean) => {
    const ok = await rulesApi.toggleRule(id, enabled);
    await refresh();
    return ok;
  }, [refresh]);

  const updateRule = useCallback(async (id: number, rule: Rule) => {
    const ok = await rulesApi.updateRule(id, rule);
    await refresh();
    return ok;
  }, [refresh]);

  const enablePreset = useCallback(async (preset: PresetRuleType) => {
    const id = await rulesApi.enablePresetRule(preset);
    await refresh();
    return id;
  }, [refresh]);

  const exportRules = useCallback(async () => {
    return rulesApi.exportRules();
  }, []);

  const importRules = useCallback(async (json: string) => {
    const count = await rulesApi.importRules(json);
    await refresh();
    return count;
  }, [refresh]);

  const clearRules = useCallback(async () => {
    await rulesApi.clearRules();
    await refresh();
  }, [refresh]);

  return {
    rules,
    loading,
    addRule,
    removeRule,
    toggleRule,
    updateRule,
    enablePreset,
    exportRules,
    importRules,
    clearRules,
    refresh,
  };
}
