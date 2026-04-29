package com.profiletailors.corvus.runtime

internal val IOS_COMPANION_MISSING_INFRASTRUCTURE: List<String> =
  listOf(
    "no embedded Rust FFI bridge exists in this repository",
  )

internal fun iosCompanionUnavailableMessage(): String = buildString {
  append("iOS companion transport is not configured for this build.")
  append(" Missing infrastructure: ")
  append(IOS_COMPANION_MISSING_INFRASTRUCTURE.joinToString(separator = "; "))
  append('.')
}
