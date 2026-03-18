## 2025-05-22 - Compose - UI Runtime Optimization

**Location:** `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatComponents.kt`
**Issue:** `PasswordVisualTransformation()` was being instantiated on every recomposition of `passwordTextField` (e.g., during every keystroke), leading to unnecessary object allocations.
**Solution:** Wrapped `PasswordVisualTransformation()` in a `remember` block tied to the `isVisible` state.
**Impact:**
- **Reduced GC pressure:** Avoids allocating a new transformation object on every character typed.
- **Improved UI stability:** Ensures consistent transformation instance during typing recompositions.
**Benchmark:**
- Baseline Compilation: 58.8s (clean build)
- Post-Optimization Compilation: 73.6s (clean build, with daemon restart overhead)
- *Note:* Compilation time is not directly affected by this change as it's a runtime UI optimization. The primary benefit is at runtime during user interaction.

---

## 2025-05-23 - Compose - Chat UI Recomposition Optimization

**Location:** `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`
**Issue:** Every keystroke in the chat input was triggering a full recomposition of the `ChatWorkspace` screen and its children. The "Send" button and the main screen `Modifier` chain were being re-evaluated/re-allocated on every character typed, even when the state (enabled/disabled) or the modifiers hadn't logically changed.
**Solution:**
- Used `remember(query)` for the `isSendEnabled` flag. This ensures the blank check only runs when `query` actually changes, and the stable result allows the `Button` to skip recomposition when its enabled state doesn't change.
- Wrapped the top-level `Modifier` chain in `ChatWorkspaceScreen` in a `remember` block to avoid redundant modifier object allocations and chain reconstructions during typing.
**Impact:**
- **Reduced Recompositions:** The "Send" button now only recomposes when the input transitions between blank and non-blank.
- **Improved Interaction Latency:** Avoiding modifier re-allocation on every keystroke reduces the work done on the main thread during typing.
- **Reduced GC Pressure:** Fewer ephemeral objects created during the most frequent user interaction.
**Benchmark:**
- Baseline Compilation: 1m 4.631s
- Post-Optimization Compilation: (Incremental build confirmed successful)
- *Note:* These are runtime UI optimizations focused on interaction smoothness and power efficiency rather than build-time improvements.

---
