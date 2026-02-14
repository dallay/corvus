package com.profiletailors.plugin.utils

import com.profiletailors.fixtures.consoleLog
import org.junit.jupiter.api.Test

/** CatalogUtil 单元测试类 */
class CatalogUtilTest {

  /** 测试 library & plugin */
  @Test
  fun getUrls() {
    val module = "org.jspecify:jspecify"
    val libraryPageUrl = CatalogUtil.getLibraryPageUrl(module)
    val libraryMetadataUrl = CatalogUtil.getLibraryMetadataUrl(module)

    val pluginId = "org.jetbrains.kotlin.jvm"
    val pluginPageUrl = CatalogUtil.getPluginPageUrl(pluginId)
    val pluginMetadataUrl = CatalogUtil.getPluginMetadataUrl(pluginId)
    consoleLog(
      """
        $module
          $libraryPageUrl
          $libraryMetadataUrl
        $pluginId
          $pluginPageUrl
          $pluginMetadataUrl
      """
        .trimIndent()
    )
  }
}
