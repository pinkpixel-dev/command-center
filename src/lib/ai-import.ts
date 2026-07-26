/** The disclosure the backend builds before anything is sent to OpenAI. */
export interface AiImportPlan {
  documentBytes: number;
  sentBytes: number;
  lineCount: number;
  model: string;
  findings: PlannedRedaction[];
}

/** A likely secret, described by where it is rather than by what it says. */
export interface PlannedRedaction {
  kind: string;
  placeholder: string;
  line: number;
}

/** A document read by Rust so the webview never touches the filesystem. */
export interface ImportDocument {
  name: string | null;
  content: string;
}

const REDACTION_LABELS: Record<string, string> = {
  private_key: "Private key",
  embedded_credentials: "Credentials in a URL",
  openai_api_key: "OpenAI API key",
  github_token: "GitHub token",
  aws_access_key_id: "AWS access key ID",
  bearer_token: "Bearer token",
  password: "Password",
  api_key: "API key",
  token: "Token or secret",
};

export function redactionLabel(kind: string): string {
  return REDACTION_LABELS[kind] ?? "Possible secret";
}

/** Sizes people can picture, without pretending 900 bytes is a kilobyte. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** The one sentence that has to be true: this much text leaves the machine. */
export function describeOutbound(plan: AiImportPlan): string {
  return `This sends ${formatBytes(plan.sentBytes)} of text to OpenAI using ${plan.model}.`;
}

/** Null when nothing was found, so the caller can leave the row out entirely. */
export function describeRedactions(plan: AiImportPlan): string | null {
  const count = plan.findings.length;
  if (count === 0) return null;
  return count === 1
    ? "1 likely secret was replaced with a placeholder."
    : `${count} likely secrets were replaced with placeholders.`;
}

/** Groups findings so a document with 40 tokens does not print 40 rows. */
export function groupRedactions(
  plan: AiImportPlan,
): { kind: string; label: string; placeholder: string; lines: number[] }[] {
  const groups = new Map<string, { kind: string; label: string; placeholder: string; lines: number[] }>();

  for (const finding of plan.findings) {
    const existing = groups.get(finding.kind);
    if (existing) {
      existing.lines.push(finding.line);
      continue;
    }
    groups.set(finding.kind, {
      kind: finding.kind,
      label: redactionLabel(finding.kind),
      placeholder: finding.placeholder,
      lines: [finding.line],
    });
  }

  return [...groups.values()];
}
