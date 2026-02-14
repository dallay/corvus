#!/bin/bash
# sync-version-with-tag.sh
# Sync project version files to the latest Git tag (vX.Y.Z)

set -euo pipefail

readonly TAG_REGEX='^v[0-9]+\.[0-9]+\.[0-9]+$'
readonly TARGETS=(
  "properties:gradle.properties:VERSION"
  "properties:gradle/build-logic/gradle.properties:VERSION"
  "json:docs/website/package.json:version"
)

declare -a CHANGED_FILES=()

write_if_changed() {
  local file="$1"
  local temp_file="$2"
  if cmp -s "$file" "$temp_file"; then
    rm -f "$temp_file"
    return
  fi
  mv "$temp_file" "$file"
  CHANGED_FILES+=("$file")
}

update_properties_key() {
  local file="$1"
  local key="$2"
  local value="$3"
  local temp_file

  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file not found"
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

update_json_string_key() {
  local file="$1"
  local key="$2"
  local value="$3"
  local temp_file

  if [[ ! -f "$file" ]]; then
    echo "ERROR: $file not found"
    exit 1
  fi

  temp_file="$(mktemp "$(dirname "$file")/.sync-version.XXXXXX")"
  awk -v key="$key" -v value="$value" '
    BEGIN { updated = 0 }
    {
      line = $0
      pattern = "^[[:space:]]*\"" key "\"[[:space:]]*:[[:space:]]*\"[^\"]*\"[[:space:]]*,?[[:space:]]*$"
      if (!updated && line ~ pattern) {
        sub("^([[:space:]]*\"" key "\"[[:space:]]*:[[:space:]]*)\"[^\"]*\"", "\\1\"" value "\"", line)
        updated = 1
      }
      print line
    }
    END {
      if (!updated) exit 1
    }
  ' "$file" > "$temp_file" || {
    rm -f "$temp_file"
    echo "ERROR: Could not find \"$key\" string key in $file"
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
    *)
      echo "ERROR: Unsupported target type '$target_type' in '$target'"
      exit 1
      ;;
  esac
}

# Get the globally latest semantic version tag matching vX.Y.Z
tag=$(git tag -l 'v*' --sort=-v:refname | grep -Em1 "$TAG_REGEX" || true)
if [[ -z "$tag" ]]; then
  echo "ERROR: No tag matching vX.Y.Z was found."
  exit 1
fi

version="${tag#v}"
echo "Syncing version files to: $version"

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
diff_files="${CHANGED_FILES[*]}"
if [[ ${#CHANGED_FILES[@]} -eq 0 ]]; then
  diff_files="gradle.properties gradle/build-logic/gradle.properties docs/website/package.json"
fi

cat <<EOF
Next steps (recommended):
  1) Review the changes: git diff "$diff_files"
  2) Commit the change: git add gradle.properties gradle/build-logic/gradle.properties docs/website/package.json && git commit -m "chore: sync version to $version"
  3) Push your branch and tag as appropriate.
     If tag v$version already exists but points at the wrong commit, prefer creating a new patch version.
     Only force-update a tag after confirming no one else depends on it and with explicit confirmation.
     See "Version already exists" troubleshooting guidance in docs/website/src/content/docs/guides/release.md.
EOF
