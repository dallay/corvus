# Delta for runtime-image-pipeline

## MODIFIED Requirements

### Requirement: REQ-8: Configuration Contract

The `[multimodal]` config section MUST continue to support the existing image-ingress settings and
it MUST additionally support an optional staged-image startup reaper threshold.

```toml
[multimodal]
enabled = false                                   # bool, default: false — global kill switch
allowed_channels = []                             # list of strings — channel allowlist
vision_model_hint = ""                            # string — model route selector
max_image_bytes = null                            # optional integer — override MAX_IMAGE_BYTES
staged_image_reaper_threshold_minutes = 30        # optional integer — startup cleanup age threshold
```

Startup validation MUST enforce:

- If `enabled=true`, then `vision_model_hint` MUST be set and non-empty. Violation MUST produce a
  startup error.
- If `enabled=true`, then `allowed_channels` MUST be non-empty. Violation MUST produce a startup
  error.
- If `max_image_bytes` is set, it MUST be > 0 and <= 52428800 (50 MiB). Violation MUST produce a
  startup error.
- If `staged_image_reaper_threshold_minutes` is not set, the effective threshold MUST default to 30
  minutes.
- If `staged_image_reaper_threshold_minutes` is set, it MUST be greater than 0. Violation MUST
  produce a startup error.
- Non-MVP channel names in `allowed_channels` SHOULD produce a startup warning (not an error).
  These channels will be fail-closed at runtime per the channel-ingestion spec (REQ-8 / ADR-4).

The runtime MAY use a large configured threshold to make startup cleanup effectively inert, but it
MUST still preserve the same matching and age-based deletion rules.

(Previously: the config contract defined `enabled`, `allowed_channels`, `vision_model_hint`, and
`max_image_bytes`, but no startup staged-image reaper threshold.)

#### Scenario: Default reaper threshold is applied

- GIVEN a config file omits `multimodal.staged_image_reaper_threshold_minutes`
- WHEN the runtime starts and executes the staged-image startup reaper
- THEN the effective reaper threshold MUST be 30 minutes

#### Scenario: Config override changes the reaper threshold

- GIVEN a config file sets `multimodal.staged_image_reaper_threshold_minutes=90`
- WHEN the runtime starts and executes the staged-image startup reaper
- THEN the effective reaper threshold MUST be 90 minutes
- AND the cleanup decision for matching files MUST use that threshold

#### Scenario: Invalid reaper threshold fails startup validation

- GIVEN a config file sets `multimodal.staged_image_reaper_threshold_minutes=0`
- WHEN the runtime starts
- THEN the runtime MUST produce a startup validation error
- AND the error message MUST indicate that the reaper threshold must be greater than 0
