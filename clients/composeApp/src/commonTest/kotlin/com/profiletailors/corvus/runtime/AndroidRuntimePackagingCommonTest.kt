package com.profiletailors.corvus.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

/**
 * Tests for packaged runtime selection.
 *
 * NOTE: This functionality is now DEPRECATED for client-first Android. Android no longer defaults
 * to local runtime packaging. These tests are kept for backward compatibility and for the optional
 * LOCAL_HOST_ADVANCED mode which may be used in the future.
 */
class AndroidRuntimePackagingCommonTest {
  @Test
  fun `should prefer packaged runtime library when present`() {
    val executable =
      selectPackagedRuntimeExecutableForTest(
        runtimeDirectory = "/data/app/com.profiletailors.corvus/lib/arm64",
        availableEntries = setOf("libcorvus.so", "libother.so"),
      )

    assertEquals("/data/app/com.profiletailors.corvus/lib/arm64/libcorvus.so", executable)
  }

  @Test
  fun `should fall back to plain binary name when packaged library is absent`() {
    val executable =
      selectPackagedRuntimeExecutableForTest(
        runtimeDirectory = "/data/app/com.profiletailors.corvus/lib/arm64",
        availableEntries = setOf("corvus"),
      )

    assertEquals("/data/app/com.profiletailors.corvus/lib/arm64/corvus", executable)
  }

  @Test
  fun `should return null when no packaged runtime executable exists`() {
    val executable =
      selectPackagedRuntimeExecutableForTest(
        runtimeDirectory = "/data/app/com.profiletailors.corvus/lib/arm64",
        availableEntries = setOf("libother.so"),
      )

    assertNull(executable)
  }
}

// Test-only helper - kept for backward compatibility with tests
// Not used in production code (client-first model)
@Suppress("ReturnCount") // Guard clauses are idiomatic for nullable chaining in test helpers
internal fun selectPackagedRuntimeExecutableForTest(
  runtimeDirectory: String?,
  availableEntries: Set<String>,
  preferredFileNames: List<String> = listOf("libcorvus.so", "corvus"),
): String? {
  val directory = runtimeDirectory?.trim()?.trimEnd('/')?.takeIf { it.isNotEmpty() } ?: return null
  val executableName = preferredFileNames.firstOrNull { it in availableEntries } ?: return null
  return "$directory/$executableName"
}
