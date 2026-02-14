package com.profiletailors.plugin.utils

object CatalogUtil {

  /**
   * Get the library page URL for the given module.
   *
   * @param module The module name in the format `groupId:artifactId` or `groupId/artifactId`.
   * @param delimiter The delimiter character used to split the module name. Defaults to ':'.
   * @return The library page URL.
   */
  fun getLibraryPageUrl(module: String, delimiter: Char = ':'): String {
    if (!module.contains(delimiter) || module.count { it == delimiter } != 1)
      error("Invalid module name: $module")
    val (groupId, artifactId) = module.split(delimiter)
    return "https://search.maven.org/artifact/${groupId}/${artifactId}"
  }

  /**
   * Returns the library URL for the given module.
   *
   * @param module The module name in the format `groupId:artifactId` or `groupId/artifactId`.
   * @param delimiter The delimiter used to split the module name.
   * @param url The URL of the Maven repository.
   * @return The library URL.
   */
  fun getLibraryMetadataUrl(
    module: String,
    delimiter: Char = ':',
    url: String = "https://mirrors.cloud.tencent.com/nexus/repository/maven-public/",
  ): String {
    return "${url}${transformModuleToLibraryMetadata(module, delimiter)}"
  }

  /**
   * Transforms the module name to the library metadata URL.
   *
   * @param module The module name in the format `groupId:artifactId` or `groupId/artifactId`.
   * @param delimiter The delimiter used to split the module name.
   * @return The library metadata URL.
   */
  fun transformModuleToLibraryMetadata(module: String, delimiter: Char = ':'): String {
    if (!module.contains(delimiter) || module.count { it == delimiter } != 1)
      error("Invalid module name: $module")
    val (groupId, artifactId) = module.split(delimiter)
    return "${groupId.replace(".", "/")}/$artifactId/maven-metadata.xml"
  }

  /**
   * Returns the package page URL for the given NPM package name.
   *
   * @param packageName The name of the NPM package.
   */
  fun getNpmPackagePageUrl(packageName: String): String {
    return "https://www.npmjs.com/package/${packageName}"
  }

  /**
   * Returns the package metadata URL for the given NPM package name.
   *
   * @param packageName The name of the NPM package.
   */
  fun getNpmPackageMetadataUrl(packageName: String): String {
    return "https://registry.npmjs.org/${packageName}"
  }

  /**
   * Get the plugin page URL.
   *
   * @param pluginId The plugin ID.
   * @return The plugin page URL.
   */
  fun getPluginPageUrl(pluginId: String): String {
    return "https://plugins.gradle.org/plugin/$pluginId"
  }

  /**
   * Get the plugin metadata URL.
   *
   * @param pluginId The plugin ID.
   * @param url The plugin metadata URL.
   * @return The plugin metadata URL.
   */
  fun getPluginMetadataUrl(
    pluginId: String,
    url: String = "https://plugins.gradle.org/m2/",
  ): String {
    return "$url${transformPluginIdToLibraryMetadata(pluginId)}"
  }

  /**
   * Transform the plugin ID to library metadata.
   *
   * @param pluginId The plugin ID.
   * @return The library metadata.
   */
  fun transformPluginIdToLibraryMetadata(pluginId: String): String {
    return "${pluginId.replace(".", "/")}/${pluginId}.gradle.plugin/maven-metadata.xml"
  }
}
