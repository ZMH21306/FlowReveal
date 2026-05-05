export type MessageDirection = "Request" | "Response";
export type HttpProtocol = "HTTP1_1" | "HTTP2" | "WebSocket";
export type Scheme = "Http" | "Https";
export type CaptureStatus = "Idle" | "Running" | "Error";
export type WsOpcode = "Continuation" | "Text" | "Binary" | "Close" | "Ping" | "Pong";

export interface TlsInfo {
  version: string;
  cipher_suite: string;
  server_name: string | null;
  cert_chain: number[][];
}

export interface Cookie {
  name: string;
  value: string;
  domain: string;
  path: string;
  expires: number | null;
  http_only: boolean;
  secure: boolean;
}

export interface HttpMessage {
  id: number;
  session_id: number;
  direction: MessageDirection;
  protocol: HttpProtocol;
  scheme: Scheme;
  method: string | null;
  url: string | null;
  status_code: number | null;
  status_text: string | null;
  headers: [string, string][];
  body: number[] | null;
  body_size: number;
  body_truncated: boolean;
  content_type: string | null;
  process_name: string | null;
  process_id: number | null;
  process_path: string | null;
  source_ip: string | null;
  dest_ip: string | null;
  source_port: number | null;
  dest_port: number | null;
  timestamp: number;
  duration_us: number | null;
  cookies: Cookie[];
  raw_tls_info: TlsInfo | null;
  stream_id: number | null;
}

export interface HttpSession {
  id: number;
  request: HttpMessage;
  response: HttpMessage | null;
  created_at: number;
  completed_at: number | null;
}

export interface WebSocketFrame {
  id: number;
  session_id: number;
  direction: MessageDirection;
  opcode: WsOpcode;
  payload: number[] | null;
  payload_size: number;
  payload_truncated: boolean;
  timestamp: number;
}

export interface CaptureConfig {
  mode: "Global" | "ProxyOnly";
  capture_http: boolean;
  capture_https: boolean;
  ports: number[];
  process_filters: string[];
  host_filters: string[];
  max_body_size: number;
  ca_cert_path: string | null;
  ca_key_path: string | null;
  mitm_bypass_hosts: string[];
  proxy_port: number;
  transparent_proxy_port: number;
  capture_ports: number[];
  exclude_pids: number[];
  include_pids: number[];
  capture_localhost: boolean;
}

export type DiverterStatus = "NotAvailable" | "Stopped" | "Running" | "Error";

export interface EngineStats {
  total_sessions: number;
  active_sessions: number;
  bytes_captured: number;
  tls_handshakes: number;
  http1_requests: number;
  http2_requests: number;
  ws_frames: number;
  filtered_out: number;
}

export interface CertificateAuthority {
  cert_pem: string;
  key_pem: string;
  serial_number: number;
  not_before: number;
  not_after: number;
  is_installed: boolean;
}

export type RuleCategory = "AutoReply" | "HeaderModifier" | "Redirect";
export type MatchField = "Method" | "Url" | "Host" | "Path" | "StatusCode" | "ContentType" | "HeaderName" | "HeaderValue" | "Body" | "ProcessName" | "Scheme" | "QueryParam";
export type MatchOperator = "Equals" | "NotEquals" | "Contains" | "NotContains" | "StartsWith" | "EndsWith" | "MatchesRegex" | "GreaterThan" | "LessThan" | "InRange" | "Wildcard";
export type MatchLogic = "And" | "Or";
export type PresetRuleType = "CorsEnable" | "CacheDisable" | "CookiesRemove" | "ServiceUnavailable503" | "Redirect302" | "Ok200";
export type RedirectType = "Permanent301" | "Temporary302" | "Temporary307" | "Permanent308";
export type BodySource = { Inline: string } | { File: string } | "Empty";

export interface MatchFilter {
  field: MatchField;
  operator: MatchOperator;
  value: string;
  case_sensitive: boolean;
}

export interface MatchCondition {
  logic: MatchLogic;
  filters: MatchFilter[];
}

export interface AutoReplyAction {
  status_code: number;
  status_text: string;
  headers: [string, string][];
  body_source: BodySource;
  delay_ms: number;
}

export interface HeaderAction {
  Add?: { name: string; value: string; only_if_missing: boolean };
  Remove?: { name: string };
  Replace?: { name: string; value: string };
  ReplaceRegex?: { name: string; pattern: string; replacement: string };
}

export interface HeaderModifierAction {
  request_actions: HeaderAction[];
  response_actions: HeaderAction[];
}

export interface RedirectAction {
  target_url: string;
  redirect_type: RedirectType;
  preserve_query: boolean;
  preserve_path: boolean;
}

export type RuleAction =
  | { AutoReply: AutoReplyAction }
  | { HeaderModifier: HeaderModifierAction }
  | { Redirect: RedirectAction };

export interface Rule {
  id: number;
  name: string;
  category: RuleCategory;
  enabled: boolean;
  priority: number;
  match_condition: MatchCondition;
  action: RuleAction;
  created_at: number;
  updated_at: number;
}
