import { describe, expect, it } from "vitest";

import {
  copySummary,
  firstLine,
  kindLabel,
  lineCount,
  prettyShortcut,
  relativeTime,
  renderTemplate,
  scopeTitle,
  scopesEqual,
} from "./format";

describe("relativeTime", () => {
  const now = new Date("2026-07-25T12:00:00Z");

  it("describes fresh timestamps as just now", () => {
    expect(relativeTime("2026-07-25T11:59:40Z", now)).toBe("just now");
  });

  it("counts minutes, hours and days", () => {
    expect(relativeTime("2026-07-25T11:30:00Z", now)).toBe("30 min ago");
    expect(relativeTime("2026-07-25T09:00:00Z", now)).toBe("3 hours ago");
    expect(relativeTime("2026-07-24T12:00:00Z", now)).toBe("yesterday");
    expect(relativeTime("2026-07-20T12:00:00Z", now)).toBe("5 days ago");
  });

  it("falls back to a date for anything older than a month", () => {
    expect(relativeTime("2026-03-12T12:00:00Z", now)).toMatch(/Mar/);
  });

  it("handles missing and broken values without throwing", () => {
    expect(relativeTime(null, now)).toBe("never");
    expect(relativeTime("not a date", now)).toBe("unknown");
  });
});

describe("copySummary", () => {
  it("reads naturally at zero, one and many", () => {
    expect(copySummary({ copyCount: 0 })).toBe("Never copied");
    expect(copySummary({ copyCount: 1 })).toBe("Copied once");
    expect(copySummary({ copyCount: 18 })).toBe("Copied 18 times");
  });
});

describe("renderTemplate", () => {
  it("substitutes the values it was given", () => {
    expect(renderTemplate("ssh {{user}}@{{host}}", { user: "pinkpixel", host: "10.0.0.4" })).toBe(
      "ssh pinkpixel@10.0.0.4",
    );
  });

  it("keeps placeholders that have no value, instead of emptying them", () => {
    expect(renderTemplate("ssh {{user}}@{{host}}", { user: "pinkpixel" })).toBe(
      "ssh pinkpixel@{{host}}",
    );
    expect(renderTemplate("ssh {{user}}", { user: "" })).toBe("ssh {{user}}");
  });

  it("tolerates spacing inside the braces", () => {
    expect(renderTemplate("kill {{ pid }}", { pid: "900" })).toBe("kill 900");
  });
});

describe("prettyShortcut", () => {
  it("translates the cross-platform modifier", () => {
    expect(prettyShortcut("CommandOrControl+Shift+Space", false)).toBe("Ctrl + Shift + Space");
    expect(prettyShortcut("CommandOrControl+Shift+Space", true)).toBe("Cmd + Shift + Space");
  });
});

describe("scope helpers", () => {
  it("names each scope", () => {
    expect(scopeTitle({ type: "all" })).toBe("All commands");
    expect(scopeTitle({ type: "tag", name: "docker" })).toBe("#docker");
  });

  it("compares scopes including their payload", () => {
    expect(scopesEqual({ type: "all" }, { type: "all" })).toBe(true);
    expect(scopesEqual({ type: "tag", name: "git" }, { type: "tag", name: "git" })).toBe(true);
    expect(scopesEqual({ type: "tag", name: "git" }, { type: "tag", name: "docker" })).toBe(false);
    expect(scopesEqual({ type: "collection", id: 1 }, { type: "collection", id: 2 })).toBe(false);
    expect(scopesEqual({ type: "all" }, { type: "favorites" })).toBe(false);
  });
});

describe("content helpers", () => {
  it("finds the first meaningful line", () => {
    expect(firstLine("\n\n  git status  \nnext")).toBe("git status");
  });

  it("counts only non-empty lines", () => {
    expect(lineCount("npm test\n\nnpm run build\n")).toBe(2);
  });

  it("labels kinds for display", () => {
    expect(kindLabel("sequence")).toBe("Sequence");
  });
});
