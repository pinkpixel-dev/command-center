import { useId, useState } from "react";
import type { KeyboardEvent } from "react";
import { X } from "lucide-react";

export interface TagInputProps {
  label: string;
  value: string[];
  onChange: (next: string[]) => void;
  suggestions?: string[];
  hint?: string;
}

/** Comma or Enter commits a tag; Backspace on an empty field removes the last. */
export function TagInput({ label, value, onChange, suggestions = [], hint }: TagInputProps) {
  const [draft, setDraft] = useState("");
  const inputId = useId();
  const listId = `${inputId}-suggestions`;
  const hintId = `${inputId}-hint`;

  const commit = (raw: string) => {
    const tag = raw.trim().toLowerCase();
    if (!tag || value.includes(tag)) {
      setDraft("");
      return;
    }
    onChange([...value, tag]);
    setDraft("");
  };

  const remove = (tag: string) => onChange(value.filter((existing) => existing !== tag));

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter" || event.key === ",") {
      event.preventDefault();
      commit(draft);
      return;
    }
    if (event.key === "Backspace" && draft === "" && value.length > 0) {
      event.preventDefault();
      remove(value[value.length - 1]);
    }
  };

  const available = suggestions.filter((suggestion) => !value.includes(suggestion));

  return (
    <div className="field">
      <label className="field__label" htmlFor={inputId}>
        {label}
      </label>

      <div className="tag-input">
        {value.map((tag) => (
          <span key={tag} className="tag-input__tag">
            {tag}
            <button
              type="button"
              className="tag-input__remove"
              onClick={() => remove(tag)}
              aria-label={`Remove tag ${tag}`}
            >
              <X size={12} aria-hidden="true" />
            </button>
          </span>
        ))}
        <input
          id={inputId}
          className="tag-input__field"
          value={draft}
          list={available.length > 0 ? listId : undefined}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={onKeyDown}
          onBlur={() => commit(draft)}
          placeholder={value.length === 0 ? "docker, cleanup" : ""}
          aria-describedby={hint ? hintId : undefined}
          autoComplete="off"
        />
      </div>

      {available.length > 0 && (
        <datalist id={listId}>
          {available.map((suggestion) => (
            <option key={suggestion} value={suggestion} />
          ))}
        </datalist>
      )}

      {hint && (
        <p className="field__hint" id={hintId}>
          {hint}
        </p>
      )}
    </div>
  );
}
