import { useCallback, useMemo, useState } from "react";

import {
  buildImportItems,
  countDrafts,
  mergeCandidates,
  splitCandidate,
  toDrafts,
  withAnalysis,
} from "../lib/import";
import type {
  CandidateDraft,
  ImportPreview,
  ImportSummary,
} from "../lib/import";
import type { AiImportPlan } from "../lib/ai-import";
import { api, toAppError } from "../lib/ipc";
import type { ImportDocumentReader } from "../lib/platform";

export type ImportStage = "source" | "disclose" | "review" | "done";

/** The document waiting at the disclosure step, still on this machine. */
export interface PendingDocument {
  content: string;
  sourceName: string | null;
}

export interface ImportSession {
  stage: ImportStage;
  busy: boolean;
  error: string | null;
  plan: AiImportPlan | null;
  pending: PendingDocument | null;
  preview: ImportPreview | null;
  drafts: CandidateDraft[];
  summary: ImportSummary | null;
  counts: ReturnType<typeof countDrafts>;

  /** Reads a document and builds the outbound-request summary. Local only. */
  prepareText: (content: string) => Promise<void>;
  prepareFile: (read: ImportDocumentReader) => Promise<void>;
  /** The one action that sends the document to OpenAI. */
  send: () => Promise<void>;
  cancelSend: () => void;
  update: (id: string, changes: Partial<CandidateDraft>) => void;
  /** Content changed, so risk, kind, and duplicate status need recalculating. */
  reanalyze: (id: string) => Promise<void>;
  setSelectionForAll: (selected: boolean) => void;
  setCollectionForAll: (collectionIds: number[]) => void;
  split: (id: string) => Promise<void>;
  mergeWithPrevious: (id: string) => Promise<void>;
  remove: (id: string) => void;
  runImport: () => Promise<void>;
  reset: () => void;
  dismissError: () => void;
}

/** Holds one trip through the importer: read, disclose, send, review, import. */
export function useImportSession(defaultCollectionIds: number[] = []): ImportSession {
  const [stage, setStage] = useState<ImportStage>("source");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [plan, setPlan] = useState<AiImportPlan | null>(null);
  const [pending, setPending] = useState<PendingDocument | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [drafts, setDrafts] = useState<CandidateDraft[]>([]);
  const [summary, setSummary] = useState<ImportSummary | null>(null);

  const prepare = useCallback(async (reader: () => Promise<PendingDocument>) => {
    setBusy(true);
    setError(null);
    try {
      const document = await reader();
      // Redaction and the size check happen in Rust before anything is sent,
      // so a rejected document never reaches the disclosure step.
      const summary = await api.prepareAiImport(document.content);
      setPending(document);
      setPlan(summary);
      setStage("disclose");
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  }, []);

  const prepareText = useCallback(
    (content: string) => prepare(async () => ({ content, sourceName: null })),
    [prepare],
  );

  const prepareFile = useCallback(
    (read: ImportDocumentReader) =>
      prepare(async () => {
        const document = await read();
        return { content: document.content, sourceName: document.name };
      }),
    [prepare],
  );

  const send = useCallback(async () => {
    if (!pending) return;

    setBusy(true);
    setError(null);
    try {
      const result = await api.runAiImport(pending.content, pending.sourceName);
      setPreview(result);
      setDrafts(toDrafts(result.candidates, defaultCollectionIds));
      setStage("review");
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  }, [defaultCollectionIds, pending]);

  const cancelSend = useCallback(() => {
    setPlan(null);
    setError(null);
    setStage("source");
  }, []);

  const update = useCallback((id: string, changes: Partial<CandidateDraft>) => {
    setDrafts((current) =>
      current.map((draft) => (draft.id === id ? { ...draft, ...changes } : draft)),
    );
  }, []);

  const analyzeInto = useCallback(async (draft: CandidateDraft): Promise<CandidateDraft> => {
    try {
      const analysis = await api.analyzeSnippet(draft.content);
      return withAnalysis(draft, analysis);
    } catch {
      // A failed re-analysis leaves the draft as the user typed it; the backend
      // classifies it again on import anyway.
      return draft;
    }
  }, []);

  const reanalyze = useCallback(
    async (id: string) => {
      const target = drafts.find((draft) => draft.id === id);
      if (!target) return;
      const analyzed = await analyzeInto(target);
      setDrafts((current) =>
        current.map((draft) => (draft.id === id ? { ...analyzed, title: draft.title } : draft)),
      );
    },
    [analyzeInto, drafts],
  );

  const setSelectionForAll = useCallback((selected: boolean) => {
    setDrafts((current) => current.map((draft) => ({ ...draft, selected })));
  }, []);

  const setCollectionForAll = useCallback((collectionIds: number[]) => {
    setDrafts((current) => current.map((draft) => ({ ...draft, collectionIds })));
  }, []);

  const split = useCallback(
    async (id: string) => {
      const target = drafts.find((draft) => draft.id === id);
      if (!target) return;

      const pieces = splitCandidate(target);
      if (pieces.length < 2) return;

      const analyzed = await Promise.all(pieces.map((piece) => analyzeInto(piece)));
      setDrafts((current) => {
        const index = current.findIndex((draft) => draft.id === id);
        if (index === -1) return current;
        return [...current.slice(0, index), ...analyzed, ...current.slice(index + 1)];
      });
    },
    [analyzeInto, drafts],
  );

  const mergeWithPrevious = useCallback(
    async (id: string) => {
      const index = drafts.findIndex((draft) => draft.id === id);
      if (index < 1) return;

      const merged = mergeCandidates(drafts[index - 1], drafts[index]);
      const analyzed = await analyzeInto(merged);
      setDrafts((current) => {
        const position = current.findIndex((draft) => draft.id === id);
        if (position < 1) return current;
        return [
          ...current.slice(0, position - 1),
          { ...analyzed, title: merged.title },
          ...current.slice(position + 1),
        ];
      });
    },
    [analyzeInto, drafts],
  );

  const remove = useCallback((id: string) => {
    setDrafts((current) => current.filter((draft) => draft.id !== id));
  }, []);

  const runImport = useCallback(async () => {
    const items = buildImportItems(drafts);
    if (items.length === 0) {
      setError("Tick at least one entry to import");
      return;
    }

    setBusy(true);
    setError(null);
    try {
      const result = await api.importCommands(items);
      setSummary(result);
      setStage("done");
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setBusy(false);
    }
  }, [drafts]);

  const reset = useCallback(() => {
    setStage("source");
    setPlan(null);
    setPending(null);
    setPreview(null);
    setDrafts([]);
    setSummary(null);
    setError(null);
  }, []);

  const counts = useMemo(() => countDrafts(drafts), [drafts]);

  return {
    stage,
    busy,
    error,
    plan,
    pending,
    preview,
    drafts,
    summary,
    counts,
    prepareText,
    prepareFile,
    send,
    cancelSend,
    update,
    reanalyze,
    setSelectionForAll,
    setCollectionForAll,
    split,
    mergeWithPrevious,
    remove,
    runImport,
    reset,
    dismissError: () => setError(null),
  };
}
