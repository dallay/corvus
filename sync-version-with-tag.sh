#!/bin/bash
# sync-version-with-tag.sh
# Sync project version files to the latest Git tag (vX.Y.Z)

set -euo pipefail

readonly TAG_REGEX='^v[0-9]+\.[0-9]+\.[0-9]+$'
readonly BASE_TARGETS=(
  "properties:gradle.properties:VERSION"
  "properties:gradle/build-logic/gradle.properties:VERSION"
  "toml:clients/agent-runtime/Cargo.toml:version"
  "json:clients/agent-runtime/npm/corvus-cli/package.json:version"
  "json:clients/agent-runtime/npm/corvus/package.json:version"
  "json:clients/agent-runtime/npm/corvus-darwin-arm64/package.json:version"
  "json:clients/agent-runtime/npm/corvus-darwin-x64/package.json:version"
  "json:clients/agent-runtime/npm/corvus-linux-arm64/package.json:version"
  "json:clients/agent-runtime/npm/corvus-linux-x64/package.json:version"
  "json:clients/agent-runtime/npm/corvus-windows-arm64/package.json:version"
  "json:clients/agent-runtime/npm/corvus-windows-x64/package.json:version"
)

declare -a TARGETS=("${BASE_TARGETS[@]}")
declare -a CHANGED_FILES=()

has_json_version_key() {
  local file="$1"
  awk '
    BEGIN { found = 0 }
    /^[[:space:]]*"version"[[:space:]]*:/ {
      found = 1
      exit
    }
    END {
      if (!found) exit 1
    }
  ' "$file"
  return 0
}

add_json_version_target() {
  local file="$1"
  if [[ -f "$file" ]] && has_json_version_key "$file"; then
    TARGETS+=("json:${file}:version")
  fi
  return 0
}

collect_web_version_targets() {
  local file
  shopt -s nullglob

  # Workspace root package version
  add_json_version_target "clients/web/package.json"

  # Monorepo apps and shared packages versions
  for file in clients/web/apps/*/package.json clients/web/packages/*/package.json; do
    add_json_version_target "$file"
  done

  shopt -u nullglob
  return 0
}

write_if_changed() {
  local file="$1"
  local temp_file="$2"
  if cmp -s "$file" "$temp_file"; then
    rm -f "$temp_file"
    return 0
  fi
  mv "$temp_file" "$file"
  CHANGED_FILES+=("$file")
  return 0
}

update_properties_key() {
  local file="$1"
  local key="$2"
  local value="$3"
  local temp_file

  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file not found" >&2
    exit 1
  fi

  temp_file="$(mktemp "$(dirname "$file")/.sync-version.XXXXXX")"
  awk -v key="$key" -v value="$value" '
    BEGIN {
      prefix = key "="
      updated = 0
    }
    index($0, prefix) == 1 {
      print prefix value
      updated = 1
      next
    }
    { print }
    END {
      if (!updated) print prefix value
    }
  ' "$file" > "$temp_file"
  write_if_changed "$file" "$temp_file"
}

update_toml_string_key() {
  local file="$1"
  local key="$2"
  local value="$3"
  local temp_file

  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file not found" >&2
    exit 1
  fi

  temp_file="$(mktemp "$(dirname "$file")/.sync-version.XXXXXX")"
  awk -v key="$key" -v value="$value" '
    BEGIN { updated = 0 }
    {
      line = $0
      pattern = "^[[:space:]]*" key "[[:space:]]*=[[:space:]]*\"[^\"]*\"[[:space:]]*$"
      if (!updated && line ~ pattern) {
        sub(/"[^"]*"/, "\"" value "\"", line)
        updated = 1
      }
      print line
    }
    END {
      if (!updated) exit 1
    }
  ' "$file" > "$temp_file" || {
    rm -f "$temp_file"
    echo "ERROR: Could not find TOML string key \"$key\" in $file" >&2
    exit 1
  }
  write_if_changed "$file" "$temp_file"
}

update_json_string_key() {
  local file="$1"
  local key="$2"
  local value="$3"
  local temp_file

  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file not found" >&2
    exit 1
  fi

  temp_file="$(mktemp "$(dirname "$file")/.sync-version.XXXXXX")"
  awk -v key="$key" -v value="$value" '
    BEGIN { updated = 0 }
    {
      line = $0
      pattern = "^[[:space:]]*\"" key "\"[[:space:]]*:[[:space:]]*\"[^\"]*\"[[:space:]]*,?[[:space:]]*$"
      if (!updated && line ~ pattern) {
        sub("\"" key "\"[[:space:]]*:[[:space:]]*\"[^\"]*\"", "\"" key "\": \"" value "\"", line)
        updated = 1
      }
      print line
    }
    END {
      if (!updated) exit 1
    }
  ' "$file" > "$temp_file" || {
    rm -f "$temp_file"
    echo "ERROR: Could not find \"$key\" string key in $file" >&2
    exit 1
  }
  write_if_changed "$file" "$temp_file"
}

apply_target_update() {
  local target="$1"
  local target_type
  local file
  local key
  IFS=: read -r target_type file key <<< "$target"

  case "$target_type" in
    properties)
      update_properties_key "$file" "$key" "$version"
      ;;
    json)
      update_json_string_key "$file" "$key" "$version"
      ;;
    toml)
      update_toml_string_key "$file" "$key" "$version"
      ;;
    *)
      echo "ERROR: Unsupported target type '$target_type' in '$target'" >&2
      exit 1
      ;;
  esac
  return 0
}

# Get the globally latest semantic version tag matching vX.Y.Z
tag=$(git tag -l 'v*' --sort=-v:refname | grep -Em1 "$TAG_REGEX" || true)
if [[ -z "$tag" ]]; then
  echo "ERROR: No tag matching vX.Y.Z was found." >&2
  exit 1
fi

version="${tag#v}"
echo "Syncing version files to: $version"

collect_web_version_targets

for target in "${TARGETS[@]}"; do
  apply_target_update "$target"
done

echo "OK: synchronized version to $version"
if [[ ${#CHANGED_FILES[@]} -eq 0 ]]; then
  echo "No files required changes."
else
  echo "Updated files:"
  for file in "${CHANGED_FILES[@]}"; do
    echo "  - $file"
  done
fi

# Helpful next-steps message
diff_files="${CHANGED_FILES[*]-}"
if [[ ${#CHANGED_FILES[@]} -eq 0 ]]; then
  declare -a target_files=()
  for target in "${TARGETS[@]}"; do
    IFS=: read -r _ file _ <<< "$target"
    already_added=false
    for added_file in "${target_files[@]-}"; do
      if [[ "$added_file" == "$file" ]]; then
        already_added=true
        break
      fi
    done
    if [[ "$already_added" == false ]]; then
      target_files+=("$file")
    fi
  done
  diff_files="${target_files[*]}"
fi

cat <<NEXT_STEPS
Next steps (recommended):
  1) Review the changes: git diff "$diff_files"
  2) Commit the change: git add $diff_files && git commit -m "chore: sync version to $version"
  3) Push your branch and tag as appropriate.
      If tag v$version already exists but points at the wrong commit, prefer creating a new patch version.
      Only force-update a tag after confirming no one else depends on it and with explicit confirmation.
      See "Version already exists" troubleshooting guidance in clients/web/apps/docs/src/content/docs/en/guides/release.md.
NEXT_STEPS
