import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { CollectionActions } from "./components/CollectionActions";
import { CollectionManager } from "./components/CollectionManager";
import type { CollectionManagerIntent } from "./components/CollectionManager";
import { CommandPalette } from "./components/CommandPalette";
import { ImportView } from "./components/import/ImportView";
import { createPaletteActions } from "./components/palette-actions";
import { ViewHeader } from "./components/ViewHeader";
import { CommandForm } from "./components/CommandForm";
import { CommandList } from "./components/CommandList";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { HelpGuide } from "./components/HelpGuide";
import { SettingsPanel } from "./components/SettingsPanel";
import { ShortcutsHelp } from "./components/ShortcutsHelp";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { Button } from "./components/ui/Button";
import { useToast } from "./components/ui/Toast";
import { useAiStatus } from "./hooks/useAiStatus";
import { useCommandActions } from "./hooks/useCommandActions";
import { useDebounced } from "./hooks/useDebounced";
import { useHotkeys } from "./hooks/useHotkeys";
import { useLibrary } from "./hooks/useLibrary";
import { useSettings, useTheme } from "./hooks/useSettings";
import { scopeTitle } from "./lib/format";
import { api, toAppError } from "./lib/ipc";
import { backupLibraryDatabase, exportLibraryMarkdown } from "./lib/library-files";
import { emptyCommandInput, toCommandInput } from "./lib/types";
import type {
  AppView,
  Collection,
  CommandEntry,
  CommandInput,
  CommandKind,
  ListQuery,
  Scope,
  SortOrder,
} from "./lib/types";

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
  const [openEntryId, setOpenEntryId] = useState<number | null>(null);
  const [navOpen, setNavOpen] = useState(false);
  const [view, setView] = useState<AppView>("library");
  const [collectionManagerIntent, setCollectionManagerIntent] =
    useState<CollectionManagerIntent | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<CommandEntry | null>(null);
  const [pendingCollectionDelete, setPendingCollectionDelete] = useState<Collection | null>(null);
  const [deletingCollection, setDeletingCollection] = useState(false);
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
  const ai = useAiStatus(settings.aiEnabled);

  // Settings is where a key is added or removed, so the AI-backed entry points
  // are re-checked as soon as the user leaves that screen.
  useEffect(() => {
    if (view !== "settings") ai.refresh();
  }, [ai.refresh, view]);

  // Turning AI off while Import is open must not leave the user on that screen.
  useEffect(() => {
    if (view === "import" && !ai.ready) setView("library");
  }, [ai.ready, view]);

  const openCreate = useCallback(() => {
    setFormError(null);
    setEditor({
      open: true,
      mode: "create",
      id: null,
      initial: emptyCommandInput({
        collectionIds: scope.type === "collection" ? [scope.id] : [],
        tags: scope.type === "tag" ? [scope.name] : [],
      }),
    });
  }, [scope]);

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
    setView("library");
    setNavOpen(false);
    setOpenEntryId(null);
  };

  const openView = (next: AppView) => {
    setView(next);
    setNavOpen(false);
  };

  const openShortcuts = () => {
    setShortcutsOpen(true);
    setNavOpen(false);
  };

  const openHelp = () => {
    setHelpOpen(true);
    setNavOpen(false);
  };

  const openPalette = () => {
    setPaletteOpen(true);
    setNavOpen(false);
  };

  const openCollectionManager = (intent: CollectionManagerIntent) => {
    setCollectionManagerIntent(intent);
    setNavOpen(false);
  };

  const requestCollectionDelete = (collection: Collection) => {
    setCollectionManagerIntent(null);
    setPendingCollectionDelete(collection);
  };

  const deleteCollection = async () => {
    const collection = pendingCollectionDelete;
    if (!collection) return;

    setDeletingCollection(true);
    try {
      await api.deleteCollection(collection.id);
      if (scope.type === "collection" && scope.id === collection.id) {
        setScope({ type: "all" });
        setOpenEntryId(null);
      }
      setPendingCollectionDelete(null);
      notify(`Deleted ${collection.name}. Its commands are still in the library.`, "success");
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    } finally {
      setDeletingCollection(false);
    }
  };

  // Search and "add" only make sense on the library screen; the rest are global.
  const libraryHotkeys = view === "library"
    ? [
        { combo: "/", handler: () => searchRef.current?.focus() },
        { combo: "n", handler: openCreate },
      ]
    : [];

  useHotkeys([
    ...libraryHotkeys,
    { combo: "mod+k", allowWhileTyping: true, handler: openPalette },
    { combo: "shift+?", allowWhileTyping: true, handler: openShortcuts },
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

  const activeCollection = scope.type === "collection"
    ? collections.find((collection) => collection.id === scope.id) ?? null
    : null;
  const title = activeCollection?.name ?? (
    scope.type === "collection" ? "Collection" : scopeTitle(scope)
  );

  const subtitle = loading
    ? "Loading"
    : `${entries.length} ${entries.length === 1 ? "entry" : "entries"}${
        debouncedSearch.trim() ? ` matching "${debouncedSearch.trim()}"` : ""
      }`;

  const focusSearch = () => {
    setView("library");
    window.setTimeout(() => searchRef.current?.focus(), 0);
  };

  const exportMarkdown = async () => {
    try {
      const destination = await exportLibraryMarkdown();
      if (destination) notify("Markdown export saved", "success");
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    }
  };

  const backupLibrary = async () => {
    try {
      const destination = await backupLibraryDatabase();
      if (destination) notify("Library backup saved", "success");
    } catch (caught) {
      notify(toAppError(caught).message, "error");
    }
  };

  const paletteActions = createPaletteActions({
    onAdd: openCreate,
    onSearch: focusSearch,
    onScopeChange: changeScope,
    onManageCollections: () => openCollectionManager({ type: "manage" }),
    onExport: () => void exportMarkdown(),
    onBackup: () => void backupLibrary(),
    onHelp: openHelp,
    onShortcuts: openShortcuts,
    onSettings: () => openView("settings"),
    onImport: ai.ready ? () => openView("import") : undefined,
  });

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
          view={view}
          importAvailable={ai.ready}
          onScopeChange={changeScope}
          onOpenImport={() => openView("import")}
          onOpenPalette={openPalette}
          onOpenHelp={openHelp}
          onOpenShortcuts={openShortcuts}
          onOpenSettings={() => openView("settings")}
          onManageCollections={() => openCollectionManager({ type: "manage" })}
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
        {view === "settings" && (
          <>
            <ViewHeader
              title="Settings"
              subtitle="Preferences are stored in the same local database as your commands."
              onOpenMenu={() => setNavOpen(true)}
              actions={
                <Button variant="secondary" size="sm" onClick={() => setView("library")}>
                  Back to library
                </Button>
              }
            />
            <div className="shell__content">
              <SettingsPanel settings={settings} onSave={saveSettings} />
            </div>
          </>
        )}

        {view === "import" && ai.ready && (
          <>
            <ViewHeader
              title="Import"
              subtitle="Pull commands out of a cheat sheet, README, or your own notes. Read here, sent to OpenAI only after you say so."
              onOpenMenu={() => setNavOpen(true)}
              actions={
                <Button variant="secondary" size="sm" onClick={() => setView("library")}>
                  Back to library
                </Button>
              }
            />
            <div className="shell__content">
              <ImportView
                collections={collections}
                tagSuggestions={tags.map((tag) => tag.name)}
                onOpenLibrary={() => setView("library")}
                onImported={() => void refresh()}
              />
            </div>
          </>
        )}

        {view === "library" && (
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
              onOpenMenu={() => setNavOpen(true)}
              contextActions={
                activeCollection ? (
                  <CollectionActions
                    collectionName={activeCollection.name}
                    onRename={() => openCollectionManager({
                      type: "rename",
                      collectionId: activeCollection.id,
                    })}
                    onDelete={() => requestCollectionDelete(activeCollection)}
                    onManageAll={() => openCollectionManager({ type: "manage" })}
                  />
                ) : undefined
              }
            />

            <div className="shell__content">
              <CommandList
                entries={entries}
                loading={loading}
                error={error}
                searching={debouncedSearch.trim().length > 0}
                viewMode={settings.commandViewMode}
                aiReady={ai.ready}
                openEntryId={openEntryId}
                onOpenEntry={setOpenEntryId}
                onCopy={(entry, text) => void actions.copy(entry, text)}
                onEdit={openEdit}
                onDelete={confirmDelete}
                onToggleFavorite={(entry) => void actions.toggleFavorite(entry)}
                onOpenSource={(url) => void actions.openSource(url)}
                onAdd={openCreate}
                onRetry={() => void refresh()}
              />

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
        open={collectionManagerIntent !== null}
        intent={collectionManagerIntent ?? { type: "manage" }}
        collections={collections}
        onClose={() => setCollectionManagerIntent(null)}
        onChanged={() => void refresh()}
        onRequestDelete={requestCollectionDelete}
      />

      <ShortcutsHelp
        open={shortcutsOpen}
        onClose={() => setShortcutsOpen(false)}
      />

      <HelpGuide
        open={helpOpen}
        onClose={() => setHelpOpen(false)}
      />

      <CommandPalette
        open={paletteOpen}
        actions={paletteActions}
        onClose={() => setPaletteOpen(false)}
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

      <ConfirmDialog
        open={pendingCollectionDelete !== null}
        title="Delete this collection?"
        body={`"${pendingCollectionDelete?.name ?? ""}" will be removed. Its commands will stay in the library and keep any other collection memberships.`}
        confirmLabel="Delete collection"
        busy={deletingCollection}
        onConfirm={() => void deleteCollection()}
        onCancel={() => {
          if (!deletingCollection) setPendingCollectionDelete(null);
        }}
      />
    </div>
  );
}
