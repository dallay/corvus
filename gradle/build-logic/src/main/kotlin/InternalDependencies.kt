@file:Suppress("PackageDirectoryMismatch")

package com.profiletailors.plugin

object InternalDependencies {

  data class Library(
    val key: String,
    val module: String,
    val version: String,
    val type: String,
    val url: String,
  )

  // spotless:off
  val data =
    """
    [libraries]
    # Maven dependencies
junitBom = { module = "org.junit:junit-bom", version = "6.0.2", type = "maven", url = "https://repo.maven.apache.org/maven2/org/junit/junit-bom/maven-metadata.xml" }
assertjBom = { module = "org.assertj:assertj-bom", version = "3.27.7", type = "maven", url = "https://repo.maven.apache.org/maven2/org/assertj/assertj-bom/maven-metadata.xml" }
    jspecify = { module = "org.jspecify:jspecify", version = "1.0.0", type = "maven", url = "https://repo.maven.apache.org/maven2/org/jspecify/jspecify/maven-metadata.xml" }
    nullaway = { module = "com.uber.nullaway:nullaway", version = "0.13.1", type = "maven", url = "https://repo.maven.apache.org/maven2/com/uber/nullaway/nullaway/maven-metadata.xml" }
    errorProneCore = { module = "com.google.errorprone:error_prone_core", version = "2.47.0", type = "maven", url = "https://repo.maven.apache.org/maven2/com/google/errorprone/error_prone_core/maven-metadata.xml" }
    errorProneContrib = { module = "tech.picnic.error-prone-support:error-prone-contrib", version = "0.28.0", type = "maven", url = "https://repo.maven.apache.org/maven2/tech/picnic/error-prone-support/error-prone-contrib/maven-metadata.xml" }
    refasterRunner = { module = "tech.picnic.error-prone-support:refaster-runner", version = "0.28.0", type = "maven", url = "https://repo.maven.apache.org/maven2/tech/picnic/error-prone-support/refaster-runner/maven-metadata.xml" }
    comFacebookKtfmt = { module = "com.facebook:ktfmt", version = "0.61", type = "maven", url = "https://repo.maven.apache.org/maven2/com/facebook/ktfmt/maven-metadata.xml" }
    spotbugsAnnotations = { module = "com.github.spotbugs:spotbugs-annotations", version = "4.9.8", type = "maven", url = "https://repo.maven.apache.org/maven2/com/github/spotbugs/spotbugs-annotations/maven-metadata.xml" }

    # NPM dependencies
    prettier = { module = "prettier", version = "3.8.1", type = "npm", url = "https://registry.npmjs.org/prettier" }
    prettierPluginXml = { module = "@prettier/plugin-xml", version = "3.4.2", type = "npm", url = "https://registry.npmjs.org/@prettier/plugin-xml" }
    """
      .trimIndent()

  // spotless:on

  val libraries: Map<String, Library> by lazy {
    data
      .lineSequence()
      .map { it.trim() }
      .filter { it.isNotEmpty() && !it.startsWith("#") && it.contains("=") }
      .associate { line ->
        val (key, value) = line.split("=", limit = 2).map { it.trim() }
        val props =
          value
            .removePrefix("{")
            .removeSuffix("}")
            .split(",")
            .map { it.trim() }
            .associate {
              val (k, v) = it.split("=", limit = 2).map { s -> s.trim().removeSurrounding("\"") }
              k to v
            }
        key to
          Library(
            key = key,
            module = props.getValue("module"),
            version = props.getValue("version"),
            type = props.getValue("type"),
            url = props.getValue("url"),
          )
      }
  }

  fun get(key: String): Library = libraries[key] ?: error("Unknown dependency: $key")

  fun useLibrary(key: String): String = get(key).let { "${it.module}:${it.version}" }

  fun maven(): List<Library> = libraries.values.filter { it.type == "maven" }

  fun npm(): List<Library> = libraries.values.filter { it.type == "npm" }
}
