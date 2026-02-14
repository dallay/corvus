@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.environment.buildProperties

val buildProperties = project.buildProperties()
val applicationIdString: String = buildProperties.getProperty("applicationId", "")
val versionNameString: String = buildProperties.getProperty("versionName", "").ifBlank { "1.0.0" }

// configure<BaseAppModuleExtension> {
//  compileSdk = 35
//  namespace = "${project.group}.${project.name}"
//  defaultConfig {
//    applicationId = applicationIdString
//    minSdk = 26
//    targetSdk = 35
//    versionCode = (System.currentTimeMillis() / 1000 / 60).toInt()
//    versionName = versionNameString
//
//    testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
//  }
//
//  buildTypes.getByName("release") { isMinifyEnabled = true }
//
//  compileOptions {
//    sourceCompatibility = JavaVersion.VERSION_17
//    targetCompatibility = JavaVersion.VERSION_17
//  }
//
//  packaging { resources.excludes.add("META-INF/**") }
// }
//
// dependencies { add("androidTestRuntimeOnly", "androidx.test.espresso:espresso-core") }
