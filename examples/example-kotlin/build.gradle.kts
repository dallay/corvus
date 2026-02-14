plugins {
  id("com.profiletailors.module.kotlin")
  id("com.profiletailors.module.app")
  id("com.profiletailors.feature.publish-vanniktech")
}

application { mainClass.set("com.profiletailors.kotlin.KotlinApp") }
