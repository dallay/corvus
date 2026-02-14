@file:Suppress("UnstableApiUsage", "PropertyName")

import com.profiletailors.plugin.gradle.eagerDiskCache
import com.profiletailors.plugin.injected

// only root project
if (path == ":") {
  val eagerDiskCacheKey = "com.profiletailors.feature.git-hook"

  // issue: https://github.com/gradle/gradle/issues/23895
  // Calculating task graph as configuration cache cannot be reused because output of the external
  // process 'git' has changed.
  //  tasks.register("installGitHooks") {
  //    group = "toolbox"
  //    description = "Initialize Git hooks"
  //    installGitHooks()
  //  }

  project.eagerDiskCache("$eagerDiskCacheKey.git.hooks") {
    val hookSource = rootProject.file("gradle/configs/git/hooks")
    val hookTarget = rootProject.file(".git/hooks")
    GitHooks.install(hookSource, hookTarget)
    "true"
  }
}

/** Install Git hooks */
fun Task.installGitHooks() {
  val hooksSourceProvider =
    injected.providers.provider { rootProject.file("gradle/configs/git/hooks") }
  val hooksTargetProvider = injected.providers.provider { rootProject.file(".git/hooks") }

  doLast {
    val hooksSource = hooksSourceProvider.get()
    val hooksTarget = hooksTargetProvider.get()
    GitHooks.install(hooksSource, hooksTarget)
  }
}

/**
 * Git hooks
 *
 * [git-hooks](https://git-scm.com/book/zh/v2/%e8%87%aa%e5%ae%9a%e4%b9%89-Git-Git-%e9%92%a9%e5%ad%90)
 */
object GitHooks {

  fun install(hooksSource: File, hooksTarget: File) {
    if (hooksSource.exists() && hooksTarget.exists()) {
      hooksTarget.mkdirs()
      hooksSource.listFiles()?.forEach { file ->
        if (file.isFile && !file.name.endsWith(".sample")) {
          val targetFile = hooksTarget.resolve(file.name.removeSuffix(".sh"))
          file.copyTo(targetFile, overwrite = true)
          targetFile.setExecutable(true)
        }
      }
      println("✓ Git hooks installed successfully")
    }
  }
}
