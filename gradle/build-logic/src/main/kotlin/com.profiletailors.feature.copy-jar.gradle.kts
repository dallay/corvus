@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.tasks.GenerateStartScript

plugins { java }

val projectRoot = isolated.rootProject.projectDirectory

val generateStartScript =
  tasks.register<GenerateStartScript>("generateStartScript") {
    appJar.set(tasks.jar.flatMap { it.archiveFileName })
  }

val copyJarToRoot =
  tasks.register<Copy>("copyJarToRoot") {
    from(tasks.jar)
    from(generateStartScript)
    exclude("**/*-plain.jar")
    into(projectRoot.dir("build/archives"))
  }

tasks.assemble { dependsOn(copyJarToRoot) }
