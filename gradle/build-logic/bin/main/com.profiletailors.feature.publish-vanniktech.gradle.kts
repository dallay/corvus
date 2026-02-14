import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

plugins {
  id(
    "com.vanniktech.maven.publish"
  ) // https://plugins.gradle.org/plugin/com.vanniktech.maven.publish
  id("com.profiletailors.feature.publish-base")
}

/*
 * docs
 * - https://vanniktech.github.io/gradle-maven-publish-plugin/central/
 * - https://vanniktech.github.io/gradle-maven-publish-plugin/what/
 */
mavenPublishing {
  // see: https://github.com/vanniktech/gradle-maven-publish-plugin
  // mavenCentralPublishing=true
  // signAllPublications=true
  coordinates(project.group.toString(), project.name, project.version.toString())
}

publishing.publications.configureEach {
  if (this is MavenPublication) {
    /** see [com.vanniktech.maven.publish.JavaLibrary] */
    if (this.name == "maven") {
      // We use consistent resolution + a platform for controlling versions
      // -> Publish the versions that are the result of the consistent resolution
      versionMapping { allVariants { fromResolutionResult() } }
    }
  }
}

val docJarEnabled = project.getPropOrDefault(LocalConfig.Props.DOC_JAR_ENABLED).toBoolean()

if (!docJarEnabled) {
  /** see [com.vanniktech.maven.publish.JavadocJar] */
  gradle.projectsEvaluated {
    val vanniktechPluginDocTasks = listOf("emptyJavadocJar", "plainJavadocJar", "dokkaJavadocJar")
    tasks
      .matching { task -> vanniktechPluginDocTasks.any { docTask -> task.name.endsWith(docTask) } }
      .forEach { it.enabled = false }
  }
}
