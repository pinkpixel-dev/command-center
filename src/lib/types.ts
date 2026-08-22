/** Mirrors the serde payloads coming out of the Rust layer. */

export type CommandKind = "command" | "script" | "sequence" | "snippet" | "reference";

export type RiskLevel = "safe" | "caution" | "destructive";

export const COMMAND_KINDS: readonly CommandKind[] = [
  "command",
  "script",
  "sequence",
  "snippet",
  "reference",
];

export const RISK_LEVELS: readonly RiskLevel[] = ["safe", "caution", "destructive"];

export interface CollectionRef {
  id: number;
  name: string;
}

export interface CommandEntry {
  id: number;
  title: string;
  content: string;
  description: string;
  kind: CommandKind;
  language: string | null;
  shell: string | null;
  operatingSystem: string | null;
  riskLevel: RiskLevel;
  riskReasons: string[];
  favorite: boolean;
  workingDirectory: string | null;
  sourceUrl: string | null;
  notes: string;
  contentHash: string;
  copyCount: number;
  lastCopiedAt: string | null;
  createdAt: string;
  updatedAt: string;
  tags: string[];
  collections: CollectionRef[];
  variables: string[];
}

export interface CommandInput {
  title: string;
  content: string;
  description: string;
  kind: CommandKind;
  language: string | null;
  shell: string | null;
  operatingSystem: string | null;
  riskLevel: RiskLevel | null;
  favorite: boolean;
  workingDirectory: string | null;
  sourceUrl: string | null;
  notes: string;
  tags: string[];
  collectionIds: number[];
}

export interface Tag {
  id: number;
  name: string;
  commandCount: number;
}

