@file:Suppress("UnstableApiUsage")

import com.android.build.api.dsl.ApplicationExtension

// REMOVED: Client-first Android does NOT package local runtime by default.
// Android now connects to an existing runtime via endpoint URL or trusted companion.
// The optional local-host advanced mode is no longer the default path.

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

  // REMOVED: No default jniLibs for local runtime packaging
  // Android is now a client-first surface - connects to remote runtime

  buildTypes { getByName("release") { isMinifyEnabled = false } }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
}

dependencies { implementation(projects.composeApp) }
