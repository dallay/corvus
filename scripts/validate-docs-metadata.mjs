import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(scriptPath), "..");
const docsRootRelative = "clients/web/apps/docs/src/content/docs";
const docsRoot = path.join(repoRoot, docsRootRelative);
const allowedStatuses = new Set(["canonical", "draft", "deprecated"]);
const allowedDocTypes = new Set(["guide", "reference", "architecture", "runbook"]);
const maxReviewAgeDaysByDocType = {
  architecture: 120,
  guide: 90,
  reference: 90,
  runbook: 60,
};
const args = process.argv.slice(2);

function normalizePathForMatch(filePath) {
  return filePath.replaceAll(path.sep, "/");
}

function getSidebarSlugs() {
  const astroConfigPath = path.join(repoRoot, "clients/web/apps/docs/astro.config.mjs");
  const astroConfig = readFileSync(astroConfigPath, "utf8");
  const slugs = new Set();

  for (const match of astroConfig.matchAll(/slug:\s*"([^"]+)"/g)) {
    slugs.add(match[1]);
    slugs.add(`es/${match[1]}`);
  }

  return slugs;
}

let _sidebarSlugs;
function lazyGetSidebarSlugs() {
  if (!_sidebarSlugs) {
    _sidebarSlugs = getSidebarSlugs();
  }
  return _sidebarSlugs;
}

function walk(directory) {
  const entries = readdirSync(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      files.push(...walk(fullPath));
      continue;
    }

    if (entry.isFile() && /\.(md|mdx)$/.test(entry.name)) {
      files.push(fullPath);
    }
  }

  return files;
}

function getChangedDocsFiles() {
  const eventName = process.env.GITHUB_EVENT_NAME;
  const baseRef = process.env.GITHUB_BASE_REF;
  const beforeSha = process.env.GITHUB_EVENT_BEFORE;
  const currentSha = process.env.GITHUB_SHA;

  try {
    let diffRange;

    if (eventName === "pull_request" && baseRef) {
      execFileSync("git", ["fetch", "origin", baseRef, "--depth=1"], {
        cwd: repoRoot,
        stdio: "ignore",
      });
      diffRange = `origin/${baseRef}...HEAD`;
    } else if (eventName === "push" && beforeSha && currentSha && !/^0+$/.test(beforeSha)) {
      diffRange = `${beforeSha}...${currentSha}`;
    }

    if (!diffRange) {
      const trackedOutput = execFileSync("git", ["diff", "--name-only", "HEAD", "--", docsRootRelative], {
        encoding: "utf8",
        cwd: repoRoot,
      });
      const untrackedOutput = execFileSync(
        "git",
        ["ls-files", "--others", "--exclude-standard", "--", docsRootRelative],
        {
          encoding: "utf8",
          cwd: repoRoot,
        },
      );

      return [...trackedOutput.split("\n"), ...untrackedOutput.split("\n")]
        .map((file) => file.trim())
        .filter(Boolean)
        .filter((file) => /\.(md|mdx)$/.test(file))
        .map((file) => path.resolve(repoRoot, file))
        .filter((file) => existsSync(file));
    }

    // Check all changed files (not just docs) for config/sidebar/validator changes.
    const allOutput = execFileSync("git", ["diff", "--name-only", diffRange], {
      encoding: "utf8",
      cwd: repoRoot,
    });
    const allChanged = allOutput.split("\n").map((f) => f.trim()).filter(Boolean);
    const configPatterns = [
      "astro.config.mjs",
      "validate-docs-metadata",
      "content.config",
    ];
    const hasConfigChange = allChanged.some(
      (f) => !f.startsWith(docsRootRelative) || configPatterns.some((p) => f.includes(p)),
    );
    const hasDeletedDocs = allChanged
      .filter((f) => f.startsWith(docsRootRelative) && /\.(md|mdx)$/.test(f))
      .some((f) => !existsSync(path.resolve(repoRoot, f)));

    if (hasConfigChange || hasDeletedDocs) {
      return walk(docsRoot);
    }

    const output = execFileSync("git", ["diff", "--name-only", diffRange, "--", docsRootRelative], {
      encoding: "utf8",
      cwd: repoRoot,
    });

    return output
      .split("\n")
      .map((file) => file.trim())
      .filter(Boolean)
      .filter((file) => /\.(md|mdx)$/.test(file))
      .map((file) => path.resolve(repoRoot, file))
      .filter((file) => existsSync(file));
  } catch (error) {
    console.error("Failed to determine changed docs files via git, falling back to full walk:", error);
    return walk(docsRoot);
  }
}

function extractFrontmatter(contents) {
  const match = contents.match(/^---\n([\s\S]*?)\n---/);
  return match ? match[1] : null;
}

