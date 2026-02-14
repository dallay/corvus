@file:Suppress("SpellCheckingInspection")

package com.profiletailors.plugin.repo

import org.gradle.api.provider.ProviderFactory

/**
 * Repository configuration
 *
 * Release repositories should only upload Release packages, Snapshot repositories should only
 * upload Snapshot packages (i.e., packages ending in -SNAPSHOT)
 * - [Reference:
 *   https://central.sonatype.org/publish/publish-portal-snapshots/](https://central.sonatype.org/publish/publish-portal-snapshots/)
 */
object RepositoryConfig {
  /** Private repository configuration. Key must be all uppercase */
  data class Repository(
    /** Repository name e.g., RELEASE */
    val name: String,
    /** Repository URL PRIREPO_URL_RELEASE */
    val url: String,
    /** Repository username PRIREPO_USERNAME_RELEASE */
    val username: String,
    /** Repository password PRIREPO_PASSWORD_RELEASE */
    val password: String,
    /** Repository package list URL PRIREPO_OPEN_RELEASE */
    val open: String,
  )

  const val REPO_PREFIX = "PRIREPO"
  const val URL_PREFIX = "${REPO_PREFIX}_URL_"
  const val USERNAME_PREFIX = "${REPO_PREFIX}_USERNAME_"
  const val PASSWORD_PREFIX = "${REPO_PREFIX}_PASSWORD_"
  const val OPEN_PREFIX = "${REPO_PREFIX}_OPEN_"

  /** Get private repositories */
  fun getPrivateRepositories(providers: ProviderFactory): List<Repository> {
    val infos = providers.gradlePropertiesPrefixedBy(REPO_PREFIX).get()

    val repoNameSet =
      infos
        .map { it.key.substringAfter(URL_PREFIX, "").uppercase() }
        .filter { it.isNotBlank() }
        .toSet()
    val repos =
      repoNameSet.map {
        Repository(
          name = it,
          url = getLocalValue(infos, URL_PREFIX + it),
          username = getLocalValue(infos, USERNAME_PREFIX + it),
          password = getLocalValue(infos, PASSWORD_PREFIX + it),
          open = getLocalValue(infos, OPEN_PREFIX + it),
        )
      }
    return repos
  }

  private fun getLocalValue(map: Map<String, Any?>, key: String): String {
    return map[key]?.toString() ?: System.getenv(key) ?: ""
  }
}
