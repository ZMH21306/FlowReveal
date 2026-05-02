import { useState } from "react";
import { useRules } from "../../hooks/useRules";
import type { Rule, RuleCategory, PresetRuleType, MatchLogic, MatchField, MatchOperator, AutoReplyAction, HeaderModifierAction, RedirectAction, BodySource, RedirectType, HeaderAction } from "../../types";

const PRESETS: { key: PresetRuleType; label: string; icon: string; category: RuleCategory }[] = [
  { key: "CorsEnable", label: "CORS", icon: "🌐", category: "HeaderModifier" },
  { key: "CacheDisable", label: "禁缓存", icon: "🚫", category: "HeaderModifier" },
  { key: "CookiesRemove", label: "删Cookie", icon: "🍪", category: "HeaderModifier" },
  { key: "ServiceUnavailable503", label: "503", icon: "⛔", category: "AutoReply" },
  { key: "Redirect302", label: "302", icon: "↗️", category: "AutoReply" },
  { key: "Ok200", label: "200", icon: "✅", category: "AutoReply" },
];

const MATCH_FIELDS: { value: MatchField; label: string }[] = [
  { value: "Method", label: "Method" },
  { value: "Url", label: "URL" },
  { value: "Host", label: "Host" },
  { value: "Path", label: "Path" },
  { value: "StatusCode", label: "Status" },
  { value: "ContentType", label: "Content-Type" },
  { value: "Body", label: "Body" },
  { value: "ProcessName", label: "Process" },
  { value: "Scheme", label: "Scheme" },
];

const MATCH_OPERATORS: { value: MatchOperator; label: string }[] = [
  { value: "Contains", label: "contains" },
  { value: "Equals", label: "equals" },
  { value: "NotEquals", label: "not equals" },
  { value: "NotContains", label: "not contains" },
  { value: "StartsWith", label: "starts with" },
  { value: "EndsWith", label: "ends with" },
  { value: "MatchesRegex", label: "regex" },
  { value: "Wildcard", label: "wildcard" },
];

function categoryLabel(c: RuleCategory): string {
  switch (c) {
    case "AutoReply": return "自动回复";
    case "HeaderModifier": return "修改标头";
    case "Redirect": return "重定向";
  }
}

function categoryIcon(c: RuleCategory): string {
  switch (c) {
    case "AutoReply": return "🤖";
    case "HeaderModifier": return "✏️";
    case "Redirect": return "↗️";
  }
}

function actionSummary(rule: Rule): string {
  const a = rule.action;
  if ("AutoReply" in a) {
    const ar = a.AutoReply;
    return `${ar.status_code} ${ar.status_text}`;
  }
  if ("HeaderModifier" in a) {
    const hm = a.HeaderModifier;
    const reqCount = hm.request_actions.length;
    const respCount = hm.response_actions.length;
    return `请求${reqCount}项 / 响应${respCount}项`;
  }
  if ("Redirect" in a) {
    const rd = a.Redirect;
    return `→ ${rd.target_url}`;
  }
  return "";
}

interface RuleEditDialogProps {
  category: RuleCategory;
  onClose: () => void;
  onSave: (rule: Omit<Rule, "id" | "created_at" | "updated_at">) => void;
}

