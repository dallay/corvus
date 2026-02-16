@file:Suppress("UnstableApiUsage")

import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.targets.native.tasks.KotlinNativeTest

plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.check.format-gradle")
  id("com.profiletailors.check.format-kotlin")
  alias(libs.plugins.org.jetbrains.kotlin.multiplatform)
  alias(libs.plugins.org.jetbrains.compose)
  alias(libs.plugins.org.jetbrains.kotlin.plugin.compose)
}

val isMacOs = System.getProperty("os.name").startsWith("Mac", ignoreCase = true)

kotlin {
  jvmToolchain(libs.versions.jdk.get().toInt())

  // Android source set is preserved under src/androidMain and can be re-enabled once AGP/KMP
  // compatibility is aligned for this baseline.
  if (isMacOs) {
    listOf(iosArm64(), iosSimulatorArm64()).forEach { iosTarget ->
      iosTarget.binaries.framework {
        baseName = "ComposeApp"
        isStatic = true
      }
    }
  }

  jvm()

  sourceSets {
    val commonMain by getting {
      dependencies {
        implementation(libs.compose.runtime)
        implementation(libs.compose.foundation)
        implementation(libs.compose.material3)
        implementation(libs.compose.ui)
        implementation(libs.compose.components.resources)
        implementation(libs.compose.ui.tooling.preview)
        implementation(libs.androidx.lifecycle.viewmodel.compose)
        implementation(libs.androidx.lifecycle.runtime.compose)
      }
    }

    val commonTest by getting { dependencies { implementation(kotlin("test")) } }

    val jvmMain by getting {
      dependencies {
        implementation(compose.desktop.currentOs)
        implementation(libs.kotlinx.coroutines.swing)
      }
    }
  }
}

compose.desktop {
  application {
    mainClass = "com.profiletailors.corvus.MainKt"

    nativeDistributions {
      targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)
      packageName = "com.profiletailors.corvus"
      packageVersion = "1.0.0"
    }
  }
}

tasks.withType<KotlinNativeTest>().configureEach { enabled = false }

tasks.matching { it.name.startsWith("linkReleaseFrameworkIos") }.configureEach { enabled = false }
