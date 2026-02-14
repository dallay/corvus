@file:Suppress("SpellCheckingInspection")

package com.profiletailors.plugin.repo

import org.gradle.api.provider.ProviderFactory

/**
 * 仓库配置
 *
 * Release 库建议只能上传 Release 包、Snapshot 库建议只能上传 Snapshot 包（即版本以 -SNAPSHOT 结尾的包）
 * - [可参考：https://central.sonatype.org/publish/publish-portal-snapshots/](https://central.sonatype.org/publish/publish-portal-snapshots/)
 * - [可参考：https://help.aliyun.com/zh/yunxiao/user-guide/faq-13](https://help.aliyun.com/zh/yunxiao/user-guide/faq-13)
 */
object RepositoryConfig {
  /** 私有仓库配置。 key需全部大写 */
  data class Repository(
    /** 仓库名称 如：RELEASE */
    val name: String,
    /** 仓库地址 PRIREPO_URL_RELEASE */
    val url: String,
    /** 仓库用户名 PRIREPO_USERNAME_RELEASE */
    val username: String,
    /** 仓库密码 PRIREPO_PASSWORD_RELEASE */
    val password: String,
    /** 仓库包列表地址 PRIREPO_OPEN_RELEASE */
    val open: String,
  )

  const val REPO_PREFIX = "PRIREPO"
  const val URL_PREFIX = "${REPO_PREFIX}_URL_"
  const val USERNAME_PREFIX = "${REPO_PREFIX}_USERNAME_"
  const val PASSWORD_PREFIX = "${REPO_PREFIX}_PASSWORD_"
  const val OPEN_PREFIX = "${REPO_PREFIX}_OPEN_"

  /** 获取私有仓库 */
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