function RuleEditDialog({ category, onClose, onSave }: RuleEditDialogProps) {
  const [name, setName] = useState("");
  const [priority, setPriority] = useState(10);
  const [matchLogic, setMatchLogic] = useState<MatchLogic>("And");
  const [filters, setFilters] = useState<{ field: MatchField; operator: MatchOperator; value: string; case_sensitive: boolean }[]>([]);
  const [statusCode, setStatusCode] = useState(200);
  const [statusText, setStatusText] = useState("OK");
  const [bodyInline, setBodyInline] = useState("");
  const [delayMs, setDelayMs] = useState(0);
  const [targetUrl, setTargetUrl] = useState("");
  const [redirectType, setRedirectType] = useState<RedirectType>("Temporary302");
  const [preserveQuery, setPreserveQuery] = useState(true);
  const [preservePath, setPreservePath] = useState(false);
  const [reqHeaderActions, setReqHeaderActions] = useState<HeaderAction[]>([]);
  const [respHeaderActions, setRespHeaderActions] = useState<HeaderAction[]>([]);
  const [newHeaderName, setNewHeaderName] = useState("");
  const [newHeaderValue, setNewHeaderValue] = useState("");

  const addFilter = () => {
    setFilters([...filters, { field: "Host", operator: "Contains", value: "", case_sensitive: false }]);
  };

  const removeFilter = (idx: number) => {
    setFilters(filters.filter((_, i) => i !== idx));
  };

  const updateFilter = (idx: number, key: string, val: string | boolean) => {
    const updated = [...filters];
    updated[idx] = { ...updated[idx], [key]: val };
    setFilters(updated);
  };

  const addHeaderAction = (target: "req" | "resp") => {
    const action: HeaderAction = { Add: { name: newHeaderName, value: newHeaderValue, only_if_missing: false } };
    if (target === "req") {
      setReqHeaderActions([...reqHeaderActions, action]);
    } else {
      setRespHeaderActions([...respHeaderActions, action]);
    }
    setNewHeaderName("");
    setNewHeaderValue("");
  };

  const removeHeaderAction = (target: "req" | "resp", idx: number) => {
    if (target === "req") {
      setReqHeaderActions(reqHeaderActions.filter((_, i) => i !== idx));
    } else {
      setRespHeaderActions(respHeaderActions.filter((_, i) => i !== idx));
    }
  };

  const buildAction = () => {
    switch (category) {
      case "AutoReply":
        return { AutoReply: { status_code: statusCode, status_text: statusText, headers: [] as [string, string][], body_source: bodyInline ? { Inline: bodyInline } : "Empty" as BodySource, delay_ms: delayMs } as AutoReplyAction };
      case "HeaderModifier":
        return { HeaderModifier: { request_actions: reqHeaderActions, response_actions: respHeaderActions } as HeaderModifierAction };
      case "Redirect":
        return { Redirect: { target_url: targetUrl, redirect_type: redirectType, preserve_query: preserveQuery, preserve_path: preservePath } as RedirectAction };
    }
  };

  const handleSave = () => {
    onSave({
      name: name || categoryLabel(category),
      category,
      enabled: true,
      priority,
      match_condition: { logic: matchLogic, filters },
      action: buildAction(),
    });
    onClose();
  };

  return (
    <div className="rule-edit-overlay" onClick={onClose}>
      <div className="rule-edit-dialog" onClick={(e) => e.stopPropagation()}>
        <h3>{categoryIcon(category)} 新建{categoryLabel(category)}规则</h3>

        <div className="form-group">
          <label>规则名称</label>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="输入规则名称" />
        </div>

        <div className="form-group">
          <label>优先级</label>
          <input type="number" value={priority} onChange={(e) => setPriority(Number(e.target.value))} />
        </div>

        <div className="form-group">
          <label>匹配条件</label>
          <select value={matchLogic} onChange={(e) => setMatchLogic(e.target.value as MatchLogic)}>
            <option value="And">AND</option>
            <option value="Or">OR</option>
          </select>
          {filters.map((f, idx) => (
            <div key={idx} className="filter-row">
              <select value={f.field} onChange={(e) => updateFilter(idx, "field", e.target.value)}>
                {MATCH_FIELDS.map((mf) => <option key={mf.value} value={mf.value}>{mf.label}</option>)}
              </select>
              <select value={f.operator} onChange={(e) => updateFilter(idx, "operator", e.target.value)}>
                {MATCH_OPERATORS.map((mo) => <option key={mo.value} value={mo.value}>{mo.label}</option>)}
              </select>
              <input value={f.value} onChange={(e) => updateFilter(idx, "value", e.target.value)} placeholder="值" />
              <button className="btn-icon" onClick={() => removeFilter(idx)}>×</button>
            </div>
          ))}
          <button className="btn-small" onClick={addFilter}>+ 添加条件</button>
        </div>

        {category === "AutoReply" && (
          <>
            <div className="form-group">
              <label>状态码</label>
              <input type="number" value={statusCode} onChange={(e) => setStatusCode(Number(e.target.value))} />
            </div>
            <div className="form-group">
              <label>状态文本</label>
              <input value={statusText} onChange={(e) => setStatusText(e.target.value)} />
            </div>
            <div className="form-group">
              <label>响应体</label>
              <textarea value={bodyInline} onChange={(e) => setBodyInline(e.target.value)} rows={3} />
            </div>
            <div className="form-group">
              <label>延迟(ms)</label>
              <input type="number" value={delayMs} onChange={(e) => setDelayMs(Number(e.target.value))} />
            </div>
          </>
        )}

        {category === "Redirect" && (
          <>
            <div className="form-group">
              <label>目标URL</label>
              <input value={targetUrl} onChange={(e) => setTargetUrl(e.target.value)} placeholder="https://example.com" />
            </div>
            <div className="form-group">
              <label>重定向类型</label>
              <select value={redirectType} onChange={(e) => setRedirectType(e.target.value as RedirectType)}>
                <option value="Permanent301">301 永久重定向</option>
                <option value="Temporary302">302 临时重定向</option>
                <option value="Temporary307">307 临时重定向(保持方法)</option>
                <option value="Permanent308">308 永久重定向(保持方法)</option>
              </select>
            </div>
            <div className="form-group">
              <label><input type="checkbox" checked={preserveQuery} onChange={(e) => setPreserveQuery(e.target.checked)} /> 保留查询参数</label>
              <label><input type="checkbox" checked={preservePath} onChange={(e) => setPreservePath(e.target.checked)} /> 保留路径</label>
            </div>
          </>
        )}

        {category === "HeaderModifier" && (
          <>
            <div className="form-group">
              <label>请求头操作</label>
              {reqHeaderActions.map((ha, idx) => (
                <div key={idx} className="header-action-row">
                  <span>{ha.Add ? `添加 ${ha.Add.name}: ${ha.Add.value}` : ha.Remove ? `删除 ${ha.Remove.name}` : ha.Replace ? `替换 ${ha.Replace.name}: ${ha.Replace.value}` : ""}</span>
                  <button className="btn-icon" onClick={() => removeHeaderAction("req", idx)}>×</button>
                </div>
              ))}
              <div className="header-add-row">
                <input value={newHeaderName} onChange={(e) => setNewHeaderName(e.target.value)} placeholder="Header名" />
                <input value={newHeaderValue} onChange={(e) => setNewHeaderValue(e.target.value)} placeholder="Header值" />
                <button className="btn-small" onClick={() => addHeaderAction("req")}>+</button>
              </div>
            </div>
            <div className="form-group">
              <label>响应头操作</label>
              {respHeaderActions.map((ha, idx) => (
                <div key={idx} className="header-action-row">
                  <span>{ha.Add ? `添加 ${ha.Add.name}: ${ha.Add.value}` : ha.Remove ? `删除 ${ha.Remove.name}` : ha.Replace ? `替换 ${ha.Replace.name}: ${ha.Replace.value}` : ""}</span>
                  <button className="btn-icon" onClick={() => removeHeaderAction("resp", idx)}>×</button>
                </div>
              ))}
              <div className="header-add-row">
                <input value={newHeaderName} onChange={(e) => setNewHeaderName(e.target.value)} placeholder="Header名" />
                <input value={newHeaderValue} onChange={(e) => setNewHeaderValue(e.target.value)} placeholder="Header值" />
                <button className="btn-small" onClick={() => addHeaderAction("resp")}>+</button>
              </div>
            </div>
          </>
        )}

        <div className="dialog-actions">
          <button className="btn-secondary" onClick={onClose}>取消</button>
          <button className="btn-primary" onClick={handleSave}>保存</button>
        </div>
      </div>
    </div>
  );
}

