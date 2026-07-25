import { describe, expect, it } from "vitest";

import { isTypingTarget, matchesCombo } from "./hotkeys";
import type { KeyStroke } from "./hotkeys";

function stroke(overrides: Partial<KeyStroke> & { key: string }): KeyStroke {
  return {
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ...overrides,
  };
}

describe("matchesCombo", () => {
  it("matches a bare key", () => {
    expect(matchesCombo("n", stroke({ key: "n" }))).toBe(true);
    expect(matchesCombo("n", stroke({ key: "m" }))).toBe(false);
  });

  it("treats mod as ctrl on linux and cmd on mac", () => {
    expect(matchesCombo("mod+k", stroke({ key: "k", ctrlKey: true }))).toBe(true);
    expect(matchesCombo("mod+k", stroke({ key: "k", metaKey: true }))).toBe(true);
    expect(matchesCombo("mod+k", stroke({ key: "k" }))).toBe(false);
  });

  it("does not fire a bare letter when a modifier is held", () => {
    expect(matchesCombo("n", stroke({ key: "n", ctrlKey: true }))).toBe(false);
    expect(matchesCombo("n", stroke({ key: "n", shiftKey: true }))).toBe(false);
  });

  it("supports symbols that need shift", () => {
    expect(matchesCombo("shift+?", stroke({ key: "?", shiftKey: true }))).toBe(true);
    expect(matchesCombo("shift+?", stroke({ key: "?" }))).toBe(false);
  });

  it("is case insensitive about the key name", () => {
    expect(matchesCombo("escape", stroke({ key: "Escape" }))).toBe(true);
  });

  it("respects alt", () => {
    expect(matchesCombo("n", stroke({ key: "n", altKey: true }))).toBe(false);
    expect(matchesCombo("alt+n", stroke({ key: "n", altKey: true }))).toBe(true);
  });
});

describe("isTypingTarget", () => {
  it("recognises fields the user could be typing into", () => {
    const input = document.createElement("input");
    const textarea = document.createElement("textarea");
    const select = document.createElement("select");
    const div = document.createElement("div");

    expect(isTypingTarget(input)).toBe(true);
    expect(isTypingTarget(textarea)).toBe(true);
    expect(isTypingTarget(select)).toBe(true);
    expect(isTypingTarget(div)).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });
});