function getField(frontmatter, fieldName) {
  const regex = new RegExp(String.raw`^${fieldName}:\s*(.+)$`, "m");
  const match = frontmatter.match(regex);
  return match ? match[1].trim().replaceAll(/^['"]|['"]$/g, "") : "";
}

function getMetadata(filePath) {
  const contents = readFileSync(filePath, "utf8");
  const frontmatter = extractFrontmatter(contents);

  if (!frontmatter) {
    return null;
  }

  return {
    description: getField(frontmatter, "description"),
    owner: getField(frontmatter, "owner"),
    status: getField(frontmatter, "status"),
    lastReviewed: getField(frontmatter, "lastReviewed"),
    appliesTo: getField(frontmatter, "appliesTo"),
    docType: getField(frontmatter, "docType"),
    slug: getField(frontmatter, "slug"),
  };
}

function getRouteSlug(filePath, metadata) {
  if (metadata?.slug) {
    return metadata.slug.replace(/\/$/, "");
  }

  const relativePath = path.relative(docsRoot, filePath);
  const withoutExtension = relativePath.replace(/\.(md|mdx)$/, "");
  return withoutExtension.replace(/\/index$/, "").replaceAll(path.sep, "/");
}

function isExternalOrEmpty(target) {
  return !target || /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(target);
}

function isWithinDocsRoot(candidate) {
  const resolvedCandidate = path.resolve(candidate);
  const relative = path.relative(docsRoot, resolvedCandidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function collectMarkdownLinkSlugs(contents, fileDirectory, referenced) {
  for (const match of contents.matchAll(/\[[^\]]{0,500}\]\(([^)]{0,500})\)/g)) {
    const target = match[1].split("#")[0].trim();

    if (isExternalOrEmpty(target)) {
      continue;
    }

    if (target.startsWith("/")) {
      referenced.add(target.replace(/^\//, "").replace(/\/$/, ""));
      continue;
    }

    const resolved = path.resolve(fileDirectory, target);
    const candidates = [resolved, `${resolved}.md`, `${resolved}.mdx`, path.join(resolved, "index.mdx"), path.join(resolved, "index.md")]
      .filter((candidate) => isWithinDocsRoot(candidate));
    const existingCandidate = candidates.find(
      (candidate) => existsSync(candidate) && statSync(candidate).isFile(),
    );

    if (existingCandidate) {
      const metadata = getMetadata(existingCandidate);
      if (metadata) {
        referenced.add(getRouteSlug(existingCandidate, metadata));
      }
    }
  }
}

function collectFrontmatterLinkSlugs(contents, referenced) {
  for (const match of contents.matchAll(/\blink:\s*([^\s]+)/g)) {
    const target = match[1].trim().replaceAll(/^['"]|['"]$/g, "").replace(/\/$/, "");
    if (!isExternalOrEmpty(target)) {
      referenced.add(target.replace(/^\//, ""));
    }
  }
}

function collectReferencedSlugs() {
  const referenced = new Set();
  const files = walk(docsRoot).filter((filePath) => !isExempt(filePath));

  for (const filePath of files) {
    const contents = readFileSync(filePath, "utf8");
    const fileDirectory = path.dirname(filePath);
    collectMarkdownLinkSlugs(contents, fileDirectory, referenced);
    collectFrontmatterLinkSlugs(contents, referenced);
  }

  return referenced;
}

let _referencedSlugs;
function lazyGetReferencedSlugs() {
  if (!_referencedSlugs) {
    _referencedSlugs = collectReferencedSlugs();
  }
  return _referencedSlugs;
}

function getParityPartnerPath(filePath) {
  const relativePath = path.relative(docsRoot, filePath);

  if (relativePath.startsWith(`es${path.sep}`)) {
    return path.join(docsRoot, relativePath.replace(`es${path.sep}`, ""));
  }

  return path.join(docsRoot, "es", relativePath);
}

function validateParity(filePath, metadata) {
  const relativePath = path.relative(repoRoot, filePath);
  const parityPath = getParityPartnerPath(filePath);
  const parityRelativePath = path.relative(repoRoot, parityPath);
  const errors = [];

  if (!existsSync(parityPath)) {
    errors.push(`${relativePath}: missing locale counterpart '${parityRelativePath}'`);
    return errors;
  }

  const parityMetadata = getMetadata(parityPath);

  if (!parityMetadata) {
    errors.push(`${relativePath}: locale counterpart '${parityRelativePath}' is missing frontmatter`);
    return errors;
  }

  for (const field of ["status", "appliesTo", "docType"]) {
    if (metadata[field] !== parityMetadata[field]) {
      errors.push(
        `${relativePath}: field '${field}' must match locale counterpart '${parityRelativePath}'`,
      );
    }
  }

  for (const field of ["description", "owner", "lastReviewed"]) {
    if (!parityMetadata[field]) {
      errors.push(
        `${relativePath}: locale counterpart '${parityRelativePath}' is missing required field '${field}'`,
      );
    }
  }

  return errors;
}

function isExempt(filePath) {
  const normalized = normalizePathForMatch(filePath);
  return normalized.endsWith("/404.mdx");
}

function isOrphanExempt(filePath, metadata) {
  const normalized = normalizePathForMatch(filePath);

  if (isExempt(filePath)) {
    return true;
  }

  if (metadata?.status === "draft" || metadata?.status === "deprecated") {
    return true;
  }

  return [
    "/src/content/docs/index.mdx",
    "/src/content/docs/es/index.mdx",
    "/src/content/docs/intro/introduction.mdx",
    "/src/content/docs/es/intro/introduction.mdx",
  ].some((suffix) => normalized.endsWith(suffix));
}

function validateOrphanStatus(filePath, metadata) {
  if (isOrphanExempt(filePath, metadata)) {
    return [];
  }

  const slug = getRouteSlug(filePath, metadata);

  if (lazyGetSidebarSlugs().has(slug) || lazyGetReferencedSlugs().has(slug)) {
    return [];
  }

  return [
    `${path.relative(repoRoot, filePath)}: appears to be orphaned (not present in sidebar and not referenced by other docs)`,
  ];
}

function validateReviewDate(relativePath, lastReviewed, status, docType) {
  const reviewedAt = new Date(`${lastReviewed}T00:00:00Z`);

  if (Number.isNaN(reviewedAt.getTime())) {
    return [`${relativePath}: invalid lastReviewed '${lastReviewed}'`];
  }

  if (reviewedAt.getTime() > Date.now()) {
    return [`${relativePath}: lastReviewed '${lastReviewed}' is in the future`];
  }

  if (status !== "deprecated") {
    const ageMs = Date.now() - reviewedAt.getTime();
    const ageDays = Math.floor(ageMs / (1000 * 60 * 60 * 24));
    const maxReviewAgeDays = maxReviewAgeDaysByDocType[docType] ?? 90;
    if (ageDays > maxReviewAgeDays) {
      return [
        `${relativePath}: lastReviewed is ${ageDays} days old, must be <= ${maxReviewAgeDays} days for docType '${docType}'`,
      ];
    }
  }

  return [];
}

function validateLastReviewed(relativePath, lastReviewed, status, docType) {
  if (!lastReviewed) {
    return [`${relativePath}: missing required field 'lastReviewed'`];
  }

  if (/^\d{4}-\d{2}-\d{2}$/.test(lastReviewed)) {
    return validateReviewDate(relativePath, lastReviewed, status, docType);
  }

  return [`${relativePath}: invalid lastReviewed '${lastReviewed}', expected YYYY-MM-DD`];
}

function validateFile(filePath) {
  const relativePath = path.relative(repoRoot, filePath);
  const errors = [];

  if (isExempt(filePath)) {
    return errors;
  }

  const contents = readFileSync(filePath, "utf8");
  const frontmatter = extractFrontmatter(contents);

  if (!frontmatter) {
    return [`${relativePath}: missing frontmatter block`];
  }

  const metadata = getMetadata(filePath);
  const { description, owner, status, lastReviewed, appliesTo, docType } = metadata;

  if (!description) {
    errors.push(`${relativePath}: missing required field 'description'`);
  }

  if (!owner) {
    errors.push(`${relativePath}: missing required field 'owner'`);
  }

  if (!status) {
    errors.push(`${relativePath}: missing required field 'status'`);
  } else if (!allowedStatuses.has(status)) {
    errors.push(
      `${relativePath}: invalid status '${status}', expected one of ${[...allowedStatuses].join(", ")}`,
    );
  }

  errors.push(...validateLastReviewed(relativePath, lastReviewed, status, docType));

  if (!appliesTo) {
    errors.push(`${relativePath}: missing required field 'appliesTo'`);
  }

  if (!docType) {
    errors.push(`${relativePath}: missing required field 'docType'`);
  } else if (!allowedDocTypes.has(docType)) {
    errors.push(
      `${relativePath}: invalid docType '${docType}', expected one of ${[...allowedDocTypes].join(", ")}`,
    );
  }

  errors.push(
    ...validateParity(filePath, metadata),
    ...validateOrphanStatus(filePath, metadata),
  );

  return errors;
}

function resolveTargetFiles() {
  const explicitFiles = args
    .map((value) => path.resolve(repoRoot, value))
    .filter((filePath) => statSync(filePath, { throwIfNoEntry: false })?.isFile());

  return explicitFiles.length > 0 ? explicitFiles : getChangedDocsFiles();
}

const files = resolveTargetFiles();

if (files.length === 0) {
  console.log("No documentation files changed; metadata validation skipped.");
  process.exit(0);
}

const failures = files.flatMap(validateFile);

if (failures.length > 0) {
  console.error("Documentation metadata validation failed:\n");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Documentation metadata validation passed for ${files.length} file(s).`);
