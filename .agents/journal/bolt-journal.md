# Bolt Performance Journal ⚡

## 2025-02-18 - KMP - UI Optimization

**Location:**
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/ui/chat/ChatWorkspace.kt`

**Issue:**
Unnecessary object allocations during user interaction (typing in the chat or config fields).
1. `RoundedCornerShape` objects were recreated on every recomposition.
2. `AgentGatewayConfig` was instantiated on every recomposition of `ChatWorkspace`, even when its inputs hadn't changed.
3. `BorderStroke` in `ChatPanel` was recreated on every recomposition.

**Solution:**
Applied Jetpack Compose performance best practices:
1. Extracted static `RoundedCornerShape` definitions into private constants.
2. Wrapped `AgentGatewayConfig` instantiation in `remember(baseUrl, pairingCode, bearerToken, webhookSecret)`.
3. Wrapped `BorderStroke` instantiation in `ChatPanel` in `remember`.
4. Stabilized event lambdas in `ChatWorkspace` with `remember`.
5. Fixed a potential logic error by keeping `isSendEnabled` evaluation simple and direct.

**Impact:**
- **Reduced GC pressure**: Fewer short-lived objects (Shapes, Data Classes, BorderStrokes) created during typing.
- **Improved UI stability**: `ChatWorkspaceScreen` and `ChatPanel` can now skip recompositions when their actual inputs haven't changed.

**Benchmark:**
Build time baseline (hot): ~2s. No regression.
