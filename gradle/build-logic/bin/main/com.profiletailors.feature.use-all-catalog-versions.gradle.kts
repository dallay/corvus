plugins { `java-platform` }

// Allow upgrading/downgrading (transitive) versions via catalog by adding strict
// constraints
// Only allow strict versions in the transitive catalog
// Also You can write you own submodules to transitive catalogs
dependencies.constraints {
  runCatching { extensions.findByType<VersionCatalogsExtension>() }
    .also {
      if (it.isSuccess) {
        it
          .getOrThrow()
          ?.filter { versionCatalog -> versionCatalog.name != "libs" }
          ?.forEach { versionCatalog ->
            versionCatalog.libraryAliases
              .map { alias -> versionCatalog.findLibrary(alias).get().get() }
              .forEach { entry ->
                val version = entry.version
                if (version != null) {
                  api(entry) { version { require(version) } }
                }
              }
          }
      }
    }
}
