import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, KeyboardEvent } from "react";
import { AlertTriangle, Check } from "lucide-react";

import { Button } from "./components/ui/Button";
import { Kbd } from "./components/ui/Kbd";
import { useDebounced } from "./hooks/useDebounced";
import { useSettings, useTheme } from "./hooks/useSettings";
import { api, toAppError } from "./lib/ipc";
import { emptyCommandInput } from "./lib/types";
import type { Collection, CommandEntry } from "./lib/types";

/**
 * The capture window. One paste, one keystroke, done. Anything more detailed
 * belongs in the main library window.
 */
export default function QuickAddApp() {
  const { settings } = useSettings();
  useTheme(settings.theme);

  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [tags, setTags] = useState("");
  const [collectionId, setCollectionId] = useState<number | null>(null);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [duplicate, setDuplicate] = useState<CommandEntry | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const contentRef = useRef<HTMLTextAreaElement>(null);
  const debouncedContent = useDebounced(content, 250);

  useEffect(() => {
    contentRef.current?.focus();
    api.listCollections().then(setCollections).catch(() => setCollections([]));
  }, []);

  useEffect(() => {
    setCollectionId(settings.defaultCollectionId);
  }, [settings.defaultCollectionId]);

  // Warn about an entry that already holds the same command.
  useEffect(() => {
    const trimmed = debouncedContent.trim();
    if (trimmed.length === 0) {
      setDuplicate(null);
      return;
    }
    let active = true;
    api
      .findDuplicate(trimmed)
      .then((found) => {
        if (active) setDuplicate(found);
      })
      .catch(() => {
        if (active) setDuplicate(null);
      });
    return () => {
      active = false;
    };
  }, [debouncedContent]);

  const parsedTags = useMemo(
    () =>
      tags
        .split(",")
        .map((tag) => tag.trim().toLowerCase())
        .filter(Boolean),
    [tags],
  );

  const reset = () => {
    setContent("");
    setTitle("");
    setTags("");
    setDuplicate(null);
    contentRef.current?.focus();
  };

  const save = async (event?: FormEvent) => {
    event?.preventDefault();
    if (content.trim().length === 0) {
      setError("Paste a command first");
      contentRef.current?.focus();
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const entry = await api.createCommand(
        emptyCommandInput({
          title,
          content,
          tags: parsedTags,
          collectionIds: collectionId !== null ? [collectionId] : [],
        }),
      );

      if (settings.closeQuickAddAfterSave) {
        reset();
        await api.closeQuickAdd();
        return;
      }

      setSaved(`Saved "${entry.title}"`);
      window.setTimeout(() => setSaved(null), 2500);
      reset();
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setSaving(false);
    }
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      void api.closeQuickAdd();
      return;
    }
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      void save();
    }
  };

  return (
    <form className="quick-add" onSubmit={save} onKeyDown={onKeyDown}>
      <div className="quick-add__head">
        <h1>Quick Add</h1>
        <span className="quick-add__hint">
          <Kbd keys="Ctrl + Enter" /> to save · <Kbd keys="Esc" /> to dismiss
        </span>
      </div>

      <textarea
        ref={contentRef}
        className="input input--textarea input--mono quick-add__content"
        value={content}
        onChange={(event) => setContent(event.target.value)}
        placeholder="Paste the command"
        aria-label="Command"
        rows={4}
        spellCheck={false}
      />

      <div className="quick-add__row">
        <input
          className="input input--compact"
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder="Title (optional)"
          aria-label="Title"
        />
        <input
          className="input input--compact"
          value={tags}
          onChange={(event) => setTags(event.target.value)}
          placeholder="tags, comma separated"
          aria-label="Tags"
          autoComplete="off"
        />
        {collections.length > 0 && (
          <select
            className="input input--select input--compact"
            value={collectionId === null ? "" : String(collectionId)}
            onChange={(event) => setCollectionId(event.target.value ? Number(event.target.value) : null)}
            aria-label="Collection"
          >
            <option value="">No collection</option>
            {collections.map((collection) => (
              <option key={collection.id} value={collection.id}>
                {collection.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {duplicate && (
        <p className="quick-add__notice" role="status">
          <AlertTriangle size={14} aria-hidden="true" />
          Already saved as "{duplicate.title}". Saving again creates a second copy.
        </p>
      )}

      {error && (
        <p className="quick-add__notice is-error" role="alert">
          <AlertTriangle size={14} aria-hidden="true" />
          {error}
        </p>
      )}

      {saved && (
        <p className="quick-add__notice is-ok" role="status">
          <Check size={14} aria-hidden="true" />
          {saved}
        </p>
      )}

      <div className="quick-add__actions">
        <Button variant="ghost" onClick={() => void api.closeQuickAdd()}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" loading={saving}>
          Save
        </Button>
      </div>
    </form>
  );
}
