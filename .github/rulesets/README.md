# GitHub Rulesets Configuration

This directory contains rulesets to protect branches in the repository.

## Files

- `main-protection.json` - Strict protection for the main branch
- `minor-protection.json` - Moderate protection for minor branches

## Importing Rulesets

### Option 1: Import via GitHub Web Interface

1. Go to the repository: https://github.com/dallay/corvus
2. Click **Settings**
3. In the left sidebar, under **Code and automation**, click **Rules** → **Rulesets**
4. Click **New ruleset** → **Import a ruleset**
5. Select the JSON file (e.g., `main-protection.json`)
6. Click **Create**
7. Repeat for the other file

### Option 2: Using GitHub CLI (requires admin token)

```bash
# For main branch
gh api \
  --method POST \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/dALLAY/corvus/rulesets \
  --input .github/rulesets/main-protection.json

# For minor branch
gh api \
  --method POST \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/dALLAY/corvus/rulesets \
  --input .github/rulesets/minor-protection.json
```

## Ruleset Details

### main-protection

**Target**: `main` branch (default)

**Rules**:

- ✅ Requires Pull Request with at least 1 approval
- ✅ Requires CI checks to pass (`core-check`)
- ✅ Requires linear history (no merge commits)
- ✅ Prevents branch deletion
- ✅ Blocks force push
- ✅ Requires signed commits

**Bypass**: Only admins can bypass (via PR)

### minor-protection

**Target**: `minor` branch

**Rules**:

- ✅ Pull Request recommended (0 approvals required)
- ✅ CI checks optional
- ✅ Prevents branch deletion
- ✅ Blocks force push

**Bypass**: Admins can bypass directly

## Security Notes

Rulesets are more flexible than traditional branch protection rules because:

- Support granular bypass by roles
- Support multiple conditions (ref name patterns)
- Allow evaluation without blocking ("evaluate" mode)
- Work with fnmatch patterns for branch names

## References

- [GitHub Rulesets Documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
- [Available Rules for Rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)