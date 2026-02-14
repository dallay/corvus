import com.autonomousapps.DependencyAnalysisExtension
import com.autonomousapps.DependencyAnalysisSubExtension
import me.champeau.jmh.JMHTask
import net.ltgt.gradle.errorprone.errorprone

plugins {
  id("java")
  id("com.profiletailors.base.jvm-conflict")
  id("com.autonomousapps.dependency-analysis")
  id("com.profiletailors.check.quality-nullaway")
  id("me.champeau.jmh")
}

jmh {
  includeTests = false
  // Filter JMH tests from command line via -PjmhTests=...
  val commandLineIncludes = providers.gradleProperty("jmhTests")
  if (commandLineIncludes.isPresent) {
    includes.add(commandLineIncludes.get())
  }
}

dependencies {
  // Required for the JMH IDEA plugin:
  // https://plugins.jetbrains.com/plugin/7529-jmh-java-microbenchmark-harness
  jmhAnnotationProcessor("org.openjdk.jmh:jmh-generator-annprocess:${jmh.jmhVersion.get()}")
}

tasks.jmh {
  group = "toolbox"
  outputs.cacheIf { false }
  outputs.upToDateWhen { false }
}

tasks.withType<JMHTask>().configureEach {
  jarArchive = tasks.jmhJar.flatMap { it.archiveFile }
  jvm = javaToolchains.launcherFor(java.toolchain).map { it.executablePath }.get().asFile.path
}

tasks.jmhJar { manifest { attributes(mapOf("Multi-Release" to true)) } }

if (project.parent == null) {
  configure<DependencyAnalysisExtension> { issues { all { ignoreSourceSet("jmh") } } }
} else {
  configure<DependencyAnalysisSubExtension> { issues { onAny { ignoreSourceSet("jmh") } } }
}

tasks.jmhCompileGeneratedClasses {
  // Disable ErrorProne https://github.com/melix/jmh-gradle-plugin/issues/248
  options.errorprone { isEnabled = false }
}
