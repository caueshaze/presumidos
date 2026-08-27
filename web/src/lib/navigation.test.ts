import { describe, expect, it } from "vitest";
import { safeReturnTo } from "./navigation";

describe("safeReturnTo", () => {
  it("preserves an internal invite route", () => {
    expect(safeReturnTo("/pools/join/ABC123?from=share")).toBe(
      "/pools/join/ABC123?from=share",
    );
  });

  it("rejects external and protocol-relative destinations", () => {
    expect(safeReturnTo("https://malicioso.example", "/")).toBe("/");
    expect(safeReturnTo("//malicioso.example", "/")).toBe("/");
    expect(safeReturnTo("javascript:alert(1)", "/")).toBe("/");
  });
});
