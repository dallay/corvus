@file:Suppress("UnstableApiUsage", "detekt:TooManyFunctions")

import com.profiletailors.plugin.DefaultProjects
import com.profiletailors.plugin.Injected
import com.profiletailors.plugin.InternalDependencies
import com.profiletailors.plugin.injected
import com.profiletailors.plugin.tasks.DependencyVersionUpgradesCheck
import com.profiletailors.plugin.tasks.JavaVersionConsistencyCheck
import com.profiletailors.plugin.utils.Ansi
import com.profiletailors.plugin.utils.CatalogUtil
import com.profiletailors.plugin.utils.HttpUtils
import com.profiletailors.plugin.utils.JsonParser
import com.profiletailors.plugin.utils.TextHandle
import com.profiletailors.plugin.utils.chunkedVirtual
import java.net.URI
import java.time.Duration
import java.time.LocalDate
import java.util.concurrent.atomic.AtomicReference

plugins {
  `java-platform`
  id("com.profiletailors.base.lifecycle")
}

// only version project
val isVersionProject = project.path == DefaultProjects.versionsPath

val checkVersionConsistency =
  tasks.register<JavaVersionConsistencyCheck>("checkVersionConsistency") {
    definedVersions = provider {
      configurations["api"].dependencyConstraints.associate {
        "${it.group}:${it.name}" to it.version!!
      }
    }
    aggregatedClasspath = provider {
      configurations["mainRuntimeClasspath"].incoming.resolutionResult.allComponents
    }
    reportFile = layout.buildDirectory.file("reports/version-consistency.txt")
  }

tasks.named("qualityCheck") { dependsOn(checkVersionConsistency) }

tasks.named("qualityGate") { dependsOn(checkVersionConsistency) }

val latestReleases: NamedDomainObjectProvider<DependencyScopeConfiguration> =
  configurations.dependencyScope("dependencyVersionUpgrades") {
    withDependencies {
      add(project.dependencies.platform(project(project.path)))
      configurations.named("api").get().dependencies.forEach {
        add(
          project.dependencies.platform("${it.group}:${it.name}:latest.release") {
            isTransitive = false
          }
        )
      }
      configurations.named("api").get().dependencyConstraints.forEach {
        add(
          project.dependencies.create("${it.group}:${it.name}:latest.release").apply {
            isTransitive = false
          }
        )
      }
    }
  }
val latestReleasesPath: NamedDomainObjectProvider<ResolvableConfiguration> =
  configurations.resolvable("latestReleasesPath") {
    attributes.attribute(Usage.USAGE_ATTRIBUTE, objects.named(Usage.JAVA_RUNTIME))
    attributes.attribute(Category.CATEGORY_ATTRIBUTE, objects.named(Category.LIBRARY))
    extendsFrom(latestReleases.get())
  }

tasks.register<DependencyVersionUpgradesCheck>("checkVersionUpgrades") {
  group = "toolbox"
  projectName.set(project.name)
  dependencies.set(
    configurations.named("api").get().dependencies.map { "${it.group}:${it.name}:${it.version}" }
  )
  dependencyConstraints.set(
    configurations.named("api").get().dependencyConstraints.map {
      "${it.group}:${it.name}:${it.version}"
    }
  )
  latestReleasesResolutionResult.set(
    latestReleasesPath.map { it.incoming.resolutionResult.allComponents }
  )
}

val currentGradleVersion: String = gradle.gradleVersion
val projectExtensions: ExtensionContainer = project.extensions

tasks.register("checkVersions") {
  group = "toolbox"
  description = "Check gradle/*.versions.toml for updates"
  CheckVersionPluginConfig.taskConfigureCheckProjectDependencies(this)
  if (isVersionProject) {
    CheckVersionPluginConfig.taskConfigureCheckVersions(this, injected, projectExtensions)
  }
  CheckVersionPluginConfig.taskConfigureCheckGradleVersion(this, currentGradleVersion)
}

object CheckVersionPluginConfig {

  fun taskConfigureCheckProjectDependencies(task: Task) {
    task.configureCheckProjectDependencies()
  }

  fun taskConfigureCheckVersions(
    task: Task,
    injected: Injected,
    projectExtensions: ExtensionContainer,
  ) {
    task.configureCheckVersions(injected, projectExtensions)
  }

