# Mobile Smoke Validation Report

Change: `2026-03-29-mobile-runtime-parity-requirements`
Date: 2026-03-30

## Outcome

Task `5.2` is still **blocked**. The normal Android and iOS entrypoints now route into the
runtime-backed facade, but a truthful end-to-end mobile smoke run still cannot be completed because:

1. Android currently packages fake placeholder runtime payloads instead of an executable mobile
   runtime.
2. iOS still has no installed companion/FFI transport client, so the runtime path remains
   intentionally unavailable.

## Current evidence

### Normal app startup no longer uses preview-only snapshots

#### Android

- `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/MainActivity.kt:15-18`
    - normal launch calls `App(platformOverride = platform)`
-

`clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.android.kt:16-22`

- real launch uses `AndroidRuntimeBridge(...)` when `initialBridgeSnapshot` is null

#### iOS

- `clients/composeApp/src/iosMain/kotlin/com/profiletailors/corvus/MainViewController.kt:5-7`
    - normal launch calls `App(platformOverride = platform)`
-

`clients/composeApp/src/iosMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.ios.kt:13-16`

- real launch constructs `IosRuntimeBridge(...)` when not in preview mode

This clears the earlier preview-wiring blocker, but it does **not** make the milestone smoke
checklist executable yet.

### Blocker A — Android payload is a fake placeholder, not a runnable runtime

Commands executed:

-

`unzip -l "clients/androidApp/build/outputs/apk/debug/androidApp-debug.apk" | rg "libcorvus\.so|corvus"`

- `file "clients/androidApp/build/generated/corvusRuntimeJniLibs/main/arm64-v8a/libcorvus.so"`
- `file "clients/androidApp/build/generated/corvusRuntimeJniLibs/main/x86_64/libcorvus.so"`
-

`xxd -g 1 -l 64 "clients/androidApp/build/generated/corvusRuntimeJniLibs/main/arm64-v8a/libcorvus.so"`
-
`xxd -g 1 -l 64 "clients/androidApp/build/generated/corvusRuntimeJniLibs/main/x86_64/libcorvus.so"`

Observed results:

- The APK contains `lib/arm64-v8a/libcorvus.so` and `lib/x86_64/libcorvus.so`.
- Both packaged files are only 35–36 bytes.
- `file` reports them as `ASCII text`.
- Hex dump shows literal placeholder text:
    - `fake android runtime payload arm64`
    - `fake android runtime payload x86_64`

Relevant code:

-

`clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/AndroidRuntimeBridge.kt:7-12`

- bridge expects an executable path and invokes it with `ProcessBuilder`

-

`clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/runtime/PackagedRuntimeExecutable.kt:3-10`

- runtime selection prefers `libcorvus.so` when present

**Result:** Android does not yet provide a real mobile runtime artifact that can satisfy link ->
ready -> create/resume/end session -> real reply -> approve/deny -> relink/reset.

### Blocker A1 — current repo/toolchain cannot truthfully produce the missing Android artifact

Additional investigation executed for this slice:

- `rustup target list --installed`
- `cargo build --manifest-path "clients/agent-runtime/Cargo.toml" --target aarch64-linux-android`
- `cargo build --manifest-path "clients/agent-runtime/Cargo.toml" --target x86_64-linux-android`
- `printenv ANDROID_HOME ANDROID_SDK_ROOT ANDROID_NDK_HOME ANDROID_NDK_ROOT`
- directory inspection of `/Users/acosta/Library/Android/sdk`,
  `/Users/acosta/Library/Android/sdk/ndk`, and `/Users/acosta/Library/Android/sdk/ndk-bundle`
- repository search for Android JNI/FFI exports under `clients/agent-runtime/src/`

Observed results:

- Installed Rust targets are only:
    - `aarch64-apple-darwin`
    - `thumbv7em-none-eabihf`
    - `wasm32-wasip1`
