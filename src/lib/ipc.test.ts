import { describe, expect, it } from "vitest";

import { toAppError } from "./ipc";

describe("toAppError", () => {
  it("passes through the structured error the Rust layer sends", () => {
    expect(toAppError({ kind: "not_found", message: "That command was not found" })).toEqual({
      kind: "not_found",
      message: "That command was not found",
    });
  });

  it("wraps a bare string", () => {
    expect(toAppError("boom")).toEqual({ kind: "runtime", message: "boom" });
  });

  it("falls back to something readable for anything else", () => {
    expect(toAppError(undefined).message).toMatch(/went wrong/i);
    expect(toAppError(new Error("thrown")).message).toBe("thrown");
  });
});