  fun taskConfigureCheckGradleVersion(task: Task, currentGradleVersion: String) {
    task.configureCheckGradleVersion(currentGradleVersion)
  }

  private fun Task.configureCheckVersions(
    injected: Injected,
    projectExtensions: ExtensionContainer,
  ) {
    val configFiles =
      injected.providers.provider {
        projectExtensions.findByType<VersionCatalogsExtension>()?.map {
          injected.layout.settingsDirectory.file("gradle/${it.name}.versions.toml").asFile
        } ?: emptyList()
      }
    doLast {
      configFiles
        .get()
        .stream()
        .parallel()
        .map { file ->
          val lastUpdate = LocalDate.now().toString()
          val content = file.readText()
          val newContent = checkTomlDependencies(content)

          if (content != newContent) {
            val newFile = File(file.parentFile, "__${lastUpdate}-${file.name}")
            newFile.writeText(newContent)
            Ansi.color("✏️ Updated file written to: ${newFile.invariantSeparatorsPath}", "32")
          } else {
            Ansi.color("🚩 No changes in ${file.name}", "32")
          }
        }
        .also {
          println(it.toList().joinToString("\n"))
          println(
            "After Updating dependencies, please run './gradlew writeLocks' to update lockfile"
          )
        }
    }
  }

  private fun getLatestVersionFromMetadata(metadata: String): String {
    return metadata.substringAfter("<release>").substringBefore("</release>").ifBlank {
      metadata.substringAfter("<latest>").substringBefore("</latest>")
    }
  }

  private fun processMetadata(
    metadataUrl: String,
    dependency: String,
    currentVersion: String,
    appendMsg: String = "",
    updateCallback: (String) -> Unit,
  ) {
    var jobMsg = metadataUrl
    val metadata =
      try {
        HttpUtils.get(URI(metadataUrl), Duration.ofSeconds(30))
      } catch (_: Exception) {
        jobMsg += "\n  ❌ $dependency -> read metadata timeout${appendMsg}"
        ""
      }

    if (!metadata.isNullOrBlank()) {
      if (metadata.contains("</metadata>")) {
        val candidate = getLatestVersionFromMetadata(metadata)
        if (candidate != currentVersion) {
          jobMsg += "\n  ✅ $dependency:$currentVersion -> $candidate${appendMsg}"
          updateCallback(candidate)
        }
      } else {
        jobMsg += "\n  ❌ $dependency -> can't find metadata${appendMsg}"
      }
    }
    println(jobMsg)
  }