- Both Android cargo builds fail immediately with:
    - `can't find crate for core`
    - `the aarch64-linux-android target may not be installed`
    - `the x86_64-linux-android target may not be installed`
- Android SDK is present at `/Users/acosta/Library/Android/sdk`, but there is no installed NDK at
  either:
    - `/Users/acosta/Library/Android/sdk/ndk`
    - `/Users/acosta/Library/Android/sdk/ndk-bundle`
- `clients/agent-runtime/Cargo.toml` defines the runtime as a CLI package (
  `[package] name = "corvus"`) and contains no `[lib]` section, no `crate-type = ["cdylib"]`, and no
  Android-native library target.
- Repository search under `clients/agent-runtime/src/` finds no `JNI_OnLoad`, `extern "C"`,
  `#[no_mangle]`, or `jni` bridge implementation.
- Android app packaging still only copies `**/libcorvus.so` into `jniLibs` (
  `clients/androidApp/build.gradle.kts:8-19,43,55`), while runtime launch still uses
  `ProcessBuilder` against the selected packaged path (
  `clients/composeApp/src/androidMain/kotlin/com/profiletailors/corvus/runtime/AndroidRuntimeBridge.kt:97-129`).

Concrete boundary:

- A truthful Android mobile artifact build path does **not** exist in the current repo.
- Missing pieces are not just a Gradle wiring tweak; they include Android Rust target installation,
  Android NDK/linker infrastructure, and an actual Android-compatible runtime artifact model (either
  an executable packaging flow or a real JNI/FFI shared-library surface) that matches how the
  Android app launches the runtime.
- Because those prerequisites are absent, there is no honest code-only repo patch in this slice that
  can turn the current placeholder `libcorvus.so` packaging hook into a real runnable Android
  runtime path without first adding unsupported cross-compilation/native-runtime infrastructure.

### Blocker B — iOS transport client is still missing

Evidence:

-

`clients/composeApp/src/iosMain/kotlin/com/profiletailors/corvus/runtime/IosRuntimeBridge.kt:23-29`

- installer API exists: `installIosRuntimeCompanionClient(...)`

- repository search finds no call sites for `installIosRuntimeCompanionClient(...)`
-

`clients/composeApp/src/iosMain/kotlin/com/profiletailors/corvus/runtime/PlatformRuntimeDependencies.ios.kt:7,14-16`

- runtime creation falls back to `MissingInfrastructureIosRuntimeCompanionClient()` when no
  client is installed

-

`clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/runtime/IosRuntimeCompanionDiagnostics.kt:3-13`

- diagnostics explicitly state:
- `no companion IPC transport client exists in this repository`
- `no embedded Rust FFI bridge exists in this repository`

**Result:** there is still no concrete iOS runtime transport to exercise for the milestone
checklist.

## Task status conclusion

- `5.2` remains **open**.
- `openspec/changes/2026-03-29-mobile-runtime-parity-requirements/tasks.md` should remain unchanged
  for this task.
- The current truthful state is: preview-launch blocker resolved, but real Android/iOS milestone
  smoke validation is still impossible because required runtime transport infrastructure is not
  actually present.

## Unambiguous prerequisites for a future rerun

1. Android: add the missing Android runtime infrastructure before retrying smoke validation:
    - install/support Rust Android targets for the required ABIs,
    - provide Android NDK/linker infrastructure,
    - choose and implement one truthful runtime artifact shape that matches app launch semantics:
        - a packaged executable path the app can launch, or
        - a real JNI/FFI shared library plus matching Android-side bridge code.
          Then replace placeholder `libcorvus.so` payloads and prove the shipped app can
          execute/bind to the real artifact.
2. iOS: implement and install a concrete `IosRuntimeCompanionClient` backed by real IPC or embedded
   Rust FFI.
3. Re-run the real checklist on device/simulator after those two prerequisites are met:
    - link
    - ready state
    - create UUID session
    - resume UUID session
    - end session
    - receive real runtime reply
    - approve and deny
    - relink/reset recovery without switching surfaces