interface RulesPanelProps {
  onClose: () => void;
}

export function RulesPanel({ onClose }: RulesPanelProps) {
  const { rules, loading, addRule, removeRule, toggleRule, enablePreset, clearRules } = useRules();
  const [activeTab, setActiveTab] = useState<RuleCategory | "All">("All");
  const [editCategory, setEditCategory] = useState<RuleCategory | null>(null);

  const filteredRules = activeTab === "All" ? rules : rules.filter((r) => r.category === activeTab);

  const handlePreset = async (preset: PresetRuleType) => {
    await enablePreset(preset);
  };

  const handleAddRule = async (ruleData: Omit<Rule, "id" | "created_at" | "updated_at">) => {
    await addRule(ruleData as Rule);
  };

  return (
    <div className="rules-panel">
      <div className="rules-header">
        <h2>📋 规则管理</h2>
        <button className="btn-icon" onClick={onClose}>✕</button>
      </div>

      <div className="rules-tabs">
        {(["All", "AutoReply", "HeaderModifier", "Redirect"] as const).map((tab) => (
          <button
            key={tab}
            className={`tab-btn ${activeTab === tab ? "active" : ""}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab === "All" ? "全部" : categoryIcon(tab as RuleCategory) + " " + categoryLabel(tab as RuleCategory)}
          </button>
        ))}
      </div>

      <div className="presets-section">
        <span className="presets-label">⚡ 快捷预设</span>
        <div className="presets-grid">
          {PRESETS.map((p) => (
            <button key={p.key} className="preset-btn" onClick={() => handlePreset(p.key)}>
              {p.icon} {p.label}
            </button>
          ))}
        </div>
      </div>

      <div className="rules-actions">
        <button className="btn-primary" onClick={() => setEditCategory("AutoReply")}>+ 自动回复</button>
        <button className="btn-primary" onClick={() => setEditCategory("HeaderModifier")}>+ 修改标头</button>
        <button className="btn-primary" onClick={() => setEditCategory("Redirect")}>+ 重定向</button>
        {rules.length > 0 && (
          <button className="btn-danger" onClick={clearRules}>清除全部</button>
        )}
      </div>

      <div className="rules-list">
        {loading && <div className="loading">加载中...</div>}
        {!loading && filteredRules.length === 0 && (
          <div className="empty-state">
            <p>暂无规则</p>
            <p className="hint">点击上方预设按钮快速启用，或创建自定义规则</p>
          </div>
        )}
        {filteredRules.map((rule) => (
          <div key={rule.id} className={`rule-card ${rule.enabled ? "" : "disabled"}`}>
            <div className="rule-card-left">
              <span className="rule-icon">{categoryIcon(rule.category)}</span>
              <div className="rule-info">
                <span className="rule-name">{rule.name}</span>
                <span className="rule-detail">
                  {rule.match_condition.filters.length > 0
                    ? rule.match_condition.filters.map((f) => `${f.field} ${f.operator} ${f.value}`).join(" & ")
                    : "匹配所有请求"}
                  {" → "}
                  {actionSummary(rule)}
                </span>
              </div>
            </div>
            <div className="rule-card-right">
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={rule.enabled}
                  onChange={(e) => toggleRule(rule.id, e.target.checked)}
                />
                <span className="toggle-slider"></span>
              </label>
              <button className="btn-icon" onClick={() => removeRule(rule.id)} title="删除">🗑</button>
            </div>
          </div>
        ))}
      </div>

      {editCategory && (
        <RuleEditDialog
          category={editCategory}
          onClose={() => setEditCategory(null)}
          onSave={handleAddRule}
        />
      )}
    </div>
  );
}
