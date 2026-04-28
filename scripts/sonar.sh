#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SONAR_ORGANIZATION="${SONAR_ORGANIZATION:-dallay}"
SONAR_PROJECT_KEY="${SONAR_PROJECT_KEY:-dallay_corvus}"
SONAR_PROJECT_KEY="${SONAR_PROJECT_KEY//-/_}"
SONAR_PROJECT_NAME="${SONAR_PROJECT_NAME:-$(basename "$ROOT_DIR")}"
VALIDATE_ONLY="${1:-}"

if [[ -z "${SONAR_TOKEN:-}" ]]; then
  echo "SONAR_TOKEN is not configured. Local SonarQube analysis requires this environment variable." >&2
  echo "Export SONAR_TOKEN and retry 'make sonar'." >&2
  exit 1
fi

if ! command -v sonar-scanner >/dev/null 2>&1; then
  echo "sonar-scanner is required for local SonarQube analysis." >&2
  echo "Install sonar-scanner and ensure it is available on PATH, then retry 'make sonar'." >&2
  exit 1
fi

if [[ "$VALIDATE_ONLY" == "--validate-only" ]]; then
  exit 0
fi

KOVER_CORE_REPORT="$ROOT_DIR/modules/agent-core-kmp/build/reports/kover/report.xml"
KOVER_APP_REPORT="$ROOT_DIR/clients/composeApp/build/reports/kover/report.xml"
RUST_LCOV_REPORT="$ROOT_DIR/coverage/agent-runtime-coverage.lcov"
WEB_LCOV_REPORT="$ROOT_DIR/clients/web/apps/dashboard/coverage/lcov.info"

for report in "$KOVER_CORE_REPORT" "$KOVER_APP_REPORT" "$RUST_LCOV_REPORT" "$WEB_LCOV_REPORT"; do
  if [[ ! -f "$report" ]]; then
    echo "Expected coverage artifact is missing: $report" >&2
    echo "Run 'make sonar' from a clean state so coverage generation completes before scanning." >&2
    exit 1
  fi
done

cd "$ROOT_DIR"

sonar-scanner \
  -Dsonar.organization="$SONAR_ORGANIZATION" \
  -Dsonar.projectKey="$SONAR_PROJECT_KEY" \
  -Dsonar.projectName="$SONAR_PROJECT_NAME" \
  -Dsonar.sources=. \
  -Dsonar.tests=. \
  -Dsonar.test.inclusions='**/*.spec.ts,**/*.test.ts,**/*.test.tsx,**/*_test.rs,**/tests/**,**/src/test/**,**/src/commonTest/**,**/src/jvmTest/**,**/src/androidUnitTest/**,**/src/iosTest/**' \
  -Dsonar.exclusions='**/.git/**,**/.gradle/**,**/build/**,**/dist/**,**/coverage/**,**/node_modules/**,**/.next/**,**/.turbo/**,**/target/**,**/vendor/**,**/generated/**,**/clients/agent-runtime/target/**' \
  -Dsonar.coverage.exclusions='**/*.spec.ts,**/*.test.ts,**/*.test.tsx,scripts/**,clients/web/apps/dashboard/src/App.vue,clients/web/packages/shared/index.ts,clients/agent-runtime/npm/corvus-cli/scripts/postinstall.mjs,clients/agent-runtime/npm/corvus-cli/lib/install.js,gradle/build-logic/src/main/kotlin/**,clients/composeApp/src/**,clients/iosApp/**,clients/web/apps/*/src/main.ts,clients/web/apps/*/src/i18n.ts,clients/web/apps/docs/src/content.config.ts,clients/web/apps/docs/astro.config.mjs,clients/web/apps/marketing/astro.config.mjs,clients/web/apps/dashboard/tsconfig.node.json,clients/web/packages/locales/src/index.ts,clients/agent-runtime/firmware/**,clients/agent-runtime/examples/**' \
  -Dsonar.cpd.exclusions='**/content/docs/**/*.md,**/content/docs/**/*.mdx' \
  -Dsonar.issue.ignore.multicriteria=e1 \
  -Dsonar.issue.ignore.multicriteria.e1.ruleKey=kotlin:S100 \
  -Dsonar.issue.ignore.multicriteria.e1.resourceKey='**/*Test.kt' \
  -Dsonar.coverage.jacoco.xmlReportPaths="$KOVER_CORE_REPORT,$KOVER_APP_REPORT" \
  -Dsonar.rust.lcov.reportPaths="$RUST_LCOV_REPORT" \
  -Dsonar.javascript.lcov.reportPaths="$WEB_LCOV_REPORT" \
  -Dsonar.python.version=3.12 \
  -Dsonar.nodejs.executable="$(command -v node)"
