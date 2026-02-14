@file:Suppress("UnstableApiUsage")

import com.autonomousapps.tasks.ProjectHealthTask
import com.profiletailors.plugin.InternalDependencies
import com.profiletailors.plugin.libs
import gradle.kotlin.dsl.accessors._2e1eabce6886db90cafe9a120e6529b7.testing
import io.fuchs.gradle.collisiondetector.DetectCollisionsTask
import org.gradle.api.Project
import org.gradle.api.artifacts.Dependency
import org.gradle.api.artifacts.dsl.DependencyHandler
import org.gradle.api.plugins.jvm.JvmTestSuite
import org.gradle.kotlin.dsl.invoke
import org.gradle.kotlin.dsl.withType

object PluginHelpers {

  /** Enables the project health plugin. */
  fun Project.enableProjectHealth() {
    tasks.named("qualityCheck") { dependsOn(tasks.withType<ProjectHealthTask>()) }

    tasks.named("qualityGate") { dependsOn(tasks.withType<ProjectHealthTask>()) }
  }

  /** Enables the collision detection plugin. */
  fun Project.enableCollisionDetection() {
    tasks.named("qualityCheck") { dependsOn(tasks.withType<DetectCollisionsTask>()) }

    tasks.named("qualityGate") { dependsOn(tasks.withType<DetectCollisionsTask>()) }
  }

  /**
   * Enables the testing suite with dependencies.
   *
   * effect:
   * ```
   * dependencies {
   *   implementation(platform("org.junit:junit-bom:<version>"))
   *   implementation(platform("org.assertj:assertj-bom:<version>"))
   *   runtimeOnly("org.junit.platform:junit-platform-launcher")
   *   implementation("org.junit.jupiter:junit-jupiter")
   *   implementation("org.assertj:assertj-core")
   * }
   * ```
   */
  fun Project.useJUnitJupiterM2(junitBomVersion: String = "", assertjBomVersion: String = "") {
    testing.suites.withType<JvmTestSuite> {
      dependencies {
        implementation(
          platform(
            if (junitBomVersion.isNotBlank()) {
              "org.junit:junit-bom:${junitBomVersion}"
            } else {
              runCatching { libs.findLibrary("junitBom").get().get() }
                .getOrElse { InternalDependencies.useLibrary("junitBom") }
                .toString()
            }
          )
        )
        implementation(
          platform(
            if (assertjBomVersion.isNotBlank()) {
              "org.assertj:assertj-bom:${assertjBomVersion}"
            } else {
              runCatching { libs.findLibrary("assertjBom").get().get() }
                .getOrElse { InternalDependencies.useLibrary("assertjBom") }
                .toString()
            }
          )
        )
        runtimeOnly("org.junit.platform:junit-platform-launcher")
        implementation("org.junit.jupiter:junit-jupiter")
        implementation("org.assertj:assertj-core")
      }
    }
  }
}

/**
 * module-info style dependency block.
 *
 * ```
 * Usage:
 * jpmsModule {
 *   require("org.slf4j:slf4j-api:<version>")
 *   requireTransitive(project(":core"))
 * }
 * ```
 */
@Suppress("detekt:UnusedPrivateMember")
fun Project.jpmsModule(configuration: ModuleInfoDependencyScope.() -> Unit) {
  ModuleInfoDependencyScope(dependencies, false).configuration()
}

@Suppress("detekt:UnusedPrivateMember")
fun Project.testJpmsModule(configuration: ModuleInfoDependencyScope.() -> Unit) {
  ModuleInfoDependencyScope(dependencies, true).configuration()
}

class ModuleInfoDependencyScope(
  private val dependencies: DependencyHandler,
  private val isTest: Boolean,
) {

  /** requires */
  fun requires(implementation: Any): Dependency? {
    return if (isTest) {
      dependencies.add("testImplementation", implementation)
    } else {
      dependencies.add("implementation", implementation)
    }
  }

  /** requires transitive */
  fun requiresTransitive(api: Any): Dependency? {
    return if (isTest) {
      dependencies.add("testApi", api)
    } else {
      dependencies.add("api", api)
    }
  }

  /** requires static */
  fun requiresStatic(compileOnly: Any): Dependency? {
    return if (isTest) {
      dependencies.add("testCompileOnly", compileOnly)
    } else {
      dependencies.add("compileOnly", compileOnly)
    }
  }

  /** requires static transitive */
  fun requireStaticsTransitive(compileOnlyApi: Any): Dependency? {
    return if (isTest) {
      dependencies.add("testCompileOnlyApi", compileOnlyApi)
    } else {
      dependencies.add("compileOnlyApi", compileOnlyApi)
    }
  }

  /** requires runtime */
  fun requiresRuntime(runtimeOnly: Any): Dependency? {
    return if (isTest) {
      dependencies.add("testRuntimeOnly", runtimeOnly)
    } else {
      dependencies.add("runtimeOnly", runtimeOnly)
    }
  }
}
