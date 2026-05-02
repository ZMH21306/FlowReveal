export type MessageDirection = "Request" | "Response";
export type HttpProtocol = "HTTP1_1" | "HTTP2" | "WebSocket";
export type Scheme = "Http" | "Https";
export type CaptureMode = "ForwardProxy" | "TransparentProxy" | "ApiHook";
export type CaptureStatus = "Idle" | "Running" | "Error";
export type WsOpcode = "Continuation" | "Text" | "Binary" | "Close" | "Ping" | "Pong";
export type TransportProtocol = "Tcp" | "Udp";
export type FilterDirection = "Request" | "Response" | "Both";
export type FilterField = "Method" | "Url" | "Host" | "StatusCode" | "ContentType" | "HeaderName" | "HeaderValue" | "Body" | "ProcessName" | "Scheme";
export type FilterOperator = "Equals" | "NotEquals" | "Contains" | "NotContains" | "StartsWith" | "EndsWith" | "Matches" | "GreaterThan" | "LessThan";
export type FilterLogic = "And" | "Or";
export type HookTarget = "WinHttpSendRequest" | "WinHttpReceiveResponse" | "WinHttpWriteData" | "WinHttpReadData" | "WinHttpCloseHandle" | "SslEncryptPacket" | "SslDecryptPacket";
export type HookDirection = "Outgoing" | "Incoming";

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
  mode: CaptureMode;
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
}

export interface CaptureFilter {
  direction: FilterDirection;
  field: FilterField;
  operator: FilterOperator;
  value: string;
}

export interface FilterGroup {
  logic: FilterLogic;
  filters: FilterGroupItem[];
}

export type FilterGroupItem = { Filter: CaptureFilter } | { Group: FilterGroup };

export interface EngineStats {
  total_sessions: number;
  active_sessions: number;
  bytes_captured: number;
  tls_handshakes: number;
  hook_injections: number;
  http1_requests: number;
  http2_requests: number;
  ws_frames: number;
  filtered_out: number;
}

export interface EngineCapabilities {
  supports_wfp: boolean;
  supports_api_hook: boolean;
  supports_tls_mitm: boolean;
  supports_http2: boolean;
  supports_websocket: boolean;
  process_identification: boolean;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  path: string | null;
  command_line: string | null;
  icon_data: number[] | null;
}

export interface ConnectionInfo {
  local_addr: string;
  local_port: number;
  remote_addr: string;
  remote_port: number;
  protocol: TransportProtocol;
  state: string;
  owning_pid: number;
}

export interface CertificateAuthority {
  cert_pem: string;
  key_pem: string;
  serial_number: number;
  not_before: number;
  not_after: number;
  is_installed: boolean;
}

export interface MitmConfig {
  enabled: boolean;
  bypass_hosts: string[];
  bypass_port_ranges: [number, number][];
  max_cert_cache_entries: number;
  cert_validity_days: number;
}

export interface HookPacket {
  target: HookTarget;
  direction: HookDirection;
  process_id: number;
  thread_id: number;
  local_port: number;
  remote_addr: string;
  remote_port: number;
  data: number[];
  sequence: number;
  timestamp: number;
}
