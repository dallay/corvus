package com.profiletailors.plugin.utils

import org.gradle.api.file.ProjectLayout
import org.gradle.api.provider.ProviderFactory

/**
 * Utility class: Encapsulate Gradle reject version range rules
 *
 * gt: greater than > x ge: greater or equal >= x lt: less than < x le: less or equal <= x
 */
object VersionRange {

  /** Reject all < version */
  fun lt(version: String): String = "(,$version)"

  /** Reject all <= version */
  fun le(version: String): String = "(,$version]"

  /** Reject all > version */
  fun gt(version: String): String = "($version,)"

  /** Reject all >= version */
  fun ge(version: String): String = "[$version,)"

  /** Custom range */
  fun range(
    from: String,
    to: String,
    inclusiveFrom: Boolean = true,
    inclusiveTo: Boolean = false,
  ): String {
    val left = if (inclusiveFrom) "[" else "("
    val right = if (inclusiveTo) "]" else ")"
    return "$left$from,$to$right"
  }
}

object SemVerUtils {

  /** return the short commit hash of the current Git commit. If not available, returns null. */
  fun gitBuildMetadata(providers: ProviderFactory, layout: ProjectLayout): String {
    return runCatching {
        val fullCommit =
          providers
            .exec {
              isIgnoreExitValue = true
              commandLine("git", "rev-parse", "HEAD")
              workingDir = layout.projectDirectory.asFile
            }
            .standardOutput
            .asText
            .get()
            .trim()
        fullCommit.ifBlank { "unknown" }
      }
      .getOrDefault("unknown")
  }
}
