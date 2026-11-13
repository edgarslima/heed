import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
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

  it("reports the reason and measured counts for a justified exclusion", () => {
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
      files: {
        "src-tauri/src/lib.rs": { lines: { covered: 3, total: 3 } },
        "src-tauri/src/main.rs": { lines: { covered: 0, total: 8 } },
      },
    };

    const result = evaluateScope(scope, report, [
      "src-tauri/src/lib.rs",
      "src-tauri/src/main.rs",
    ]);

    expect(result.status).toBe("pass");
    expect(result.notes).toContain(
      "excluded src-tauri/src/main.rs (lines 0/8): entry point needs a window event loop",
    );
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

describe("scope validation", () => {
  const report = {
    files: { "src-tauri/src/lib.rs": { lines: { covered: 9, total: 10 } } },
  };

  it("fails a scope with an unknown metric name", () => {
    const scope = { ...rustScope, metrics: ["lines", "branch"] };

    const result = evaluateScope(scope, report, ["src-tauri/src/lib.rs"]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("unknown metric: branch");
  });

  it("fails a scope with an unknown kind or status", () => {
    for (const scope of [
      { ...rustScope, kind: "lcov" },
      { ...rustScope, status: "skipped" },
    ]) {
      expect(
        evaluateScope(scope, report, ["src-tauri/src/lib.rs"]).status,
      ).toBe("fail");
    }
  });

  it("rejects a justified exclusion with a glob path", () => {
    const scope = {
      ...rustScope,
      justifiedExclusions: [{ path: "src-tauri/src/m*.rs", reason: "entry" }],
    };

    const result = evaluateScope(scope, report, [
      "src-tauri/src/lib.rs",
      "src-tauri/src/main.rs",
    ]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("must be a literal path");
  });

  it("rejects a justified exclusion with an empty or missing reason", () => {
    for (const entry of [
      { path: "src-tauri/src/main.rs", reason: "  " },
      { path: "src-tauri/src/main.rs" },
    ]) {
      const scope = { ...rustScope, justifiedExclusions: [entry] };

      const result = evaluateScope(scope, report, [
        "src-tauri/src/lib.rs",
        "src-tauri/src/main.rs",
      ]);

      expect(result.status).toBe("fail");
      expect(result.reasons.join(" ")).toContain("needs a reason");
    }
  });

  it("rejects a justified exclusion that matches no source file", () => {
    const scope = {
      ...rustScope,
      justifiedExclusions: [{ path: "src-tauri/src/gone.rs", reason: "entry" }],
    };

    const result = evaluateScope(scope, report, ["src-tauri/src/lib.rs"]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("matches no source file");
  });

  it("lists the files removed by each exclude pattern", () => {
    const vitest = {
      files: {
        "src/app.ts": {
          lines: { covered: 9, total: 10 },
          branches: { covered: 2, total: 2 },
        },
      },
    };

    const result = evaluateScope(frontendScope, vitest, [
      "src/app.ts",
      "src/app.test.ts",
    ]);

    expect(result.notes).toContain(
      "excluded by rule **/*.test.ts: src/app.test.ts",
    );
  });

  it("fails when an in-scope source contains a coverage pragma", () => {
    const sources = {
      "src-tauri/src/lib.rs": "#[cfg(not(coverage))]\nfn hidden() {}\n",
      "src/app.ts": "/* v8 ignore next */\nexport const x = 1;\n",
    };
    const readSource = (path) => sources[path] ?? "";
    const vitest = {
      files: {
        "src/app.ts": {
          lines: { covered: 9, total: 10 },
          branches: { covered: 2, total: 2 },
        },
      },
    };

    const rust = evaluateScope(
      rustScope,
      report,
      ["src-tauri/src/lib.rs"],
      readSource,
    );
    const frontend = evaluateScope(
      frontendScope,
      vitest,
      ["src/app.ts"],
      readSource,
    );

    expect(rust.status).toBe("fail");
    expect(rust.reasons.join(" ")).toContain(
      "coverage pragma in src-tauri/src/lib.rs",
    );
    expect(frontend.status).toBe("fail");
    expect(frontend.reasons.join(" ")).toContain(
      "coverage pragma in src/app.ts",
    );
  });

  it("fails a pending scope whose report already has measured code", () => {
    const scope = { ...rustScope, name: "rust-shared", status: "pending" };

    const result = evaluateScope(scope, report, ["src-tauri/src/lib.rs"]);

    expect(result.status).toBe("fail");
    expect(result.reasons.join(" ")).toContain("activate it");
  });
});

describe("command line", () => {
  it("exits 1 with a missing report when run through a symlinked path", () => {
    const repo = mkdtempSync(join(tmpdir(), "coverage-gate-repo-"));
    spawnSync("git", ["init", "-q"], { cwd: repo });
    mkdirSync(join(repo, "scripts"));
    mkdirSync(join(repo, "src"));
    writeFileSync(join(repo, "src", "app.ts"), "export const x = 1;\n");
    writeFileSync(
      join(repo, "scripts", "coverage-scopes.json"),
      JSON.stringify({
        scopes: [{ ...frontendScope, report: "coverage/missing.json" }],
      }),
    );
    const linkParent = mkdtempSync(join(tmpdir(), "coverage-gate-link-"));
    const link = join(linkParent, "scripts");
    symlinkSync(resolve("scripts"), link, "junction");

    const run = spawnSync(process.execPath, [join(link, "coverage-gate.mjs")], {
      cwd: repo,
      encoding: "utf8",
    });

    expect(run.status).toBe(1);
    expect(run.stdout).toContain("FAIL frontend");
  });
});
