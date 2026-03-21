#!/bin/sh
# ============================================
# Git pre-commit hook
# 1) Auto-format staged Kotlin/Gradle sources with Spotless
# 2) Check whether newly added staged lines contain sensitive keywords
# 3) Check project links with lychee
# ============================================

if [ "${SKIP_GIT_HOOKS:-0}" = "1" ]; then
  echo "⏭️  Skipping pre-commit hook (SKIP_GIT_HOOKS=1)"
  exit 0
fi

PART1="TO"
PART2="DO"
KEYWORDS="${PART1}${PART2}"

FORMAT_FILE_REGEX='\.(kt|kts|java|gradle\.kts)$'
STAGED_FORMAT_FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep -E "$FORMAT_FILE_REGEX" || true)

if [ -n "$STAGED_FORMAT_FILES" ]; then
  echo "🎨 Running spotlessApply on repository..."
  ./gradlew spotlessApply

  echo "$STAGED_FORMAT_FILES" | while IFS= read -r FILE; do
    if [ -n "$FILE" ] && [ -e "$FILE" ]; then
      git add -- "$FILE"
    fi
  done
fi

# Only scan staged source files (skip docs like .md)
SOURCE_FILE_REGEX='\.(kt|kts|java|groovy|gradle|xml|properties|toml|ya?ml|json|rs|swift|[cm]|cc|cpp|h|hpp|js|jsx|ts|tsx|py|rb|go|sh)$'
STAGED_SOURCE_FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep -E "$SOURCE_FILE_REGEX" || true)

HAS_FORBIDDEN=0

if [ -n "$STAGED_SOURCE_FILES" ]; then
  for FILE in $STAGED_SOURCE_FILES; do
    # Get newly added lines from staged changes for each source file
    DIFF_CONTENT=$(git diff --cached --unified=0 -- "$FILE" | grep '^+' | grep -v '^+++' || true)
    if [ -n "$DIFF_CONTENT" ]; then
      for KEY in $KEYWORDS; do
        if echo "$DIFF_CONTENT" | grep -i -w "$KEY" >/dev/null 2>&1; then
          echo "❌ ERROR: detected forbidden keyword in added lines: '$KEY' (file: $FILE)" >&2
          HAS_FORBIDDEN=1
        fi
      done
    fi
  done
fi

if [ "$HAS_FORBIDDEN" -ne 0 ]; then
  echo "🚫 Commit blocked: please remove sensitive content."
  exit 1
fi

LINK_FILE_REGEX='\.(md|mdx|html?|ya?ml|toml)$'
STAGED_LINK_FILES=$(git diff --cached --name-only --diff-filter=ACMR | grep -E "$LINK_FILE_REGEX" || true)

if [ -z "$STAGED_LINK_FILES" ]; then
  echo "✅ pre-commit check passed."
  exit 0
fi

if ! command -v lychee >/dev/null 2>&1; then
  echo "⚠️  WARN: 'lychee' was not found. Skipping staged link validation." >&2
  echo "Install: https://github.com/lycheeverse/lychee" >&2
  echo "✅ pre-commit check passed."
  exit 0
fi

echo "🔗 Running lychee offline check on staged docs/config files..."
if ! lychee --config "lychee.toml" --offline --no-progress $STAGED_LINK_FILES; then
  echo "🚫 Commit blocked: broken local links detected by lychee." >&2
  echo "   External links stay in the scheduled/manual GitHub Actions audit." >&2
  exit 1
fi

echo "✅ pre-commit check passed."
exit 0
