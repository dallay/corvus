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
