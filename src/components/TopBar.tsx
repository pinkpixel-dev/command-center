import type { ReactNode, RefObject } from "react";
import { Menu, Plus, Search, X } from "lucide-react";

import { kindLabel } from "../lib/format";
import { COMMAND_KINDS } from "../lib/types";
import type { CommandKind, SortOrder } from "../lib/types";
import { Button } from "./ui/Button";
import { Kbd } from "./ui/Kbd";

export interface TopBarProps {
  title: string;
  subtitle: string;
  search: string;
  sort: SortOrder;
  kind: CommandKind | "";
  searchRef: RefObject<HTMLInputElement | null>;
  onSearchChange: (value: string) => void;
  onSortChange: (value: SortOrder) => void;
  onKindChange: (value: CommandKind | "") => void;
  onAdd: () => void;
  onOpenMenu: () => void;
  contextActions?: ReactNode;
}

const SORTS: { value: SortOrder; label: string }[] = [
  { value: "updated", label: "Last updated" },
  { value: "created", label: "Newest" },
  { value: "title", label: "Title A-Z" },
  { value: "copies", label: "Most copied" },
  { value: "lastCopied", label: "Last copied" },
];

export function TopBar({
  title,
  subtitle,
  search,
  sort,
  kind,
  searchRef,
  onSearchChange,
  onSortChange,
  onKindChange,
  onAdd,
  onOpenMenu,
  contextActions,
}: TopBarProps) {
  return (
    <header className="topbar">
      <div className="topbar__row">
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          className="topbar__menu"
          aria-label="Open navigation"
          onClick={onOpenMenu}
        >
          <Menu size={18} aria-hidden="true" />
        </Button>

        <div className="topbar__heading">
          <h1>{title}</h1>
          <p>{subtitle}</p>
        </div>

        <div className="topbar__actions">
          {contextActions}
          <Button variant="primary" size="sm" onClick={onAdd}>
            <Plus size={15} aria-hidden="true" />
            Add command
          </Button>
        </div>
      </div>

      <div className="topbar__row topbar__row--filters">
        <div className="search">
          <Search size={16} aria-hidden="true" className="search__icon" />
          <input
            ref={searchRef}
            type="search"
            className="search__input"
            placeholder="Search commands, tags, notes"
            value={search}
            onChange={(event) => onSearchChange(event.target.value)}
            aria-label="Search the library"
            autoComplete="off"
          />
          {search ? (
            <button
              type="button"
              className="search__clear"
              onClick={() => onSearchChange("")}
              aria-label="Clear search"
            >
              <X size={14} aria-hidden="true" />
            </button>
          ) : (
            <span className="search__hint" aria-hidden="true">
              <Kbd keys="Ctrl + K" />
            </span>
          )}
        </div>

        <label className="select-inline">
          <span className="visually-hidden">Filter by type</span>
          <select
            className="input input--select input--compact"
            value={kind}
            onChange={(event) => onKindChange(event.target.value as CommandKind | "")}
          >
            <option value="">All types</option>
            {COMMAND_KINDS.map((option) => (
              <option key={option} value={option}>
                {kindLabel(option)}
              </option>
            ))}
          </select>
        </label>

        <label className="select-inline">
          <span className="visually-hidden">Sort order</span>
          <select
            className="input input--select input--compact"
            value={sort}
            onChange={(event) => onSortChange(event.target.value as SortOrder)}
          >
            {SORTS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      </div>
    </header>
  );
}
