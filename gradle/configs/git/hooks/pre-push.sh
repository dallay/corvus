#!/bin/sh
# ============================================
# Diff-aware pre-push hook
#
# Philosophy: pre-push catches obvious breakage FAST.
# CI is the real quality gate. This hook only runs checks
# relevant to the files you actually changed.
#
# Bypass: SKIP_GIT_HOOKS=1 git push
# Full:   FULL_PRE_PUSH=1 git push  (runs everything like CI)
# ============================================
set -e

if [ "${SKIP_GIT_HOOKS:-0}" = "1" ]; then
    echo "⏭️  Skipping pre-push hook (SKIP_GIT_HOOKS=1)"
    exit 0
fi

echo "🚀 Pre-push check start"

# ── Detect what changed vs the remote tracking branch ──────────
REMOTE_REF=$(git rev-parse --abbrev-ref '@{upstream}' 2>/dev/null || echo "origin/main")
CHANGED_FILES=$(git diff --name-only "$REMOTE_REF"...HEAD 2>/dev/null || git diff --name-only HEAD~1)

HAS_RUST=0
HAS_KOTLIN=0
HAS_WEB=0
HAS_DOCS=0
HAS_GRADLE_CONFIG=0

for f in $CHANGED_FILES; do
    case "$f" in
        clients/agent-runtime/*) HAS_RUST=1 ;;
        clients/composeApp/*|modules/agent-core-kmp/*|clients/androidApp/*) HAS_KOTLIN=1 ;;
        clients/web/*) HAS_WEB=1 ;;
        *.md|*.mdx|docs/*) HAS_DOCS=1 ;;
        gradle/build-logic/*|*.gradle.kts|settings.gradle*|gradle.properties|gradle/libs.versions.toml) HAS_GRADLE_CONFIG=1 ;;
    esac
done

CHECKS_RUN=0

# ── Full mode: run everything (opt-in) ─────────────────────────
if [ "${FULL_PRE_PUSH:-0}" = "1" ]; then
    echo "🔧 Full pre-push mode enabled"
    HAS_RUST=1
    HAS_KOTLIN=1
    HAS_WEB=1
    HAS_GRADLE_CONFIG=1
fi

# ── Rust runtime checks ───────────────────────────────────────
if [ "$HAS_RUST" = "1" ] && [ -d "clients/agent-runtime" ]; then
    echo "🦀 Running Rust checks (changed files detected in clients/agent-runtime/)..."
    (
        cd clients/agent-runtime
        cargo fmt --check
        cargo clippy --all-targets -- -D warnings
        cargo test --lib --quiet
    )
    CHECKS_RUN=$((CHECKS_RUN + 1))
fi

# ── Kotlin / KMP checks ──────────────────────────────────────
if [ "$HAS_KOTLIN" = "1" ] || [ "$HAS_GRADLE_CONFIG" = "1" ]; then
    echo "☕ Running Kotlin compile check (changed files detected)..."
    bash ./scripts/gradlew.sh compileKotlinJvm --no-daemon --quiet 2>/dev/null || \
        bash ./scripts/gradlew.sh compileKotlinJvm --no-daemon
    CHECKS_RUN=$((CHECKS_RUN + 1))
fi

# ── Web checks ────────────────────────────────────────────────
if [ "$HAS_WEB" = "1" ]; then
    if command -v pnpm >/dev/null 2>&1; then
        echo "🌐 Running web lint (changed files detected in clients/web/)..."
        (cd clients/web && pnpm check 2>/dev/null || pnpm run check)
        CHECKS_RUN=$((CHECKS_RUN + 1))
    else
        echo "⚠️  pnpm not found — skipping web checks"
    fi
fi

# ── Documentation link check ─────────────────────────────────
if [ "$HAS_DOCS" = "1" ]; then
    if command -v lychee >/dev/null 2>&1; then
        echo "📖 Running doc link check (changed docs detected)..."
        DOC_FILES=$(echo "$CHANGED_FILES" | grep -E '\.(md|mdx)$' || true)
        if [ -n "$DOC_FILES" ]; then
            lychee --config "lychee.toml" --offline --no-progress $DOC_FILES || true
        fi
        CHECKS_RUN=$((CHECKS_RUN + 1))
    fi
fi

# ── Gradle lock check (only if build config changed) ─────────
if [ "$HAS_GRADLE_CONFIG" = "1" ]; then
    echo "🔒 Running dependency lock check (Gradle config changed)..."
    bash ./scripts/gradlew.sh checkLocksAll --no-parallel --no-daemon --quiet 2>/dev/null || \
        bash ./scripts/gradlew.sh checkLocksAll --no-parallel --no-daemon
    CHECKS_RUN=$((CHECKS_RUN + 1))
fi

# ── Summary ───────────────────────────────────────────────────
if [ "$CHECKS_RUN" = "0" ]; then
    echo "No documentation files changed; metadata validation skipped."
fi

echo "✅ Pre-push check passed"
