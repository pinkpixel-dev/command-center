import type { CommandInput, CommandKind, RiskLevel } from "./types";
import { emptyCommandInput } from "./types";

/** What the Rust parser hands back for one proposed entry. */
export interface ImportCandidate {
  id: string;
  title: string;
  content: string;
  description: string;
  kind: CommandKind;
  language: string | null;
  shell: string | null;
  tags: string[];
  riskLevel: RiskLevel;
  /** Reasons the local rules gave. */
  riskReasons: string[];
  /** Reasons the model gave, kept separate so the review screen can label them. */
  aiRiskReasons: string[];
  /** What the model said it was unsure about. */
  aiNotes: string | null;
  variables: string[];
  headingPath: string[];
  sourceLine: number;
  looksLikeOutput: boolean;
  outputReason: string | null;
  droppedOutputLines: number;
  duplicate: DuplicateMatch | null;
  repeatedInDocument: boolean;
  selected: boolean;
}

export interface DuplicateMatch {
  id: number;
  title: string;
}

export interface PreviewStats {
  blocksFound: number;
  commands: number;
  outputBlocks: number;
  duplicates: number;
  repeated: number;
}

export interface ImportPreview {
  sourceName: string | null;
  suggestedCollection: string | null;
  candidates: ImportCandidate[];
  stats: PreviewStats;
}

/** Recalculated facts after an edit, split, or merge. */
export interface SnippetAnalysis {
  kind: CommandKind;
  riskLevel: RiskLevel;
  riskReasons: string[];
  variables: string[];
  looksLikeOutput: boolean;
  outputReason: string | null;
  duplicate: DuplicateMatch | null;
  suggestedTitle: string;
}

export type DuplicateAction = "create" | "skip" | "replace" | "merge";

export interface ImportItem {
  input: CommandInput;
  duplicateAction: DuplicateAction;
  existingId: number | null;
}

export interface ImportFailure {
  title: string;
  message: string;
}

export interface ImportSummary {
  created: number;
  replaced: number;
  merged: number;
  skipped: number;
  failures: ImportFailure[];
}

/** A candidate plus the choices the user has made about it. */
export interface CandidateDraft extends ImportCandidate {
  duplicateAction: DuplicateAction;
  collectionIds: number[];
}

/** Duplicates default to skipping, which is the safe answer. */
export function toDrafts(
  candidates: ImportCandidate[],
  collectionIds: number[] = [],
): CandidateDraft[] {
  return candidates.map((candidate) => ({
    ...candidate,
    duplicateAction: candidate.duplicate ? "skip" : "create",
    collectionIds: [...collectionIds],
  }));
}

/**
 * Splits a multi-line candidate into one draft per line, keeping the metadata
 * that still applies. The caller re-analyses each piece, because kind, risk,
 * and duplicate status all change once the lines stand alone.
 */
export function splitCandidate(draft: CandidateDraft): CandidateDraft[] {
  const lines = draft.content
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (lines.length < 2) return [draft];

  return lines.map((line, index) => ({
    ...draft,
    id: `${draft.id}-split-${index}`,
    title: index === 0 ? draft.title : line,
    content: line,
    // Only the first piece keeps the shared description; repeating it on every
    // line would be noise.
    description: index === 0 ? draft.description : "",
    kind: "command",
    variables: [],
    duplicate: null,
    duplicateAction: "create",
    repeatedInDocument: false,
    droppedOutputLines: 0,
    // The model described the whole block, not this one line.
    aiRiskReasons: [],
    aiNotes: null,
    selected: draft.selected,
  }));
}

/** Folds the second draft into the first: contents stacked, tags unioned. */
export function mergeCandidates(first: CandidateDraft, second: CandidateDraft): CandidateDraft {
  const tags = [...first.tags];
  for (const tag of second.tags) {
    if (!tags.includes(tag)) tags.push(tag);
  }

  const collectionIds = [...first.collectionIds];
  for (const id of second.collectionIds) {
    if (!collectionIds.includes(id)) collectionIds.push(id);
  }

  return {
    ...first,
    content: `${first.content.trimEnd()}\n${second.content.trim()}`,
    description: first.description || second.description,
    tags,
    collectionIds,
    kind: "sequence",
    duplicate: null,
    duplicateAction: "create",
    variables: [],
    droppedOutputLines: first.droppedOutputLines + second.droppedOutputLines,
    riskReasons: [],
    // Neither description covers the combined entry any more.
    aiRiskReasons: [],
    aiNotes: null,
  };
}

/** Applies a fresh analysis on top of a draft the user just changed. */
export function withAnalysis(draft: CandidateDraft, analysis: SnippetAnalysis): CandidateDraft {
  return {
    ...draft,
    kind: analysis.kind,
    riskLevel: analysis.riskLevel,
    riskReasons: analysis.riskReasons,
    // The content changed, so anything the model said about it is stale.
    aiRiskReasons: [],
    aiNotes: null,
    variables: analysis.variables,
    looksLikeOutput: analysis.looksLikeOutput,
    outputReason: analysis.outputReason,
    duplicate: analysis.duplicate,
    duplicateAction: analysis.duplicate ? "skip" : "create",
    title: draft.title.trim() ? draft.title : analysis.suggestedTitle,
  };
}

/** Builds the payload for the selected drafts only. */
export function buildImportItems(drafts: CandidateDraft[]): ImportItem[] {
  return drafts
    .filter((draft) => draft.selected)
    .map((draft) => ({
      input: emptyCommandInput({
        title: draft.title,
        content: draft.content,
        description: draft.description,
        kind: draft.kind,
        language: draft.language,
        shell: draft.shell,
        tags: draft.tags,
        collectionIds: draft.collectionIds,
      }),
      duplicateAction: draft.duplicate ? draft.duplicateAction : "create",
      existingId: draft.duplicate?.id ?? null,
    }));
}

export interface DraftCounts {
  total: number;
  selected: number;
  duplicates: number;
  output: number;
  willCreate: number;
  willSkip: number;
}

export function countDrafts(drafts: CandidateDraft[]): DraftCounts {
  const selected = drafts.filter((draft) => draft.selected);
  return {
    total: drafts.length,
    selected: selected.length,
    duplicates: drafts.filter((draft) => draft.duplicate !== null).length,
    output: drafts.filter((draft) => draft.looksLikeOutput).length,
    willSkip: selected.filter((draft) => draft.duplicate && draft.duplicateAction === "skip").length,
    willCreate: selected.filter((draft) => !draft.duplicate || draft.duplicateAction !== "skip")
      .length,
  };
}

/** One-line description of what an import did. */
export function summarize(summary: ImportSummary): string {
  const parts: string[] = [];
  if (summary.created > 0) parts.push(`${summary.created} added`);
  if (summary.replaced > 0) parts.push(`${summary.replaced} replaced`);
  if (summary.merged > 0) parts.push(`${summary.merged} merged`);
  if (summary.skipped > 0) parts.push(`${summary.skipped} skipped`);
  if (summary.failures.length > 0) parts.push(`${summary.failures.length} failed`);
  return parts.length > 0 ? parts.join(", ") : "Nothing to import";
}
