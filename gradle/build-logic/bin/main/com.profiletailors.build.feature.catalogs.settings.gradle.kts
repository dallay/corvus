@file:Suppress("UnstableApiUsage")

dependencyResolutionManagement {
  versionCatalogs {
    layout.settingsDirectory
      .dir("gradle")
      .asFile
      .listFiles()
      .filter {
        it.isFile &&
          it.name != "libs.versions.toml" &&
          !it.name.startsWith("__") &&
          it.name.endsWith(".versions.toml")
      }
      .forEach {
        val name = it.name.removeSuffix(".versions.toml")
        create(name) { from(layout.settingsDirectory.files(it.path)) }
      }
  }
}
