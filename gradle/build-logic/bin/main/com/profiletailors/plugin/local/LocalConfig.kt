@file:Suppress("DuplicatedCode")

package com.profiletailors.plugin.local

import com.profiletailors.plugin.local.LocalConfig.DEFAULT_LOCAL_PROPERTY_FILE
import com.profiletailors.plugin.propOrDefault
import java.nio.file.Path
import java.util.*
import org.gradle.api.Project
import org.gradle.api.initialization.Settings
import org.gradle.api.plugins.ExtensionAware
import org.gradle.api.plugins.ExtraPropertiesExtension
import org.gradle.api.plugins.PluginAware

fun PluginAware.getPropOrDefault(prop: LocalConfig.Props, fromProvider: Boolean = true) =
  propOrDefault(prop.key, prop.defaultValue, fromProvider)

/** Loads the local properties from the given file. */
object LocalConfig {

  enum class Props(val key: String, val defaultValue: String) {
    IS_DEBUG("IS_DEBUG", "false"),
    IS_JMH("IS_JMH", "false"),
    // CI环境
    CI("CI", "false"),
    // 项目的group
    GROUP("GROUP", "starter.gradle"),
    // 项目的version
    VERSION("VERSION", "1.0.0-SNAPSHOT"),
    LICENSE("LICENSE", ""),
    // gradle远程缓存账户名
    BUILD_CACHE_USER("BUILD_CACHE_USER", ""),
    // gradle远程缓存密码
    BUILD_CACHE_PWD("BUILD_CACHE_PWD", ""),
    ENABLE_AUTO_STRUCTURE("ENABLE_AUTO_STRUCTURE", "false"),
    // 是否国内仓库代理
    ENABLE_PROXY_REPO("ENABLE_PROXY_REPO", "false"),
    PRIREPO_URL_RELEASE("PRIREPO_URL_RELEASE", ""),
    PRIREPO_USERNAME_RELEASE("PRIREPO_USERNAME_RELEASE", ""),
    PRIREPO_PASSWORD_RELEASE("PRIREPO_PASSWORD_RELEASE", ""),
    PRIREPO_URL_SNAPSHOT("PRIREPO_URL_SNAPSHOT", ""),
    PRIREPO_USERNAME_SNAPSHOT("PRIREPO_USERNAME_SNAPSHOT", ""),
    PRIREPO_PASSWORD_SNAPSHOT("PRIREPO_PASSWORD_SNAPSHOT", ""),
    // 是否开启JPMS
    JPMS_ENABLED("JPMS_ENABLED", "false"),
    // 是否开启JEP新特性
    JEP_ENABLE_PREVIEW("JEP_ENABLE_PREVIEW", "false"),
    DOC_FAIL_ON_ERROR("DOC_FAIL_ON_ERROR", "false"),
    DOC_JAR_ENABLED("DOC_JAR_ENABLED", "true"),
    DOCKER_REGISTRY_URL("DOCKER_REGISTRY_URL", ""),
    DOCKER_REGISTRY_USERNAME("DOCKER_REGISTRY_USERNAME", ""),
    DOCKER_REGISTRY_PASSWORD("DOCKER_REGISTRY_PASSWORD", ""),
    // 产物信息: 开发者名称
    POM_DEVELOPER_NAME("POM_DEVELOPER_NAME", "dallay"),
    // 产物信息: 项目地址
    POM_URL("POM_URL", "https://github.com/dallay"),
    // 产物信息: 源码仓库地址
    POM_SCM_CONNECTION("POM_SCM_CONNECTION", "scm:git:https://github.com/dallay/404-page.git"),
    // 产物信息: 许可证地址
    POM_LICENSE_URL("POM_LICENSE_URL", "https://mit-license.org"),
    // central.sonatype.com 账号
    SONATYPE_USERNAME("mavenCentralUsername", ""),
    // central.sonatype.com 密码
    SONATYPE_PASSWORD("mavenCentralPassword", ""),
    // OpenPGP 密钥标识符（可选，如未填写，则密钥需为主密钥）
    GPG_SIGNING_KEY_ID("signingInMemoryKeyId", ""),
    // OpenPGP 密钥（如未填写GPG_SIGNING_KEY_ID，则为主密钥，否则为子密钥）
    GPG_SIGNING_KEY("signingInMemoryKey", ""),
    // OpenPGP 密码
    GPG_SIGNING_PASSWORD("signingInMemoryKeyPassword", ""),
    // 内置测试套件
    JUNIT_JUPITER_M2_ENABLED("JUNIT_JUPITER_M2_ENABLED", "false"),
  }

  /** The default local properties file. */
  private const val DEFAULT_LOCAL_PROPERTY_FILE = "__local.properties"

  /**
   * Loads the `local.properties`: [DEFAULT_LOCAL_PROPERTY_FILE]
   *
   * @param script `project` or `settings`.
   * @return The full extraProperties.
   */
  @Suppress("UnstableApiUsage", "detekt:NestedBlockDepth", "unused")
  private fun loadLocalProperties(script: ExtensionAware): ExtraPropertiesExtension {
    val extraProperties = script.extensions.extraProperties
    if (script is Project) {
      val loaded =
        runCatching { extraProperties.get("loadLocalPropertiesToProject") }.getOrNull() != null

      if (!loaded) {
        val enableLocalConfig =
          runCatching { extraProperties.get("ENABLE_LOCAL_CONFIG")?.toString()?.toBoolean() }
            .getOrNull() == true
        if (enableLocalConfig) {
          script.providers
            .fileContents(script.isolated.projectDirectory.file(DEFAULT_LOCAL_PROPERTY_FILE))
            .asText
            .orNull
            ?.also {
              val properties = Properties()
              properties.load(it.reader())
              properties.forEach { (key, value) ->
                extraProperties.set(key.toString(), value.toString())
              }
            }
        }
        extraProperties.set("loadLocalPropertiesToProject", true)
      }
      return extraProperties
    } else if (script is Settings) {
      val loaded =
        runCatching { extraProperties.get("loadLocalPropertiesToSettings") }.getOrNull() != null

      if (!loaded) {
        val enableLocalConfig =
          runCatching { extraProperties.get("ENABLE_LOCAL_CONFIG")?.toString()?.toBoolean() }
            .getOrNull() == true
        if (enableLocalConfig) {
          val file = Path.of(script.settingsDir.path, DEFAULT_LOCAL_PROPERTY_FILE).toFile()
          if (file.exists()) {
            val properties = Properties()
            properties.load(file.reader())
            properties.forEach { (key, value) ->
              extraProperties.set(key.toString(), value.toString())
            }
          }
        }
        extraProperties.set("loadLocalPropertiesToSettings", true)
      }
      return extraProperties
    }
    error("Unsupported script type: $script")
  }
}
