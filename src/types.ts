export type HookSource = "claude" | "codex" | "gemini" | "generic";
export type EventStatus =
  | "idle"
  | "working"
  | "notification"
  | "waitingApproval"
  | "waitingInput"
  | "completed"
  | "error";

export interface QuestionOption {
  id: string;
  label: string;
  description?: string;
}

export interface HookQuestion {
  id: string;
  prompt: string;
  options: QuestionOption[];
  multiSelect: boolean;
}

export interface HookEvent {
  id: string;
  source: HookSource;
  providerName: string;
  eventType: string;
  sessionKey: string;
  project: string;
  cwd?: string;
  title: string;
  body: string;
  url?: string;
  status: EventStatus;
  toolName?: string;
  toolUseId?: string;
  questions: HookQuestion[];
  expectsResponse: boolean;
  shouldNotify: boolean;
  receivedAt: string;
  resolvedAt?: string;
  resolution?: string;
}

export interface AppSettings {
  notificationsEnabled: boolean;
  showPanelOnAttention: boolean;
  openAtLogin: boolean;
  playSound: boolean;
  language: "auto" | "zh-CN" | "en";
}

export interface SourceProfile {
  id: HookSource;
  name: string;
  icon: string;
  accentColor: string;
  showToast: boolean;
  playSound: boolean;
  autoExpand: boolean;
  enabled: boolean;
}

export interface HookInstallState {
  title: string;
  installed: boolean;
  filePath: string;
  error?: string;
}

export interface AppState {
  listening: boolean;
  compact: boolean;
  port?: number;
  events: HookEvent[];
  pendingEventIds: string[];
  selectedEventId?: string;
  settings: AppSettings;
  sourceProfiles: SourceProfile[];
  hookStatus: Partial<Record<HookSource, HookInstallState>>;
  genericCommand: string;
  platform: string;
  version: string;
}

export interface HookResponse {
  decision: "approve" | "approveForSession" | "deny" | "answer" | "cancel";
  reason?: string;
  answers?: Record<string, string>;
}

export interface OperationResult {
  ok: boolean;
  error?: string;
}

export interface HookDockBridge {
  getState(): Promise<AppState>;
  onState(callback: (state: AppState) => void): () => void;
  respond(id: string, response: HookResponse): Promise<OperationResult>;
  dismissEvent(id: string): Promise<OperationResult>;
  updateSettings(patch: Partial<AppSettings>): Promise<AppSettings>;
  updateSourceProfile(id: HookSource, patch: Partial<Omit<SourceProfile, "id">>): Promise<SourceProfile[]>;
  installHooks(sources: HookSource[]): Promise<OperationResult>;
  uninstallHooks(sources: HookSource[]): Promise<OperationResult>;
  copyGenericCommand(): Promise<OperationResult>;
  sendTestEvent(): Promise<OperationResult>;
  openFolder(path: string): Promise<OperationResult>;
  openUrl(url: string): Promise<OperationResult>;
  setCompact(compact: boolean): Promise<OperationResult>;
  snapWindow(): Promise<OperationResult>;
  hideWindow(): void;
}