  @Suppress("detekt:NestedBlockDepth", "detekt:CyclomaticComplexMethod", "detekt:LongMethod")
  private fun checkTomlDependencies(content: String): String {
    val newContent = AtomicReference(content)
    val jobs = mutableListOf<() -> Unit>()
    val contentLines = content.lines()
    var currentTopic = ""

    val versionsRegex = Regex("""(\w+)\s*=\s*"([^"]+)"""")
    val libraryRegex =
      Regex("""(\w+)\s*=\s*\{\s*module\s*=\s*"([^"]+)"\s*,\s*version\s*=\s*"([^"]+)"\s*}""")
    val pluginRegex =
      Regex("""([^\s=]+)\s*=\s*\{\s*id\s*=\s*"([^"]+)"\s*,\s*version\s*=\s*"([^"]+)"\s*}""")

    contentLines.forEachIndexed { lineNumber, line ->
      val trimmed = line.trim()
      if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
        currentTopic = line.removeSurrounding("[", "]")
      }
      if (!line.startsWith("#") && line.contains("=")) {
        when (currentTopic) {
          "versions" -> {
            val match = versionsRegex.find(line)
            match?.destructured?.let { (alias, version) ->
              if (alias != "latest.release") {
                if (alias == "kotlin" && version != embeddedKotlinVersion) {
                  jobs.add {
                    newContent.updateAndGet {
                      it.replace(line, """$alias = "$embeddedKotlinVersion"""")
                    }
                  }
                }
              }
            }
          }
          "libraries" -> {
            val match = libraryRegex.find(line)
            match?.destructured?.let { (alias, module, version) ->
              val (groupId, artifactId) = module.split(":")
              val dependency = "$groupId:$artifactId:$version"
              val dependencyAlias = let {
                val artifactIdToAlias = TextHandle.toCamelCase(artifactId)
                if (alias.startsWith(artifactIdToAlias)) alias else artifactIdToAlias
              }

              if (version != "latest.release") {
                val preLine = contentLines[lineNumber - 1].trim()
                val metadataUrl =
                  if (preLine.startsWith("# http") && preLine.endsWith(".xml")) {
                    preLine.substringAfter("#").trim()
                  } else {
                    if ("${groupId}.gradle.plugin" == artifactId) {
                      CatalogUtil.getPluginMetadataUrl(groupId)
                    } else {
                      CatalogUtil.getLibraryMetadataUrl("${groupId}:${artifactId}")
                    }
                  }
                jobs.add {
                  processMetadata(metadataUrl, dependency, version) { candidate ->
                    newContent.updateAndGet {
                      it.replace(
                        line,
                        """$dependencyAlias = { module = "$module", version = "$candidate" }""",
                      )
                    }
                  }
                }
              }
            }
          }
          "plugins" -> {
            val match = pluginRegex.find(line)
            match?.destructured?.let { (_, pluginId, version) ->
              if (version != "latest.release") {
                val preLine = contentLines[lineNumber - 1].trim()
                val metadataUrl =
                  if (preLine.startsWith("# http") && preLine.endsWith(".xml")) {
                    preLine.substringAfter("#").trim()
                  } else {
                    CatalogUtil.getPluginMetadataUrl(pluginId)
                  }
                jobs.add {
                  processMetadata(metadataUrl, pluginId, version) { candidate ->
                    newContent.updateAndGet {
                      it.replace(
                        line,
                        """${pluginId.replace(".", "-")} = { id = "$pluginId", version = "$candidate" }""",
                      )
                    }
                  }
                }
              }
            }
          }
        }
      }
    }

    jobs.chunkedVirtual(size = 300, timeout = Duration.ofMinutes(5)) { it() }
    return newContent.get()
  }

  private fun Task.configureCheckProjectDependencies() {
    doLast {
      val jobs = mutableListOf<() -> Unit>()
      InternalDependencies.libraries.values.forEach {
        val qualifiedName = it::class.qualifiedName
        val moduleId = it.module
        val moduleVersion = it.version
        val moduleUrl = it.url
        val appendMsg = "\n    $qualifiedName"
        if (moduleUrl.endsWith("/maven-metadata.xml")) {
          jobs.add { processMetadata(moduleUrl, moduleId, moduleVersion, appendMsg) {} }
        } else if (moduleUrl.contains("npmjs.org")) {
          jobs.add {
            var jobMsg = moduleUrl
            val metadata =
              try {
                HttpUtils.get(URI(moduleUrl), Duration.ofSeconds(30))
              } catch (_: Exception) {
                jobMsg += "\n  ❌ $moduleId -> read metadata timeout${appendMsg}"
                ""
              }
            if (!metadata.isNullOrBlank()) {
              if (metadata.contains("dist-tags")) {
                val candidate =
                  runCatching {
                      val map = JsonParser.parseMap(metadata)
                      (map["dist-tags"] as Map<*, *>)["latest"] as String
                    }
                    .getOrNull()
                if (candidate != moduleVersion) {
                  jobMsg += "\n  ✅ $moduleId:$moduleVersion -> $candidate${appendMsg}"
                }
              } else {
                jobMsg += "\n  ❌ $moduleId -> can't find metadata${appendMsg}"
              }
            }
            println(jobMsg)
          }
        }
      }
      jobs.chunkedVirtual(size = 300, timeout = Duration.ofMinutes(5)) { it() }
    }
  }

  private fun Task.configureCheckGradleVersion(currentGradleVersion: String) {
    doLast {
      val gradleVersionUrl = "https://services.gradle.org/versions/current"
      var newestGradleVersion: String?
      val gradleVersionContent =
        try {
          HttpUtils.get(URI(gradleVersionUrl), Duration.ofSeconds(30))
        } catch (_: Exception) {
          ""
        }
      if (!gradleVersionContent.isNullOrBlank()) {
        newestGradleVersion =
          runCatching { JsonParser.parseMap(gradleVersionContent)["version"] as String }.getOrNull()
        if (newestGradleVersion != currentGradleVersion) {
          println(Ansi.color("🐘 Current Stable Gradle version => $newestGradleVersion", "31"))
        }
      }
    }
  }
}
