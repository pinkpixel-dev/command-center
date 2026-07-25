import type { CommandEntry } from "../lib/types";

/** Builds a realistic entry for component tests. */
export function makeEntry(overrides: Partial<CommandEntry> = {}): CommandEntry {
  return {
    id: 1,
    title: "Update Arch packages",
    content: "sudo pacman -Syu",
    description: "Refresh every installed package",
    kind: "command",
    language: null,
    shell: "bash",
    operatingSystem: "linux",
    riskLevel: "caution",
    riskReasons: ["Runs with elevated privileges", "Installs or removes system packages"],
    favorite: false,
    workingDirectory: null,
    sourceUrl: null,
    notes: "",
    contentHash: "hash",
    copyCount: 18,
    lastCopiedAt: "2026-07-25T10:00:00Z",
    createdAt: "2026-07-01T10:00:00Z",
    updatedAt: "2026-07-20T10:00:00Z",
    tags: ["arch", "pacman"],
    collections: [],
    variables: [],
    ...overrides,
  };
}
