@file:Suppress("UnstableApiUsage")

import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.targets.native.tasks.KotlinNativeTest

plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.check.format-gradle")
  id("com.profiletailors.check.format-kotlin")
  alias(libs.plugins.org.jetbrains.kotlin.multiplatform)
  alias(libs.plugins.com.android.kotlin.multiplatform.library)
  alias(libs.plugins.org.jetbrains.compose)
  alias(libs.plugins.org.jetbrains.kotlin.plugin.compose)
  id("org.jetbrains.kotlinx.kover")
}

val isMacOs = System.getProperty("os.name").startsWith("Mac", ignoreCase = true)

kotlin {
  jvmToolchain(libs.versions.jdk.get().toInt())

  androidLibrary {
    namespace = "com.profiletailors.corvus.shared"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    minSdk = libs.versions.android.minSdk.get().toInt()
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
    androidResources { enable = true }
  }

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
        implementation(compose.materialIconsExtended)
      }
    }

    val commonTest by getting { dependencies { implementation(kotlin("test")) } }

    val jvmMain by getting {
      dependencies {
        implementation(compose.desktop.currentOs)
        implementation(libs.kotlinx.coroutines.swing)
      }
    }

    val androidMain by getting {
      dependencies {
        implementation(libs.compose.ui.tooling.preview)
        implementation(libs.androidx.activity.compose)
      }
    }
  }
}

dependencies { add("androidRuntimeClasspath", libs.compose.ui.tooling) }

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
