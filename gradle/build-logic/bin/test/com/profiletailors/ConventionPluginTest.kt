package com.profiletailors

import com.profiletailors.fixtures.GradleProject
import java.io.File
import org.gradle.testkit.runner.BuildResult
import org.gradle.testkit.runner.TaskOutcome.SUCCESS
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.parallel.Execution
import org.junit.jupiter.api.parallel.ExecutionMode
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.MethodSource

class ConventionPluginTest {

  @Execution(ExecutionMode.CONCURRENT)
  @ParameterizedTest
  @MethodSource("pluginIds")
  fun `each plugin can be applied individually without error`(pluginId: String) {
    val p = GradleProject()
    when {
      pluginId.endsWith(".settings") ->
        p.defaultGradleProperties()
          .settingsFile("""plugins { id("${pluginId.substringBeforeLast(".settings")}") }""")
      pluginId.endsWith(".root") ->
        p.defaultGradleProperties()
          .rootBuildFile("""plugins { id("${pluginId.substringBeforeLast(".settings")}") }""")
      else -> p.withMinimalStructure().moduleBuildFile("""plugins { id("$pluginId") }""")
    }

    val result = p.help()

    assertEquals(result.task(":help")!!.outcome, SUCCESS)
  }

  @Suppress("detekt:LongMethod")
  @Test
  fun `qualityGate sorts dependencies of a library`() {
    val p = GradleProject().withMinimalStructure()
    p.catalog(
      """
      [libraries]
      resteasy-core = { module = "org.jboss.resteasy:resteasy-core", version = "4.7.6.Final" }
      resteasy-jackson2-provider = { module = "org.jboss.resteasy:resteasy-jackson2-provider" }
      guice = { module = "com.google.inject:guice", version = "5.1.0" }
      """
        .trimIndent()
    )
    p.moduleBuildPropertiesFile(
      """
      jdk=25
      """
        .trimIndent()
    )
    val buildFile =
      p.moduleBuildFile(
        """
        plugins {
          id("com.profiletailors.module.java")
        }

        dependencyLocking { lockMode = LockMode.LENIENT }

        jvmDependencyConflicts.patch {
          align(
            "org.jboss.resteasy:resteasy-core",
            "org.jboss.resteasy:resteasy-guice",
            "org.jboss.resteasy:resteasy-jackson2-provider",
          )
        }

        dependencies {
          implementation("com.google.errorprone:error_prone_annotations:2.42.0")
          implementation("com.google.guava:guava:33.5.0-jre")
          implementation(libs.resteasy.jackson2.provider)
          implementation(libs.resteasy.core)
          implementation(libs.guice)
        }
        """
          .trimIndent()
      )
    p.file(
      "module/src/main/java/foo/Bar.java",
      """
        package foo;

        import org.jboss.resteasy.plugins.providers.jackson.ResteasyJackson2Provider;
        import com.google.inject.Guice;
        import org.jboss.resteasy.plugins.server.servlet.ResteasyBootstrap;
        import org.jspecify.annotations.NullUnmarked;

        @NullUnmarked
        public class Bar {
          private ResteasyJackson2Provider a;

          private Guice b;

          private ResteasyBootstrap c;

          @SuppressWarnings("SystemOut")
          public void useDependencies() {
            IO.println(a);
            IO.println(b.hashCode());
            IO.println(c);
          }
      }
      """
        .trimIndent(),
    )

    p.qualityGate()

    assertTrue { buildFile.readText().isNotBlank() }
  }

  fun GradleProject.defaultGradleProperties(): GradleProject {
    gradlePropertiesFile.writeText(
      """
      org.gradle.configuration-cache=true
      org.gradle.caching=true
      #org.gradle.unsafe.isolated-projects=true
      ENABLE_AUTO_STRUCTURE=true
      """
        .trimIndent()
    )
    return this
  }

  fun GradleProject.withMinimalStructure(): GradleProject {
    defaultGradleProperties()
    settingsFile.writeText(
      """
      plugins {
        id("com.profiletailors.build.feature.repositories")
        id("com.profiletailors.build.feature.project-structure")
      }
      rootProject.name = "test-project"
      """
        .trimIndent()
    )
    rootBuildFile.writeText(
      """
      plugins {
        id("com.autonomousapps.dependency-analysis")
      }
      """
        .trimIndent()
    )
    versions.writeText(
      """
      plugins {
        id("com.profiletailors.base.lifecycle")
      }
      """
        .trimIndent()
    )
    aggregation.writeText("")
    catalog(
      """
      [libraries]
      foo = { module = "foo:bar" }
      """
        .trimIndent()
    )
    file("app/src/main/resources").mkdirs()
    return this
  }

  fun GradleProject.qualityGate(): BuildResult = runner(listOf("qualityGate")).build()

  fun GradleProject.failQualityGate(): BuildResult = runner(listOf("qualityGate")).buildAndFail()

  companion object {
    @JvmStatic
    fun pluginIds(): Array<String> {
      val pluginList =
        File("src/main/kotlin")
          .listFiles()!!
          .filter { it.isFile && it.name.endsWith(".gradle.kts") }
          .map { it.name.substringBeforeLast(".gradle.kts") }
      return pluginList.toTypedArray()
    }
  }
}
