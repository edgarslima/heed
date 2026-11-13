// Coverage gate for the rules in doc/standard/coding-practices-desktop-audio-recorder.md
// section 6: every active scope must have covered * 100 > total * 80 on integer counts.
// Missing, empty, or invalid reports, scopes with zero measured items, and production
// files absent from a report all fail, as do invalid scope definitions, unjustified
// exclusions, coverage pragmas in production sources, and pending scopes that already
// have measured code. Uses Node built-ins only.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { isAbsolute, join, matchesGlob, relative, sep } from "node:path";
import { cwd, exit } from "node:process";

const THRESHOLD_PERCENT = 80;
const KINDS = ["llvm-cov", "vitest", "xccov"];
const STATUSES = ["active", "pending"];
const METRICS = ["lines", "branches"];
const GLOB_CHARACTERS = /[*?[\]{}]/;
// In-source markers that hide code from V8/Istanbul or llvm-cov instrumentation.
const COVERAGE_PRAGMAS = [
  /\b(?:v8|c8|istanbul)\s+ignore\b/,
  /cfg\(\s*(?:not\(\s*)?coverage\b/,
  /\bcoverage\s*\(\s*off\s*\)/,
];

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

// Audited path lists (justified exclusions, declarative files) need literal paths
// that exist in the source inventory and a non-empty reason.
function validateAuditedPaths(entries, label, sourceFiles, reasons) {
  for (const entry of entries ?? []) {
    const path = String(entry.path);
    if (GLOB_CHARACTERS.test(path)) {
      reasons.push(`${label} must be a literal path: ${path}`);
    } else if (!sourceFiles.includes(path)) {
      reasons.push(`${label} matches no source file: ${path}`);
    }
    if (typeof entry.reason !== "string" || entry.reason.trim() === "") {
      reasons.push(`${label} needs a reason: ${path}`);
    }
  }
}

function validateScope(scope, sourceFiles) {
  const reasons = [];
  if (!KINDS.includes(scope.kind)) reasons.push(`unknown kind: ${scope.kind}`);
  if (!STATUSES.includes(scope.status)) {
    reasons.push(`unknown status: ${scope.status}`);
  }
  if (!Array.isArray(scope.include) || scope.include.length === 0) {
    reasons.push("include must list at least one glob");
  }
  if (!Array.isArray(scope.metrics) || scope.metrics.length === 0) {
    reasons.push("metrics must list at least one metric");
  } else {
    for (const metric of scope.metrics.filter((m) => !METRICS.includes(m))) {
      reasons.push(`unknown metric: ${metric}`);
    }
  }
  validateAuditedPaths(
    scope.justifiedExclusions,
    "justified exclusion",
    sourceFiles,
    reasons,
  );
  validateAuditedPaths(
    scope.declarativeFiles,
    "declarative file",
    sourceFiles,
    reasons,
  );
  return reasons;
}

function exclusionNotes(scope, report, sourceFiles) {
  const notes = [];
  for (const pattern of scope.exclude ?? []) {
    const removed = sourceFiles.filter(
      (file) =>
        scope.include.some((glob) => matchesGlob(file, glob)) &&
        matchesGlob(file, pattern),
    );
    if (removed.length > 0) {
      notes.push(`excluded by rule ${pattern}: ${removed.join(", ")}`);
    }
  }
  const primary = scope.metrics?.[0];
  for (const { path, reason } of scope.justifiedExclusions ?? []) {
    const counts = report.files?.[path]?.[primary];
    const measured = counts
      ? `${primary} ${counts.covered}/${counts.total}`
      : "not in report";
    notes.push(`excluded ${path} (${measured}): ${reason}`);
  }
  for (const { path, reason } of scope.declarativeFiles ?? []) {
    notes.push(`declarative ${path}: ${reason}`);
  }
  return notes;
}

function readSourceFromDisk(path) {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return "";
  }
}

/**
 * Applies the gate rules to one scope. sourceFiles are repository-relative paths;
 * readSource returns a source file's text for the coverage pragma check.
 */
export function evaluateScope(
  scope,
  report,
  sourceFiles,
  readSource = readSourceFromDisk,
) {
  const base = { name: scope.name, metrics: [], reasons: [], notes: [] };
  const invalid = validateScope(scope, sourceFiles);
  if (invalid.length > 0) {
    return { ...base, status: "fail", reasons: invalid };
  }
  const notes = exclusionNotes(scope, report, sourceFiles);

  if (scope.status === "pending") {
    // A pending scope must be activated as soon as its report measures real code.
    const measured = Object.keys(report.files ?? {}).some(
      (path) =>
        inScope(scope, path) &&
        (report.files[path][scope.metrics[0]]?.total ?? 0) > 0,
    );
    return measured
      ? {
          ...base,
          notes,
          status: "fail",
          reasons: ["pending scope has measured code; activate it"],
        }
      : { ...base, notes, status: "pending" };
  }
  if ("error" in report) {
    return { ...base, notes, status: "fail", reasons: [report.error] };
  }

  const reported = Object.keys(report.files).filter((path) =>
    inScope(scope, path),
  );
  const reasons = [];
  // llvm-cov omits files without functions (module declarations only); those are
  // listed as declarative and must stay unmeasured.
  const declarative = (scope.declarativeFiles ?? []).map((entry) => entry.path);
  for (const path of declarative) {
    if ((report.files[path]?.lines?.total ?? 0) > 0) {
      reasons.push(`declarative file has measured code: ${path}`);
    }
  }
  for (const path of sourceFiles.filter((file) => inScope(scope, file))) {
    if (COVERAGE_PRAGMAS.some((pragma) => pragma.test(readSource(path)))) {
      reasons.push(`coverage pragma in ${path}`);
    }
  }
  for (const path of sourceFiles.filter((file) => inScope(scope, file))) {
    if (!reported.includes(path) && !declarative.includes(path)) {
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
    notes,
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
    const report = loadReport(scope, root);
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

// import.meta.main is true only when this file is the entry point, including when it
// is started through a symlinked path.
if (import.meta.main) {
  main();
}
