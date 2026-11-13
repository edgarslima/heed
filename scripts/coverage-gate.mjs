// Coverage gate for the rules in doc/standard/coding-practices-desktop-audio-recorder.md
// section 6: every active scope must have covered * 100 > total * 80 on integer counts.
// Missing, empty, or invalid reports, scopes with zero measured items, and production
// files absent from a report all fail. Uses Node built-ins only.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { isAbsolute, join, matchesGlob, relative, sep } from "node:path";
import { argv, cwd, exit } from "node:process";
import { fileURLToPath } from "node:url";

const THRESHOLD_PERCENT = 80;

/** True when covered / total is strictly above the threshold, using integers only. */
export function passesThreshold(covered, total) {
  return covered * 100 > total * THRESHOLD_PERCENT;
}

function toRepoPath(filename, sourceRoot) {
  const path = isAbsolute(filename) ? relative(sourceRoot, filename) : filename;
  return path.split(sep).join("/");
}

function parseLlvmCov(data, sourceRoot) {
  const files = data?.data?.[0]?.files;
  if (!Array.isArray(files)) {
    return { error: "report has no data[0].files array" };
  }
  const result = {};
  for (const file of files) {
    const lines = file.summary?.lines;
    result[toRepoPath(file.filename, sourceRoot)] = {
      lines: { covered: lines?.covered ?? 0, total: lines?.count ?? 0 },
    };
  }
  return { files: result };
}

function parseVitest(data, sourceRoot) {
  if (data === null || typeof data !== "object" || !("total" in data)) {
    return { error: "report has no total entry" };
  }
  const result = {};
  for (const [filename, entry] of Object.entries(data)) {
    if (filename === "total") continue;
    result[toRepoPath(filename, sourceRoot)] = {
      lines: {
        covered: entry.lines?.covered ?? 0,
        total: entry.lines?.total ?? 0,
      },
      branches: {
        covered: entry.branches?.covered ?? 0,
        total: entry.branches?.total ?? 0,
      },
    };
  }
  return { files: result };
}

/**
 * Reads a scope's report from reportRoot. File names are made relative to sourceRoot.
 * Returns { files } on success or { error } when the report is missing or unusable.
 */
export function loadReport(scope, reportRoot, sourceRoot = reportRoot) {
  const path = join(reportRoot, scope.report);
  let text;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return { error: `report not found: ${scope.report}` };
  }
  if (text.trim() === "") {
    return { error: `report is empty: ${scope.report}` };
  }
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    return { error: `report is not valid JSON: ${scope.report}` };
  }
  if (scope.kind === "llvm-cov") return parseLlvmCov(data, sourceRoot);
  if (scope.kind === "vitest") return parseVitest(data, sourceRoot);
  return { error: `unsupported report kind: ${scope.kind}` };
}

// `exclude` holds paths the coding practices exclude by rule (tests, fixtures,
// generated code). `justifiedExclusions` holds reviewed exceptions with a reason.
function inScope(scope, path) {
  const included = scope.include.some((glob) => matchesGlob(path, glob));
  const excluded = [
    ...(scope.exclude ?? []),
    ...(scope.justifiedExclusions ?? []).map((entry) => entry.path),
  ].some((glob) => matchesGlob(path, glob));
  return included && !excluded;
}

/** Applies the gate rules to one scope. sourceFiles are repository-relative paths. */
export function evaluateScope(scope, report, sourceFiles) {
  const notes = (scope.justifiedExclusions ?? []).map(
    ({ path, reason }) => `excluded ${path}: ${reason}`,
  );
  const base = { name: scope.name, metrics: [], reasons: [], notes };
  if (scope.status === "pending") {
    return { ...base, status: "pending" };
  }
  if ("error" in report) {
    return { ...base, status: "fail", reasons: [report.error] };
  }

  const reported = Object.keys(report.files).filter((path) =>
    inScope(scope, path),
  );
  const reasons = [];
  for (const path of sourceFiles.filter((file) => inScope(scope, file))) {
    if (!reported.includes(path)) {
      reasons.push(`production file missing from report: ${path}`);
    }
  }

  const metrics = scope.metrics.map((metric) => {
    let covered = 0;
    let total = 0;
    for (const path of reported) {
      covered += report.files[path][metric]?.covered ?? 0;
      total += report.files[path][metric]?.total ?? 0;
    }
    return { metric, covered, total };
  });

  const [primary, ...secondary] = metrics;
  if (primary === undefined || primary.total === 0) {
    reasons.push("active scope has zero measured items");
  } else if (!passesThreshold(primary.covered, primary.total)) {
    reasons.push(
      `${primary.metric} ${primary.covered}/${primary.total} is not above 80%`,
    );
  }
  for (const metric of secondary) {
    // A secondary metric with no items (for example, code without branches) has nothing to cover.
    if (metric.total > 0 && !passesThreshold(metric.covered, metric.total)) {
      reasons.push(
        `${metric.metric} ${metric.covered}/${metric.total} is not above 80%`,
      );
    }
  }

  return {
    ...base,
    metrics,
    reasons,
    status: reasons.length === 0 ? "pass" : "fail",
  };
}

function listSourceFiles(root) {
  const output = execFileSync(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard"],
    {
      cwd: root,
      encoding: "utf8",
    },
  );
  return output.split("\n").filter((line) => line !== "");
}

function main() {
  const root = cwd();
  const configPath = join(root, "scripts", "coverage-scopes.json");
  const { scopes } = JSON.parse(readFileSync(configPath, "utf8"));
  const sourceFiles = listSourceFiles(root);

  let failed = false;
  for (const scope of scopes) {
    const report =
      scope.status === "pending" ? { files: {} } : loadReport(scope, root);
    const result = evaluateScope(scope, report, sourceFiles);
    const counts = result.metrics
      .map((m) => `${m.metric} ${m.covered}/${m.total}`)
      .join(", ");
    console.log(
      `${result.status.toUpperCase()} ${result.name}${counts ? ` — ${counts}` : ""}`,
    );
    for (const reason of result.reasons) console.log(`  - ${reason}`);
    for (const note of result.notes) console.log(`  * ${note}`);
    if (result.status === "fail") failed = true;
  }
  exit(failed ? 1 : 0);
}

if (argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
