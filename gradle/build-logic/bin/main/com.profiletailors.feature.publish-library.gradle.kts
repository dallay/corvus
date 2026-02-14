import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

plugins {
  java
  id("com.profiletailors.feature.publish-base")
}

val docJarEnabled = project.getPropOrDefault(LocalConfig.Props.DOC_JAR_ENABLED).toBoolean()

/*
 * `java-library` include the `java` plugin.
 * The plugin exposes two configurations that can be used to declare dependencies: api and implementation.
 *
 * Publish with sources
 */
java {
  val signingPassword = project.getPropOrDefault(LocalConfig.Props.GPG_SIGNING_PASSWORD)
  if (signingPassword.isNotBlank()) {
    withSourcesJar()
    if (docJarEnabled) {
      withJavadocJar()
    }
  }
}

val publishTaskName = "mavenLibrary"

val mavenLibrary =
  publishing.publications.create<MavenPublication>(publishTaskName) {
    from(components["java"])

    // We use consistent resolution + a platform for controlling versions
    // -> Publish the versions that are the result of the consistent resolution
    versionMapping { allVariants { fromResolutionResult() } }
  }

signing { sign(mavenLibrary) }