export interface Collection {
  id: number;
  name: string;
  description: string;
  commandCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface CollectionInput {
  name: string;
  description: string;
}

export interface LibraryStats {
  total: number;
  favorites: number;
  scripts: number;
  recent: number;
}

export type Scope =
  | { type: "all" }
  | { type: "favorites" }
  | { type: "recent" }
  | { type: "scripts" }
  | { type: "collection"; id: number }
  | { type: "tag"; name: string };

export type SortOrder = "updated" | "created" | "title" | "copies" | "lastCopied";

export interface ListQuery {
  search?: string | null;
  scope?: Scope;
  tags?: string[];
  kinds?: CommandKind[];
  risk?: RiskLevel | null;
  sort?: SortOrder;
  limit?: number | null;
}

/** Which top-level screen the main window is showing. */
export type AppView = "library" | "collections" | "tags" | "import" | "settings";

export type ThemePreference = "dark" | "high-contrast" | "light" | "system";

export type CommandViewMode = "compact" | "cards";

/** Which provider the AI workflows use. Never interchangeable. */
export type AiProvider = "openaiApi" | "chatgptCodex";

export interface AppSettings {
  theme: ThemePreference;
  commandViewMode: CommandViewMode;
  confirmBeforeDelete: boolean;
  launchAtStartup: boolean;
  closeToTray: boolean;
  aiEnabled: boolean;
  aiModel: string | null;
  aiProvider: AiProvider;
  /** Kept apart from aiModel: the two catalogues are not the same. */
  codexModel: string | null;
  /** An explicit Codex path. Null means the app finds it. */
  codexPath: string | null;
}

/** Why a found Codex cannot be used as it stands. */
export type CodexUnusableReason =
  | "missing"
  | "notExecutable"
  | "shimNotSupported"
  | "noVersion"
  | "timeout";

/** Whether Codex itself is present and usable, before any account exists. */
export type CodexAvailability =
  | { state: "ready"; version: string }
  | { state: "notFound" }
  | { state: "tooOld"; version: string; minimum: string }
  | { state: "unusable"; reason: CodexUnusableReason }
  | { state: "unsupportedPlatform" };

/**
 * The account in Command Center's own Codex home. `plan` is display metadata
 * only; entitlements are enforced upstream and must never be inferred here.
 */
export type CodexAccount =
  | { state: "notConnected" }
  | { state: "connected"; email: string | null; plan: string | null }
  | { state: "connectedWithOtherCredentials"; kind: string };

/** What a started sign-in needs from the user next. */
export type CodexLoginPrompt =
  | { mode: "browser" }
  | { mode: "deviceCode"; verificationUrl: string; userCode: string };

export interface CodexModel {
  id: string;
  displayName: string;
  isDefault: boolean;
}

export interface CodexStatus {
  availability: CodexAvailability;
  account: CodexAccount;
  accountError: string | null;
  diagnostics: string[];
}

export interface AiStatus {
  keyStored: boolean;
  credentialManagerAvailable: boolean;
  defaultModel: string;
  effectiveModel: string;
  models: string[];
}

export interface AiKeyStatus {
  keyStored: boolean;
}

export interface AiConnectionResult {
  model: string;
}

export interface ExplanationFlag {
  flag: string;
  meaning: string;
}

export interface ExplanationStage {
  stage: string;
  purpose: string;
}

/** The effective level is the stricter of the local verdict and the model's. */
export interface ExplanationSafety {
  level: RiskLevel;
  localReasons: string[];
  aiReasons: string[];
}

/** A safer way to preview the command, already checked by the local rules. */
export interface ExplanationPreview {
  command: string;
  riskLevel: RiskLevel;
  riskReasons: string[];
}

export interface Explanation {
  summary: string;
  flags: ExplanationFlag[];
  pipeline: ExplanationStage[];
  sideEffects: string[];
  safety: ExplanationSafety;
  previewCommand: ExplanationPreview | null;
  assumptions: string[];
  caveats: string[];
}

export interface ExplanationView {
  commandId: number;
  explanation: Explanation;
  model: string;
  /** The entry changed after this explanation was written. */
  stale: boolean;
  generatedAt: string;
}

/** A command the assistant suggested, already checked by the local rules. */
export interface CommandProposal {
  command: string;
  title: string;
  why: string;
  kind: CommandKind;
  shell: string | null;
  /** The stricter of the local verdict and the model's suggestion. */
  riskLevel: RiskLevel;
  localReasons: string[];
  aiReasons: string[];
}

export interface AssistantReply {
  reply: string;
  proposals: CommandProposal[];
}

/** How much the pasted output actually supports the reading. */
export type AnalysisConfidence = "high" | "medium" | "low";

/** A line from the paste that carries part of the diagnosis. */
export interface RelevantLine {
  line: number | null;
  quote: string;
  why: string;
}

/** Something to do next, and what doing it would tell you. */
export interface SuggestedCheck {
  check: string;
  why: string;
}

/** One reading of pasted terminal output, already risk-checked locally. */
export interface ErrorAnalysis {
  summary: string;
  cause: string;
  confidence: AnalysisConfidence;
  relevantLines: RelevantLine[];
  checks: SuggestedCheck[];
  proposals: CommandProposal[];
  /** What the model would need to see to be sure. Empty when nothing. */
  uncertainty: string;
}

export type TargetShell = "bash" | "fish" | "zsh" | "powershell";

/**
 * How much of the original behaviour survived the rewrite. There is
 * deliberately no value meaning "exact": a conversion is never presented as a
 * guaranteed equivalent.
 */
export type Equivalence = "close" | "partial" | "uncertain";

/** A shell the backend will convert to, named by the backend. */
export interface ShellOption {
  id: TargetShell;
  label: string;
}

export interface ShellConversion {
  sourceShell: string | null;
  targetShell: TargetShell;
  original: string;
  /** The rewrite, already checked by the local rules. */
  converted: CommandProposal;
  equivalence: Equivalence;
  differences: string[];
  unsupported: string[];
  notes: string;
}

/** One earlier message, resent so a follow-up has something to refer to. */
export interface AssistantTurn {
  role: "user" | "assistant";
  text: string;
  commands: string[];
}

export interface AssistantAsk {
  requestId: number;
  commandId: number | null;
  /**
   * Terminal output an analysis already ran on, resent so a follow-up can
   * still refer to it. It is never stored, here or anywhere else.
   */
  errorOutput: string | null;
  turns: AssistantTurn[];
  message: string;
}

/** Shape of a rejected `invoke` call. */
export interface AppErrorPayload {
  kind:
    | "database"
    | "invalid"
    | "not_found"
    | "runtime"
    | "credential"
    | "ai_disabled"
    | "ai_not_configured"
    | "ai_auth"
    | "ai_model"
    | "ai_rate_limit"
    | "ai_network"
    | "ai_response"
    | "ai_refusal"
    | "ai_incomplete"
    | "ai_malformed"
    | "ai_response_too_large"
    | "ai_cancelled";
  message: string;
}

/** A blank entry used by the command editor. */
export function emptyCommandInput(overrides: Partial<CommandInput> = {}): CommandInput {
  return {
    title: "",
    content: "",
    description: "",
    kind: "command",
    language: null,
    shell: null,
    operatingSystem: null,
    riskLevel: null,
    favorite: false,
    workingDirectory: null,
    sourceUrl: null,
    notes: "",
    tags: [],
    collectionIds: [],
    ...overrides,
  };
}

/** Turns a stored entry back into an editable input. */
export function toCommandInput(entry: CommandEntry): CommandInput {
  return {
    title: entry.title,
    content: entry.content,
    description: entry.description,
    kind: entry.kind,
    language: entry.language,
    shell: entry.shell,
    operatingSystem: entry.operatingSystem,
    riskLevel: entry.riskLevel,
    favorite: entry.favorite,
    workingDirectory: entry.workingDirectory,
    sourceUrl: entry.sourceUrl,
    notes: entry.notes,
    tags: [...entry.tags],
    collectionIds: entry.collections.map((collection) => collection.id),
  };
}
