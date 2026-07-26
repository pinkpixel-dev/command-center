import type { CommandKind, RiskLevel, Scope } from "./types";

const KIND_LABELS: Record<CommandKind, string> = {
  command: "Command",
  script: "Script",
  sequence: "Sequence",
  snippet: "Snippet",
  reference: "Reference",
};

const RISK_LABELS: Record<RiskLevel, string> = {
  safe: "Safe",
  caution: "Caution",
  destructive: "Destructive",
};

export function kindLabel(kind: CommandKind): string {
  return KIND_LABELS[kind];
}

export function riskLabel(risk: RiskLevel): string {
  return RISK_LABELS[risk];
}

/** "3 minutes ago", "yesterday", "12 Mar" - short enough for a card. */
export function relativeTime(iso: string | null, now: Date = new Date()): string {
  if (!iso) return "never";

  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) return "unknown";

  const seconds = Math.round((now.getTime() - then.getTime()) / 1000);
  if (seconds < 45) return "just now";

  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;

  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} ${hours === 1 ? "hour" : "hours"} ago`;

  const days = Math.round(hours / 24);
  if (days === 1) return "yesterday";
  if (days < 30) return `${days} days ago`;

  return then.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: then.getFullYear() === now.getFullYear() ? undefined : "numeric",
  });
}

export function scopeTitle(scope: Scope): string {
  switch (scope.type) {
    case "all":
      return "All commands";
    case "favorites":
      return "Favorites";
    case "recent":
      return "Recently used";
    case "scripts":
      return "Scripts";
    case "collection":
      return "Collection";
    case "tag":
      return `#${scope.name}`;
  }
}

export function scopesEqual(left: Scope, right: Scope): boolean {
  if (left.type !== right.type) return false;
  if (left.type === "collection" && right.type === "collection") return left.id === right.id;
  if (left.type === "tag" && right.type === "tag") return left.name === right.name;
  return true;
}

/** First line of a command, for collapsed previews. */
export function firstLine(content: string): string {
  const line = content.split("\n").find((candidate) => candidate.trim().length > 0) ?? "";
  return line.trim();
}

export function lineCount(content: string): number {
  return content.split("\n").filter((line) => line.trim().length > 0).length;
}

/**
 * Fills `{{placeholders}}` with the values a user typed. Missing values keep
 * their placeholder so nothing silently becomes an empty string.
 */
export function renderTemplate(content: string, values: Record<string, string>): string {
  return content.replace(/\{\{\s*([\w.-]+)\s*\}\}/g, (match, name: string) => {
    const value = values[name];
    return value !== undefined && value !== "" ? value : match;
  });
}

/** Accelerator strings look better as "Ctrl + Shift + Space" in the UI. */
export function prettyShortcut(accelerator: string, isMac = /mac/i.test(navigator.platform)): string {
  return accelerator
    .split("+")
    .map((part) => {
      const key = part.trim();
      if (key === "CommandOrControl" || key === "CmdOrCtrl") return isMac ? "Cmd" : "Ctrl";
      if (key === "Control") return "Ctrl";
      if (key === "Meta" || key === "Super") return isMac ? "Cmd" : "Super";
      return key;
    })
    .join(" + ");
}
