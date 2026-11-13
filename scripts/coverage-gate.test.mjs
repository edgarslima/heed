import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  evaluateScope,
  loadReport,
  passesThreshold,
} from "./coverage-gate.mjs";

function writeTemp(name, contents) {
  const dir = mkdtempSync(join(tmpdir(), "coverage-gate-"));
  const path = join(dir, name);
  writeFileSync(path, contents);
  return { dir, path };
}

function llvmReport(files) {
  return JSON.stringify({
    type: "llvm.coverage.json.export",
    data: [
      {
        files: files.map(([filename, count, covered]) => ({
          filename,
          summary: { lines: { count, covered } },
        })),
      },
    ],
  });
}

function vitestReport(files) {
  const report = { total: {} };
  for (const [filename, lines, branches] of files) {
    report[filename] = {
      lines: { total: lines[0], covered: lines[1] },
      branches: { total: branches[0], covered: branches[1] },
    };
  }
  return JSON.stringify(report);
}

const rustScope = {
  name: "rust-tauri",
  kind: "llvm-cov",
  report: "coverage/rust.json",
  include: ["src-tauri/src/**/*.rs"],
  exclude: [],
  metrics: ["lines"],
  status: "active",
};

const frontendScope = {
  name: "frontend",
  kind: "vitest",
  report: "coverage/frontend/coverage-summary.json",
  include: ["src/**/*.ts"],
  exclude: ["**/*.test.ts"],
  metrics: ["lines", "branches"],
  status: "active",
};

describe("passesThreshold", () => {
  it("passes at 81 of 100 lines", () => {
    expect(passesThreshold(81, 100)).toBe(true);
  });

  it("fails at exactly 80 of 100 lines", () => {
    expect(passesThreshold(80, 100)).toBe(false);
  });

  it("passes at 801 of 1000 lines", () => {
    expect(passesThreshold(801, 1000)).toBe(true);
  });
});

describe("evaluateScope", () => {
  it("passes a scope above the threshold with integer counts", () => {
    const report = {
      files: { "src-tauri/src/lib.rs": { lines: { covered: 81, total: 100 } } },
    };

    const result = evaluateScope(rustScope, report, ["src-tauri/src/lib.rs"]);

    expect(result.status).toBe("pass");
    expect(result.metrics).toEqual([
      { metric: "lines", covered: 81, total: 100 },
    ]);
  });

  it("fails when the report file is missing", () => {
    const report = loadReport(rustScope, "/nonexistent-root");

    const result = evaluateScope(rustScope, report, ["src-tauri/src/lib.rs"]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("coverage/rust.json");
  });

  it("fails when the report is empty or invalid JSON", () => {
    for (const contents of ["", "{not json"]) {
      const { dir } = writeTemp("rust.json", contents);
      const scope = { ...rustScope, report: "rust.json" };

      const result = evaluateScope(scope, loadReport(scope, dir), [
        "src-tauri/src/lib.rs",
      ]);

      expect(result.status).toBe("fail");
    }
  });

  it("fails when an active scope has zero measured items", () => {
    const { dir } = writeTemp(
      "rust.json",
      llvmReport([["/repo/src-tauri/src/lib.rs", 0, 0]]),
    );
    const scope = { ...rustScope, report: "rust.json" };

    const result = evaluateScope(scope, loadReport(scope, dir, "/repo"), [
      "src-tauri/src/lib.rs",
    ]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("zero measured");
  });

  it("fails when a production file is missing from the report", () => {
    const { dir } = writeTemp(
      "rust.json",
      llvmReport([["/repo/src-tauri/src/lib.rs", 10, 10]]),
    );
    const scope = { ...rustScope, report: "rust.json" };

    const result = evaluateScope(scope, loadReport(scope, dir, "/repo"), [
      "src-tauri/src/lib.rs",
      "src-tauri/src/commands.rs",
    ]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("src-tauri/src/commands.rs");
  });

  it("reports a pending scope as pending, not pass", () => {
    const scope = { ...rustScope, name: "rust-shared", status: "pending" };

    const result = evaluateScope(
      scope,
      loadReport(scope, "/nonexistent-root"),
      [],
    );

    expect(result.status).toBe("pending");
  });

  it("fails the frontend scope when branches are at exactly 80%", () => {
    const { dir } = writeTemp(
      "summary.json",
      vitestReport([["/repo/src/app.ts", [100, 100], [10, 8]]]),
    );
    const scope = { ...frontendScope, report: "summary.json" };

    const result = evaluateScope(scope, loadReport(scope, dir, "/repo"), [
      "src/app.ts",
    ]);

    expect(result.status).toBe("fail");
    expect(result.metrics).toContainEqual({
      metric: "branches",
      covered: 8,
      total: 10,
    });
  });

  it("reports the justification for every excluded path", () => {
    const scope = {
      ...rustScope,
      justifiedExclusions: [
        {
          path: "src-tauri/src/main.rs",
          reason: "entry point needs a window event loop",
        },
      ],
    };
    const report = {
      files: { "src-tauri/src/lib.rs": { lines: { covered: 3, total: 3 } } },
    };

    const result = evaluateScope(scope, report, [
      "src-tauri/src/lib.rs",
      "src-tauri/src/main.rs",
    ]);

    expect(result.status).toBe("pass");
    expect(result.notes).toEqual([
      "excluded src-tauri/src/main.rs: entry point needs a window event loop",
    ]);
  });

  it("ignores test files and files outside the scope include", () => {
    const { dir } = writeTemp(
      "summary.json",
      vitestReport([
        ["/repo/src/app.ts", [10, 9], [2, 2]],
        ["/repo/src/app.test.ts", [10, 0], [2, 0]],
        ["/repo/scripts/tool.mjs", [10, 0], [2, 0]],
      ]),
    );
    const scope = { ...frontendScope, report: "summary.json" };

    const result = evaluateScope(scope, loadReport(scope, dir, "/repo"), [
      "src/app.ts",
      "src/app.test.ts",
    ]);

    expect(result.status).toBe("pass");
    expect(result.metrics).toContainEqual({
      metric: "lines",
      covered: 9,
      total: 10,
    });
  });
});
