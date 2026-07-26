import { useEffect, useMemo, useState } from "react";
import { Search } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Kbd } from "./ui/Kbd";
import { Modal } from "./ui/Modal";

export interface PaletteAction {
  id: string;
  label: string;
  description: string;
  icon: LucideIcon;
  keywords?: string[];
  onSelect: () => void;
}

export interface CommandPaletteProps {
  open: boolean;
  actions: PaletteAction[];
  onClose: () => void;
}

export function CommandPalette({ open, actions, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  const filtered = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase();
    if (!needle) return actions;

    return actions.filter((action) =>
      [action.label, action.description, ...(action.keywords ?? [])]
        .join(" ")
        .toLocaleLowerCase()
        .includes(needle),
    );
  }, [actions, query]);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActiveIndex(0);
  }, [open]);

  useEffect(() => {
    setActiveIndex((current) => Math.min(current, Math.max(filtered.length - 1, 0)));
  }, [filtered.length]);

  const choose = (action: PaletteAction) => {
    onClose();
    action.onSelect();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((current) => (filtered.length ? (current + 1) % filtered.length : 0));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) =>
        filtered.length ? (current - 1 + filtered.length) % filtered.length : 0,
      );
    } else if (event.key === "Enter" && filtered[activeIndex]) {
      event.preventDefault();
      choose(filtered[activeIndex]);
    }
  };

  return (
    <Modal
      open={open}
      title="Command palette"
      description="Move around the app without leaving the keyboard."
      onClose={onClose}
    >
      <div className="palette">
        <label className="palette__search">
          <Search size={16} aria-hidden="true" />
          <input
            className="input"
            type="search"
            aria-label="Find an action"
            value={query}
            placeholder="Find an action"
            autoComplete="off"
            data-modal-autofocus
            onChange={(event) => {
              setQuery(event.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={onKeyDown}
          />
          <span className="palette__escape" aria-hidden="true">
            <Kbd keys="Esc" />
          </span>
        </label>

        {filtered.length > 0 ? (
          <ul className="palette__results" aria-label="Available actions">
            {filtered.map((action, index) => {
              const Icon = action.icon;
              return (
                <li key={action.id}>
                  <button
                    type="button"
                    className={`palette__action${index === activeIndex ? " is-active" : ""}`}
                    onMouseEnter={() => setActiveIndex(index)}
                    onFocus={() => setActiveIndex(index)}
                    onClick={() => choose(action)}
                  >
                    <Icon size={17} aria-hidden="true" />
                    <span>
                      <strong>{action.label}</strong>
                      <small>{action.description}</small>
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        ) : (
          <p className="palette__empty" role="status">
            No matching actions.
          </p>
        )}
      </div>
    </Modal>
  );
}
