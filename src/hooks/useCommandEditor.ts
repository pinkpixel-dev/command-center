import { useCallback, useState } from "react";

import { toAppError } from "../lib/ipc";
import { emptyCommandInput, toCommandInput } from "../lib/types";
import type { CommandEntry, CommandInput } from "../lib/types";

export interface CommandEditorState {
  open: boolean;
  mode: "create" | "edit";
  initial: CommandInput;
  saving: boolean;
  error: string | null;
  /** A blank entry, plus whatever the current scope implies. */
  openCreate: (overrides?: Partial<CommandInput>) => void;
  openEdit: (entry: CommandEntry) => void;
  /** A prefilled new entry, used by the assistant's Review and save. */
  openWith: (input: CommandInput) => void;
  submit: (input: CommandInput) => void;
  close: () => void;
}

interface EditorState {
  open: boolean;
  mode: "create" | "edit";
  id: number | null;
  initial: CommandInput;
}

const CLOSED: EditorState = {
  open: false,
  mode: "create",
  id: null,
  initial: emptyCommandInput(),
};

/**
 * The entry form's own state. Everything that opens the form goes through here,
 * so a proposed command reaches the library the same way a typed one does.
 */
export function useCommandEditor(
  save: (input: CommandInput, id: number | null) => Promise<unknown>,
): CommandEditorState {
  const [editor, setEditor] = useState<EditorState>(CLOSED);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const openCreate = useCallback((overrides: Partial<CommandInput> = {}) => {
    setError(null);
    setEditor({
      open: true,
      mode: "create",
      id: null,
      initial: emptyCommandInput(overrides),
    });
  }, []);

  const openWith = useCallback((initial: CommandInput) => {
    setError(null);
    setEditor({ open: true, mode: "create", id: null, initial });
  }, []);

  const openEdit = useCallback((entry: CommandEntry) => {
    setError(null);
    setEditor({ open: true, mode: "edit", id: entry.id, initial: toCommandInput(entry) });
  }, []);

  const submit = useCallback(
    (input: CommandInput) => {
      setSaving(true);
      setError(null);
      save(input, editor.id)
        .then(() => setEditor((current) => ({ ...current, open: false })))
        .catch((caught) => setError(toAppError(caught).message))
        .finally(() => setSaving(false));
    },
    [editor.id, save],
  );

  return {
    open: editor.open,
    mode: editor.mode,
    initial: editor.initial,
    saving,
    error,
    openCreate,
    openEdit,
    openWith,
    submit,
    close: useCallback(() => setEditor((current) => ({ ...current, open: false })), []),
  };
}
