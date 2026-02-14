import com.profiletailors.plugin.DefaultProjects
import com.profiletailors.plugin.dyIncludeProjects
import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

val enableAutoStructure =
  settings.getPropOrDefault(LocalConfig.Props.ENABLE_AUTO_STRUCTURE).toBoolean()

if (enableAutoStructure) {
  applyIncludes(rootDir)
}

dyIncludeProjects(
  mapOf(
    // Platform project
    DefaultProjects.versionsPath to DefaultProjects.versions,
    // Aggregation and analysis project to create reports about the
    // whole software (coverage, SBOM, ...)
    DefaultProjects.aggregationPath to DefaultProjects.aggregation,
  )
)

/**
 * Include all subfolders that contain a 'build.gradle.kts' as subprojects. (e.g. 'app', 'libs',
 * 'libs/lib1', 'libs/lib2', ...)
 *
 * Do not create 'build.gradle.kts' file in an empty directory. This can lead to phantom build
 * directories. Instead, create a 'build.gradle.kt' file in the directory.
 *
 * [multi_project_builds](https://docs.gradle.org/current/userguide/multi_project_builds.html)
 * [settings_defined_composite](https://docs.gradle.org/current/userguide/composite_builds.html#settings_defined_composite)
 */
fun findSubproject(file: File): List<Pair<String, File>>? {
  return file.listFiles()?.mapNotNull { subDir ->
    if (subDir.isDirectory) {
      if (File(subDir, "build.gradle.kts").exists()) {
        Pair("build.gradle.kts", subDir)
      } else if (File(subDir, "build.gradle.kt").exists()) {
        Pair("build.gradle.kt", subDir)
      } else null
    } else null
  }
}

fun applyIncludes(currentDir: File, parentName: String = "") {
  findSubproject(currentDir)?.forEach { subProject ->
    val (buildFileName, projectDirectory) = subProject
    if (buildFileName == "build.gradle.kts") {
      if (parentName == "__empty") {
        val projectName = ":${projectDirectory.name}"
        include(projectName)
        project(projectName).projectDir = projectDirectory
      } else {
        val projectName = "${parentName}:${projectDirectory.name}"
        include(projectName)
      }
    } else if (buildFileName == "build.gradle.kt") {
      applyIncludes(projectDirectory, "__empty")
    }
  }
}
