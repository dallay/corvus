#!/bin/sh
# ============================================
# Git pre-commit hook
# 1) Check whether newly added staged lines contain sensitive keywords
# 2) Check project links with lychee
# ============================================

PART1="TO"
PART2="DO"
KEYWORDS="${PART1}${PART2}"

# Get newly added lines from staged changes
DIFF_CONTENT=$(git diff --cached --unified=0 | grep '^+' | grep -v '^+++' || true)

HAS_FORBIDDEN=0

if [ -n "$DIFF_CONTENT" ]; then
  for KEY in $KEYWORDS; do
    if echo "$DIFF_CONTENT" | grep -i "$KEY" >/dev/null 2>&1; then
      echo "❌ ERROR: detected forbidden keyword in added lines: '$KEY'"
      HAS_FORBIDDEN=1
    fi
  done
fi

if [ "$HAS_FORBIDDEN" -ne 0 ]; then
  echo "🚫 Commit blocked: please remove sensitive content."
  exit 1
fi

if ! command -v lychee >/dev/null 2>&1; then
  echo "❌ ERROR: 'lychee' is required for link validation but was not found."
  echo "Install: https://github.com/lycheeverse/lychee"
  exit 1
fi

echo "🔗 Running lychee link check..."
if ! lychee --no-progress --max-retries 2 --retry-wait-time 2 --exclude-all-private .; then
  echo "🚫 Commit blocked: broken links detected by lychee."
  exit 1
fi

echo "✅ pre-commit check passed."
exit 0
