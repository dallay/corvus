import com.profiletailors.plugin.GradleExtTool
import com.profiletailors.plugin.environment.EnvAccess
import com.profiletailors.plugin.injected
import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault
import com.profiletailors.plugin.repo.RepositoryConfig
import com.profiletailors.plugin.utils.CatalogUtil
import org.gradle.kotlin.dsl.support.uppercaseFirstChar

plugins {
  idea
  signing // https://docs.gradle.org/current/userguide/signing_plugin.html
  `maven-publish` // https://docs.gradle.org/current/userguide/publishing_maven.html
}

idea {
  module {
    isDownloadSources = true
    isDownloadJavadoc = false
  }
}

publishing.publications.configureEach {
  if (this is MavenPublication) {
    group = project.group
    artifactId = project.name
    version = project.version.toString()
    val projectName = project.group.toString() + ":" + project.name
    // the pom info
    pom {
      name = projectName
      description = project.description.orEmpty().ifBlank { projectName }
      url = project.getPropOrDefault(LocalConfig.Props.POM_URL)
      scm { url = project.getPropOrDefault(LocalConfig.Props.POM_SCM_CONNECTION) }
      licenses { license { url = project.getPropOrDefault(LocalConfig.Props.POM_LICENSE_URL) } }
      developers {
        developer { name = project.getPropOrDefault(LocalConfig.Props.POM_DEVELOPER_NAME) }
      }
    }
  }
}

// https://github.com/vanniktech/gradle-maven-publish-plugin
val vanniktechPlugin = project.plugins.hasPlugin("com.vanniktech.maven.publish")

if (!vanniktechPlugin) {
  // sign
  // see: https://central.sonatype.org/publish/requirements/gpg/
  // see: https://docs.gradle.org.cn/current/userguide/signing_plugin.html
  val signingPassword = project.getPropOrDefault(LocalConfig.Props.GPG_SIGNING_PASSWORD)
  if (signingPassword.isNotBlank()) {
    val signKeyId = project.getPropOrDefault(LocalConfig.Props.GPG_SIGNING_KEY_ID)
    val signSecretKey = project.getPropOrDefault(LocalConfig.Props.GPG_SIGNING_KEY)
    signing {
      if (signKeyId.isNotBlank()) {
        useInMemoryPgpKeys(signKeyId, signSecretKey, signingPassword)
      } else {
        useInMemoryPgpKeys(signSecretKey, signingPassword)
      }
    }
  }
}

tasks.withType<Zip>().configureEach { isZip64 = true }

val mavenCentralUsername = project.getPropOrDefault(LocalConfig.Props.SONATYPE_USERNAME)
val mavenCentralPassword = project.getPropOrDefault(LocalConfig.Props.SONATYPE_PASSWORD)

// The repository to publish to
publishing.repositories {
  maven {
    name = "tmp"
    url = uri(layout.buildDirectory.dir("publishing/tmpRepo"))
  }
  if (GradleExtTool.isSnapshot(version.toString()) && !vanniktechPlugin) {
    // central.sonatype.com 仓库地址（-SNAPSHOT 版本不执行任何验证，会在一段时间（当前为 90 天）后清理）
    // 详见 https://central.sonatype.org/publish/publish-portal-snapshots/
    maven("https://central.sonatype.com/repository/maven-snapshots/") {
      name = "CentralSnapshot"
      if (url.scheme == "https") {
        credentials {
          username = mavenCentralUsername
          password = mavenCentralPassword
        }
      }
    }
  }
  RepositoryConfig.getPrivateRepositories(providers).forEach {
    val pass = it.password
    if (pass.isNotBlank()) {
      maven(it.url) {
        name = "${it.name.lowercase().uppercaseFirstChar()}Pri"
        credentials {
          username = it.username
          password = pass
        }
      }
    }
  }
}

tasks.withType<PublishToMavenRepository>().configureEach {
  val inject = injected
  val metadataXmlProvider =
    CatalogUtil.transformModuleToLibraryMetadata("${project.group}:${project.name}").let {
      provider { it }
    }
  val versionProvider = provider { project.version }
  doLast {
    val provider = inject.providers
    val publishVersion = versionProvider.get()
    val isCI = EnvAccess.isCi(provider)
    // publish[PubName]PublicationTo[RepoName]Repository
    // https://docs.gradle.org/nightly/userguide/publishing_setup.html#sec:basic_publishing
    val taskName = this.name
    //    val repos =
    //      publishing.repositories.filterIsInstance<MavenArtifactRepository>().map {
    //        it.name
    //      }
    val message = "execute task: $taskName, version: $publishVersion"
    if (isCI) {
      logger.lifecycle(message)
      return@doLast
    }
    val toMavenCentral = PublishUnit.getPublishingTaskNameSuffix("MavenCentral")
    if (taskName.endsWith(toMavenCentral)) {
      val openUrl =
        if (GradleExtTool.isSnapshot(publishVersion.toString())) {
          "https://central.sonatype.com/repository/maven-snapshots/" + metadataXmlProvider.get()
        } else "https://central.sonatype.com/publishing/deployments"
      val xmlUrl =
        if (GradleExtTool.isSnapshot(publishVersion.toString())) {
          "https://central.sonatype.com/repository/maven-snapshots/" + metadataXmlProvider.get()
        } else "https://repo1.maven.org/maven2/org/" + metadataXmlProvider.get()
      logger.lifecycle(
        """
        $message
          opening url: $openUrl
          xml url: $xmlUrl
        """
          .trimIndent()
      )
      return@doLast
    }
    val toCentralSnapshot = PublishUnit.getPublishingTaskNameSuffix("CentralSnapshot")
    if (taskName.endsWith(toCentralSnapshot)) {
      val xmlUrl =
        "https://central.sonatype.com/repository/maven-snapshots/" + metadataXmlProvider.get()
      logger.lifecycle(
        """
        $message
          opening url: $xmlUrl
          xml url: $xmlUrl
        """
          .trimIndent()
      )
      return@doLast
    }
    RepositoryConfig.getPrivateRepositories(provider)
      .filter { it.open.isNotBlank() }
      .forEach {
        val toPriRepo =
          PublishUnit.getPublishingTaskNameSuffix("${it.name.lowercase().uppercaseFirstChar()}Pri")
        if (taskName.endsWith(toPriRepo)) {
          val openUrl = it.open
          logger.lifecycle(
            """
            $message
              opening url: $openUrl
            """
              .trimIndent()
          )
        }
      }
  }
}

private object PublishUnit {
  fun getPublishingTaskNameSuffix(repoName: String): String {
    return "To${repoName.uppercaseFirstChar()}Repository"
  }
}
