import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const MAX_SOURCE_LINES = 300;
const FRONTEND_SOURCE_EXTENSIONS = new Set([".ts", ".tsx", ".css"]);

function sourceFilesIn(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFilesIn(entryPath);
    return FRONTEND_SOURCE_EXTENSIONS.has(path.extname(entry.name)) ? [entryPath] : [];
  });
}

describe("limite estrutural de linhas", () => {
  it("mantém todo arquivo de frontend em até 300 linhas", () => {
    const sourceRoot = path.resolve(process.cwd(), "src");
    const violations = sourceFilesIn(sourceRoot)
      .map((file) => {
        const content = readFileSync(file, "utf8");
        return {
          file: path.relative(process.cwd(), file),
          lines: content === "" ? 0 : content.split(/\r?\n/).length - Number(content.endsWith("\n")),
        };
      })
      .filter(({ lines }) => lines > MAX_SOURCE_LINES)
      .map(({ file, lines }) => `${file}: ${lines} linhas (máximo: ${MAX_SOURCE_LINES})`);

    expect(violations, violations.join("\n")).toEqual([]);
  });
});
