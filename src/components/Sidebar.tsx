import { Clock, Ellipsis, Library, Settings, Star, Terminal, X } from "lucide-react";

import { APP_NAME, APP_VERSION } from "../lib/app-info";
import { scopesEqual } from "../lib/format";
import type { AppView, Collection, LibraryStats, Scope, Tag } from "../lib/types";
import { Button } from "./ui/Button";

export interface SidebarProps {
  stats: LibraryStats;
  tags: Tag[];
  collections: Collection[];
  scope: Scope;
  view: AppView;
  onScopeChange: (scope: Scope) => void;
  onOpenSettings: () => void;
  onCreateCollection: () => void;
  onManageCollections: () => void;
  onDismiss: () => void;
}

const NAV: { scope: Scope; label: string; icon: typeof Library; countKey: keyof LibraryStats }[] = [
  { scope: { type: "all" }, label: "All commands", icon: Library, countKey: "total" },
  { scope: { type: "favorites" }, label: "Favorites", icon: Star, countKey: "favorites" },
  { scope: { type: "recent" }, label: "Recent", icon: Clock, countKey: "recent" },
  { scope: { type: "scripts" }, label: "Scripts", icon: Terminal, countKey: "scripts" },
];

export function Sidebar({
  stats,
  tags,
  collections,
  scope,
  view,
  onScopeChange,
  onOpenSettings,
  onManageCollections,
  onDismiss,
}: SidebarProps) {
  const isActive = (candidate: Scope) => view === "library" && scopesEqual(candidate, scope);

  return (
    <nav className="sidebar" aria-label="Library sections">
      <div className="sidebar__brand">
        <div>
          <span className="sidebar__title">{APP_NAME}</span>
          <span className="sidebar__version">v{APP_VERSION}</span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          className="sidebar__dismiss"
          aria-label="Close navigation"
          onClick={onDismiss}
        >
          <X size={16} aria-hidden="true" />
        </Button>
      </div>

      <ul className="sidebar__nav">
        {NAV.map(({ scope: target, label, icon: Icon, countKey }) => (
          <li key={label}>
            <button
              type="button"
              className={`nav-item${isActive(target) ? " is-active" : ""}`}
              aria-current={isActive(target) ? "page" : undefined}
              onClick={() => onScopeChange(target)}
            >
              <Icon size={15} aria-hidden="true" />
              <span className="nav-item__label">{label}</span>
              <span className="nav-item__count">{stats[countKey]}</span>
            </button>
          </li>
        ))}
      </ul>

      <section className="sidebar__section" aria-labelledby="sidebar-collections">
        <div className="sidebar__section-header">
          <h2 id="sidebar-collections">Collections</h2>
          <div className="sidebar__section-actions" aria-label="Collection actions">

            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Manage collections"
              title="Manage collections"
              onClick={onManageCollections}
            >
              <Ellipsis size={17} aria-hidden="true" />
            </Button>
          </div>
        </div>

        {collections.length === 0 ? (
          <p className="sidebar__hint">Create a collection to keep related commands together.</p>
        ) : (
          <ul className="sidebar__nav">
            {collections.map((collection) => {
              const target: Scope = { type: "collection", id: collection.id };
              return (
                <li key={collection.id}>
                  <button
                    type="button"
                    className={`nav-item${isActive(target) ? " is-active" : ""}`}
                    aria-current={isActive(target) ? "page" : undefined}
                    onClick={() => onScopeChange(target)}
                  >
                    <span className="nav-item__label">{collection.name}</span>
                    <span className="nav-item__count">{collection.commandCount}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      {tags.length > 0 && (
        <section className="sidebar__section" aria-labelledby="sidebar-tags">
          <div className="sidebar__section-header">
            <h2 id="sidebar-tags">Tags</h2>
          </div>
          <div className="tag-cloud">
            {tags.slice(0, 18).map((tag) => {
              const target: Scope = { type: "tag", name: tag.name };
              const active = isActive(target);
              return (
                <button
                  key={tag.id}
                  type="button"
                  className={`chip${active ? " is-active" : ""}`}
                  aria-pressed={active}
                  onClick={() => onScopeChange(active ? { type: "all" } : target)}
                >
                  {tag.name}
                  <span className="chip__count">{tag.commandCount}</span>
                </button>
              );
            })}
          </div>
        </section>
      )}

      <div className="sidebar__footer">
        <button
          type="button"
          className={`nav-item${view === "settings" ? " is-active" : ""}`}
          aria-current={view === "settings" ? "page" : undefined}
          onClick={onOpenSettings}
        >
          <Settings size={15} aria-hidden="true" />
          <span className="nav-item__label">Settings</span>
        </button>
      </div>
    </nav>
  );
}
