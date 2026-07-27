import {
  Bug,
  Clock,
  DatabaseBackup,
  Download,
  FileInput,
  FolderCog,
  HelpCircle,
  Keyboard,
  Library,
  MessagesSquare,
  Plus,
  Search,
  Settings,
  Star,
  Terminal,
} from "lucide-react";

import type { Scope } from "../lib/types";
import type { PaletteAction } from "./CommandPalette";

export interface PaletteActionHandlers {
  onAdd: () => void;
  onSearch: () => void;
  onScopeChange: (scope: Scope) => void;
  onManageCollections: () => void;
  onExport: () => void;
  onBackup: () => void;
  onHelp: () => void;
  onShortcuts: () => void;
  onSettings: () => void;
  /** Only provided when AI is on and a key is stored. */
  onImport?: () => void;
  /** Same gate as Import. */
  onAssistant?: () => void;
  /** Same gate again: opens the assistant straight onto the paste box. */
  onAnalyzeError?: () => void;
}

export function createPaletteActions(handlers: PaletteActionHandlers): PaletteAction[] {
  const importAction: PaletteAction[] = handlers.onImport
    ? [
        {
          id: "import-document",
          label: "Import from a document",
          description: "Pull commands out of a cheat sheet with OpenAI",
          icon: FileInput,
          keywords: ["ai", "openai", "markdown", "readme"],
          onSelect: handlers.onImport,
        },
      ]
    : [];

  const assistantAction: PaletteAction[] = handlers.onAssistant
    ? [
        {
          id: "open-assistant",
          label: "Ask the assistant",
          description: "Find a command or ask about one you saved",
          icon: MessagesSquare,
          keywords: ["ai", "openai", "chat", "help", "explain"],
          onSelect: handlers.onAssistant,
        },
      ]
    : [];

  const errorAction: PaletteAction[] = handlers.onAnalyzeError
    ? [
        {
          id: "analyze-error",
          label: "Read a terminal error",
          description: "Paste what failed and get a likely cause",
          icon: Bug,
          keywords: ["ai", "openai", "debug", "stack", "trace", "failed", "crash"],
          onSelect: handlers.onAnalyzeError,
        },
      ]
    : [];

  return [
    {
      id: "add-command",
      label: "Add command",
      description: "Create a new library entry",
      icon: Plus,
      keywords: ["new", "create"],
      onSelect: handlers.onAdd,
    },
    {
      id: "search-library",
      label: "Search the library",
      description: "Focus the main command search",
      icon: Search,
      keywords: ["find", "filter"],
      onSelect: handlers.onSearch,
    },
    {
      id: "all-commands",
      label: "Open All commands",
      description: "Show the complete library",
      icon: Library,
      onSelect: () => handlers.onScopeChange({ type: "all" }),
    },
    {
      id: "favorites",
      label: "Open Favorites",
      description: "Show starred entries",
      icon: Star,
      onSelect: () => handlers.onScopeChange({ type: "favorites" }),
    },
    {
      id: "recent",
      label: "Open Recent",
      description: "Show recently copied entries",
      icon: Clock,
      onSelect: () => handlers.onScopeChange({ type: "recent" }),
    },
    {
      id: "scripts",
      label: "Open Scripts",
      description: "Show saved scripts",
      icon: Terminal,
      onSelect: () => handlers.onScopeChange({ type: "scripts" }),
    },
    ...assistantAction,
    ...errorAction,
    ...importAction,
    {
      id: "manage-collections",
      label: "Manage collections",
      description: "Create, rename, or delete collections",
      icon: FolderCog,
      onSelect: handlers.onManageCollections,
    },
    {
      id: "export-markdown",
      label: "Export Markdown",
      description: "Save a readable copy of the complete library",
      icon: Download,
      keywords: ["save", "data"],
      onSelect: handlers.onExport,
    },
    {
      id: "backup-library",
      label: "Back up library",
      description: "Save a restorable copy of the SQLite database",
      icon: DatabaseBackup,
      keywords: ["save", "data", "database"],
      onSelect: handlers.onBackup,
    },
    {
      id: "help",
      label: "Open Help",
      description: "Learn the main workflow and placeholder syntax",
      icon: HelpCircle,
      onSelect: handlers.onHelp,
    },
    {
      id: "shortcuts",
      label: "Keyboard shortcuts",
      description: "See every available shortcut",
      icon: Keyboard,
      onSelect: handlers.onShortcuts,
    },
    {
      id: "settings",
      label: "Open Settings",
      description: "Change appearance and app behavior",
      icon: Settings,
      onSelect: handlers.onSettings,
    },
  ];
}
