import { useCallback, useMemo, useRef, useState } from "react";
import { Menu } from "lucide-react";

import { CollectionManager } from "./components/CollectionManager";
import { CommandForm } from "./components/CommandForm";
import { CommandList } from "./components/CommandList";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { SettingsPanel } from "./components/SettingsPanel";
import { ShortcutsHelp } from "./components/ShortcutsHelp";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { Button } from "./components/ui/Button";
import { useToast } from "./components/ui/Toast";
import { useCommandActions } from "./hooks/useCommandActions";
import { useDebounced } from "./hooks/useDebounced";
import { useHotkeys } from "./hooks/useHotkeys";
import { useLibrary } from "./hooks/useLibrary";
import { useSettings, useTheme } from "./hooks/useSettings";
import { scopeTitle } from "./lib/format";
import { api, toAppError } from "./lib/ipc";
import { emptyCommandInput, toCommandInput } from "./lib/types";
import type { CommandEntry, CommandInput, CommandKind, ListQuery, Scope, SortOrder } from "./lib/types";

interface EditorState {
  open: boolean;
  mode: "create" | "edit";
  id: number | null;
  initial: CommandInput;
}

export default function App() {
  const [scope, setScope] = useState<Scope>({ type: "all" });
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SortOrder>("updated");
  const [kind, setKind] = useState<CommandKind | "">("");
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [navOpen, setNavOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [collectionsOpen, setCollectionsOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<CommandEntry | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [editor, setEditor] = useState<EditorState>({
    open: false,
    mode: "create",
    id: null,
    initial: emptyCommandInput(),
  });

  const searchRef = useRef<HTMLInputElement>(null);
  const { notify } = useToast();
  const { settings, save: saveSettings } = useSettings();
  useTheme(settings.theme);

  // Typing should not fire a query per keystroke.
  const debouncedSearch = useDebounced(search, 160);

  const filter = useMemo<ListQuery>(
    () => ({
      search: debouncedSearch.trim() || null,
      scope,
      sort,
      kinds: kind ? [kind] : [],
    }),
    [debouncedSearch, scope, sort, kind],
  );

  const { entries, stats, tags, collections, loading, error, refresh } = useLibrary(filter);
  const actions = useCommandActions(refresh);

  const openCreate = useCallback(() => {
    setFormError(null);
    setEditor({
      open: true,
      mode: "create",
      id: null,
      initial: emptyCommandInput({
        collectionIds:
          scope.type === "collection"
            ? [scope.id]
            : settings.defaultCollectionId !== null
              ? [settings.defaultCollectionId]
              : [],
        tags: scope.type === "tag" ? [scope.name] : [],
      }),
    });
  }, [scope, settings.defaultCollectionId]);

  const openEdit = useCallback((entry: CommandEntry) => {
    setFormError(null);
    setEditor({ open: true, mode: "edit", id: entry.id, initial: toCommandInput(entry) });
  }, []);

  const submitEditor = async (input: CommandInput) => {
    setSaving(true);
    setFormError(null);
    try {
      await actions.save(input, editor.id);
      setEditor((current) => ({ ...current, open: false }));
    } catch (caught) {
      setFormError(toAppError(caught).message);
    } finally {
      setSaving(false);
    }
  };

  const confirmDelete = (entry: CommandEntry) => {
    if (settings.confirmBeforeDelete) {
      setPendingDelete(entry);
      return;
    }
    void actions.remove(entry);
  };

  const changeScope = (next: Scope) => {
    setScope(next);
    setSettingsOpen(false);
    setNavOpen(false);
    setExpandedId(null);
  };

  useHotkeys([
    {
      combo: "mod+k",
      allowWhileTyping: true,
      handler: () => searchRef.current?.focus(),
    },
    { combo: "/", handler: () => searchRef.current?.focus() },
    { combo: "n", handler: openCreate },
    { combo: "shift+?", allowWhileTyping: true, handler: () => setShortcutsOpen(true) },
    {
      combo: "escape",
      allowWhileTyping: true,
      handler: () => {
        if (document.activeElement === searchRef.current && search) {
          setSearch("");
          return;
        }
        setNavOpen(false);
      },
    },
  ]);

  const title = scope.type === "collection"
    ? collections.find((collection) => collection.id === scope.id)?.name ?? "Collection"
    : scopeTitle(scope);

  const subtitle = loading
    ? "Loading"
    : `${entries.length} ${entries.length === 1 ? "entry" : "entries"}${
        debouncedSearch.trim() ? ` matching "${debouncedSearch.trim()}"` : ""
      }`;

  return (
    <div className={`shell${navOpen ? " is-nav-open" : ""}`}>
      <a className="skip-link" href="#library">
        Skip to the command list
      </a>

      <div className="shell__sidebar">
        <Sidebar
          stats={stats}
          tags={tags}
          collections={collections}
          scope={scope}
          settingsOpen={settingsOpen}
          onScopeChange={changeScope}
          onOpenSettings={() => {
            setSettingsOpen(true);
            setNavOpen(false);
          }}
          onManageCollections={() => setCollectionsOpen(true)}
          onDismiss={() => setNavOpen(false)}
        />
      </div>

      {navOpen && (
        <button
          type="button"
          className="shell__scrim"
          aria-label="Close navigation"
          onClick={() => setNavOpen(false)}
        />
      )}

      <main className="shell__main" id="library">
        {settingsOpen ? (
          <>
            <header className="topbar">
              <div className="topbar__row">
                <Button
                  variant="ghost"
                  size="sm"
                  className="topbar__menu"
                  aria-label="Open navigation"
                  iconOnly
                  onClick={() => setNavOpen(true)}
                >
                  <Menu size={18} aria-hidden="true" />
                </Button>
                <div className="topbar__heading">
                  <h1>Settings</h1>
                  <p>Preferences are stored in the same local database as your commands.</p>
                </div>
                <div className="topbar__actions">
                  <Button variant="secondary" size="sm" onClick={() => setSettingsOpen(false)}>
                    Back to library
                  </Button>
                </div>
              </div>
            </header>
            <div className="shell__content">
              <SettingsPanel
                settings={settings}
                collections={collections}
                onSave={saveSettings}
              />
            </div>
          </>
        ) : (
          <>
            <TopBar
              title={title}
              subtitle={subtitle}
              search={search}
              sort={sort}
              kind={kind}
              searchRef={searchRef}
              onSearchChange={setSearch}
              onSortChange={setSort}
              onKindChange={setKind}
              onAdd={openCreate}
              onQuickAdd={() => {
                void api.openQuickAdd().catch((caught) => {
                  notify(toAppError(caught).message, "error");
                });
              }}
              onOpenMenu={() => setNavOpen(true)}
            />

            <div className="shell__content">
              <CommandList
                entries={entries}
                loading={loading}
                error={error}
                searching={debouncedSearch.trim().length > 0}
                expandedId={expandedId}
                onExpand={setExpandedId}
                onCopy={(entry, text) => void actions.copy(entry, text)}
                onEdit={openEdit}
                onDelete={confirmDelete}
                onToggleFavorite={(entry) => void actions.toggleFavorite(entry)}
                onOpenSource={(url) => void actions.openSource(url)}
                onAdd={openCreate}
                onRetry={() => void refresh()}
              />

              <footer className="shell__footer">
                <button type="button" className="link-button" onClick={() => setShortcutsOpen(true)}>
                  Keyboard shortcuts
                </button>
              </footer>
            </div>
          </>
        )}
      </main>

      <CommandForm
        open={editor.open}
        mode={editor.mode}
        initial={editor.initial}
        collections={collections}
        tagSuggestions={tags.map((tag) => tag.name)}
        saving={saving}
        error={formError}
        onSubmit={(input) => void submitEditor(input)}
        onClose={() => setEditor((current) => ({ ...current, open: false }))}
      />

      <CollectionManager
        open={collectionsOpen}
        collections={collections}
        onClose={() => setCollectionsOpen(false)}
        onChanged={() => void refresh()}
      />

      <ShortcutsHelp
        open={shortcutsOpen}
        quickAddShortcut={settings.quickAddShortcut}
        onClose={() => setShortcutsOpen(false)}
      />

      <ConfirmDialog
        open={pendingDelete !== null}
        title="Delete this command?"
        body={`"${pendingDelete?.title ?? ""}" will be removed from the library. This cannot be undone.`}
        confirmLabel="Delete"
        onConfirm={() => {
          if (pendingDelete) void actions.remove(pendingDelete);
          setPendingDelete(null);
        }}
        onCancel={() => setPendingDelete(null)}
      />
    </div>
  );
}
