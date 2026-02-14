import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

plugins {
  id("org.jetbrains.kotlin.jvm")
  // https://kotlinlang.org/docs/dokka-migration.html
  id("org.jetbrains.dokka")
}

val docFailOnError = project.getPropOrDefault(LocalConfig.Props.DOC_FAIL_ON_ERROR).toBoolean()

gradle.projectsEvaluated {
  val vanniktechPlugin = plugins.hasPlugin("com.vanniktech.maven.publish")
  if (!vanniktechPlugin) {
    val javadocJar = tasks.findByName("javadocJar")
    if (javadocJar != null) {
      tasks.named<Jar>("javadocJar") {
        val dokka = tasks.dokkaGeneratePublicationHtml
        archiveClassifier.set("javadoc")

        dependsOn(dokka)
        // the default output directory is `/build/dokka/html`
        // dokka.get().outputDirectory
        from(dokka)
      }
    }
  }
}

dokka { dokkaPublications.html { failOnWarning = docFailOnError } }

tasks.register("docKotlin") {
  group = "docs"
  description = "Generate Kotlin docs [group = docs]"
  dependsOn(tasks.dokkaGeneratePublicationHtml)
}
