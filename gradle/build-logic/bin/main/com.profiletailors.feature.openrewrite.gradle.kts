@file:Suppress("UnstableApiUsage", "detekt:SpreadOperator")

import com.profiletailors.plugin.resetTaskGroup

plugins { id("org.openrewrite.rewrite") }

dependencies {
  rewrite(platform("org.openrewrite.recipe:rewrite-recipe-bom:latest.release"))
  rewrite("org.openrewrite:rewrite-java")
  rewrite("org.openrewrite:rewrite-kotlin")
  rewrite("org.openrewrite.recipe:rewrite-rewrite")
  rewrite("org.openrewrite.recipe:rewrite-static-analysis")
  rewrite("org.openrewrite.recipe:rewrite-migrate-java")
  rewrite("org.openrewrite.recipe:rewrite-testing-frameworks")
  rewrite("org.openrewrite.recipe:rewrite-spring")
  rewrite("org.openrewrite.recipe:rewrite-jackson")
  rewrite("org.openrewrite.recipe:rewrite-okhttp")
  rewrite("org.openrewrite.recipe:rewrite-openapi")
  rewrite("org.openrewrite.recipe:rewrite-third-party") {
    exclude(group = "commons-logging", module = "commons-logging")
  }
  rewrite("org.openrewrite.recipe:rewrite-java-security:latest.release")
}

val rewriteYml =
  layout.projectDirectory.file("configs/rewrite/rewrite.yml").asFile.takeIf { it.exists() }
    ?: isolated.rootProject.projectDirectory
      .file("gradle/configs/rewrite/rewrite.yml")
      .asFile
      .takeIf { it.exists() }
val rewriteActiveRecipes = listOf("com.profiletailors.openrewrite.SanityCheck")
val rewriteActiveStyles = listOf("com.profiletailors.openrewrite.SpotlessFormat")

// default excludes.
val defaultRewriteExcludes = arrayOf("**/nocheck/**", "**/autogen/**", "**/generated/**")

rewrite {
  // https://docs.openrewrite.org/reference/gradle-plugin-configuration#configuring-the-rewrite-dsl
  // default value is `<project directory>/rewrite.yml`
  if (rewriteYml != null) {
    configFile = rewriteYml
    activeRecipe(*rewriteActiveRecipes.toTypedArray())
    activeStyle(*rewriteActiveStyles.toTypedArray())
  }
  exclusion(*defaultRewriteExcludes)
  isExportDatatables = true
  failOnDryRunResults = true
}

listOf("rewriteDiscover" to "help", "rewriteDryRun" to "toolbox", "rewriteRun" to "toolbox")
  .forEach { resetTaskGroup(it.first, it.second) }
