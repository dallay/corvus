@file:Suppress("UnstableApiUsage")

import com.android.build.api.dsl.ApplicationExtension

plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.check.format-gradle")
  alias(libs.plugins.com.android.application)
}

extensions.configure<ApplicationExtension>("android") {
  namespace = "com.profiletailors.corvus"
  compileSdk = libs.versions.android.compileSdk.get().toInt()

  defaultConfig {
    applicationId = "com.profiletailors.corvus"
    minSdk = libs.versions.android.minSdk.get().toInt()
    targetSdk = libs.versions.android.targetSdk.get().toInt()
    versionCode = 1
    versionName = "1.0"
  }

  packaging { resources { excludes += "/META-INF/{AL2.0,LGPL2.1}" } }

  buildTypes { getByName("release") { isMinifyEnabled = false } }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
}

dependencies { implementation(projects.composeApp) }
