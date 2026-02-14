import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

plugins {
  application
  id("com.profiletailors.feature.copy-jar")
}

val jepEnablePreview = project.getPropOrDefault(LocalConfig.Props.JEP_ENABLE_PREVIEW).toBoolean()

tasks.withType<JavaExec>().configureEach {
  if (jepEnablePreview) {
    jvmArgs("--enable-preview")
  }
}

// Make the Jar itself executable by setting the 'Main-Class' manifest attribute.
tasks.jar { manifest { attributes("Main-Class" to application.mainClass) } }
