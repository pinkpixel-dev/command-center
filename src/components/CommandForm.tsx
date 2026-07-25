import { useEffect, useState } from "react";
import type { FormEvent } from "react";

import { kindLabel, riskLabel } from "../lib/format";
import { COMMAND_KINDS, RISK_LEVELS } from "../lib/types";
import type { Collection, CommandInput, CommandKind, RiskLevel } from "../lib/types";
import { Button } from "./ui/Button";
import { CheckboxField, SelectField, TextAreaField, TextField } from "./ui/Field";
import { Modal } from "./ui/Modal";
import { TagInput } from "./ui/TagInput";

export interface CommandFormProps {
  open: boolean;
  mode: "create" | "edit";
  initial: CommandInput;
  collections: Collection[];
  tagSuggestions: string[];
  saving: boolean;
  error: string | null;
  onSubmit: (input: CommandInput) => void;
  onClose: () => void;
}

export function CommandForm({
  open,
  mode,
  initial,
  collections,
  tagSuggestions,
  saving,
  error,
  onSubmit,
  onClose,
}: CommandFormProps) {
  const [draft, setDraft] = useState<CommandInput>(initial);
  const [contentError, setContentError] = useState<string | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Reopening the dialog always starts from the entry it was opened for.
  useEffect(() => {
    if (open) {
      setDraft(initial);
      setContentError(null);
      setShowAdvanced(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, initial.title, initial.content]);

  const patch = (changes: Partial<CommandInput>) =>
    setDraft((current) => ({ ...current, ...changes }));

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (draft.content.trim().length === 0) {
      setContentError("Paste or type the command first");
      return;
    }
    setContentError(null);
    onSubmit(draft);
  };

  const toggleCollection = (id: number) =>
    patch({
      collectionIds: draft.collectionIds.includes(id)
        ? draft.collectionIds.filter((existing) => existing !== id)
        : [...draft.collectionIds, id],
    });

  return (
    <Modal
      open={open}
      size="lg"
      title={mode === "create" ? "Add a command" : "Edit command"}
      description="Title, command, and a couple of tags is plenty. The rest is optional."
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" form="command-form" type="submit" loading={saving}>
            {mode === "create" ? "Save command" : "Save changes"}
          </Button>
        </>
      }
    >
      <form id="command-form" className="form" onSubmit={submit} noValidate>
        {error && (
          <p className="form__error" role="alert">
            {error}
          </p>
        )}

        <TextField
          label="Title"
          value={draft.title}
          onChange={(event) => patch({ title: event.target.value })}
          placeholder="Leave blank to use the first line"
          autoFocus
        />

        <TextAreaField
          label="Command"
          mono
          rows={5}
          required
          value={draft.content}
          error={contentError ?? undefined}
          onChange={(event) => patch({ content: event.target.value })}
          placeholder="lsof -i :3000"
          hint="Use {{placeholders}} for the parts that change each time."
        />

        <TextField
          label="Description"
          value={draft.description}
          onChange={(event) => patch({ description: event.target.value })}
          placeholder="What it does, in one line"
        />

        <TagInput
          label="Tags"
          value={draft.tags}
          suggestions={tagSuggestions}
          onChange={(tags) => patch({ tags })}
          hint="Enter or comma to add. Tags describe what a command is about."
        />

        <div className="form__disclosure">
          <button
            type="button"
            className="disclosure-toggle"
            aria-expanded={showAdvanced}
            onClick={() => setShowAdvanced((current) => !current)}
          >
            {showAdvanced ? "Hide extra details" : "Add extra details"}
          </button>
        </div>

        {showAdvanced && (
          <div className="form__advanced">
            <div className="form__grid">
              <SelectField
                label="Type"
                value={draft.kind}
                onChange={(event) => patch({ kind: event.target.value as CommandKind })}
                options={COMMAND_KINDS.map((kind) => ({ value: kind, label: kindLabel(kind) }))}
              />

              <SelectField
                label="Risk"
                value={draft.riskLevel ?? ""}
                onChange={(event) =>
                  patch({ riskLevel: (event.target.value || null) as RiskLevel | null })
                }
                hint="Detected locally unless you set it yourself."
                options={[
                  { value: "", label: "Detect automatically" },
                  ...RISK_LEVELS.map((risk) => ({ value: risk, label: riskLabel(risk) })),
                ]}
              />

              <TextField
                label="Shell"
                value={draft.shell ?? ""}
                onChange={(event) => patch({ shell: event.target.value || null })}
                placeholder="bash, fish, pwsh"
              />

              <TextField
                label="Operating system"
                value={draft.operatingSystem ?? ""}
                onChange={(event) => patch({ operatingSystem: event.target.value || null })}
                placeholder="linux, macos, windows"
              />

              <TextField
                label="Language"
                value={draft.language ?? ""}
                onChange={(event) => patch({ language: event.target.value || null })}
                placeholder="toml, sql, json"
              />

              <TextField
                label="Working directory"
                value={draft.workingDirectory ?? ""}
                onChange={(event) => patch({ workingDirectory: event.target.value || null })}
                placeholder="~/projects/thing"
              />
            </div>

            <TextField
              label="Source"
              type="url"
              value={draft.sourceUrl ?? ""}
              onChange={(event) => patch({ sourceUrl: event.target.value || null })}
              placeholder="https://wiki.archlinux.org/..."
            />

            <TextAreaField
              label="Notes"
              rows={3}
              value={draft.notes}
              onChange={(event) => patch({ notes: event.target.value })}
              placeholder="The gotcha you will forget by next month"
            />

            {collections.length > 0 && (
              <fieldset className="fieldset">
                <legend className="field__label">Collections</legend>
                <div className="checkbox-grid">
                  {collections.map((collection) => (
                    <CheckboxField
                      key={collection.id}
                      label={collection.name}
                      checked={draft.collectionIds.includes(collection.id)}
                      onChange={() => toggleCollection(collection.id)}
                    />
                  ))}
                </div>
              </fieldset>
            )}

            <CheckboxField
              label="Favorite"
              hint="Pinned to the Favorites list in the sidebar."
              checked={draft.favorite}
              onChange={(event) => patch({ favorite: event.target.checked })}
            />
          </div>
        )}
      </form>
    </Modal>
  );
}
