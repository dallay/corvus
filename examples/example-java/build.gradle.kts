plugins {
  id("com.profiletailors.module.java")
  id("com.profiletailors.module.app")
  id("com.profiletailors.feature.publish-vanniktech")
}

application { mainClass.set("com.profiletailors.java.JavaApp") }
