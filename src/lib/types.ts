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
export type AppView = "library" | "import" | "settings";

export type ThemePreference = "dark" | "high-contrast" | "light" | "system";

export type CommandViewMode = "compact" | "cards";

export interface AppSettings {
  theme: ThemePreference;
  commandViewMode: CommandViewMode;
  confirmBeforeDelete: boolean;
  launchAtStartup: boolean;
  closeToTray: boolean;
  aiEnabled: boolean;
  aiModel: string | null;
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
    | "ai_response_too_large";
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
