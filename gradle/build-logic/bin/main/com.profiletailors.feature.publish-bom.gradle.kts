plugins {
  `java-platform` // combination with the `java` and `java-library` plugins
  id("com.profiletailors.feature.publish-base")
}

val publishTaskName = "mavenBom"

val mavenBom =
  publishing.publications.create<MavenPublication>(publishTaskName) {
    from(components["javaPlatform"])
  }

signing { sign(mavenBom) }
