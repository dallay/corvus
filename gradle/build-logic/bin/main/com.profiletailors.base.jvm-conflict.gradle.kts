import com.profiletailors.plugin.DefaultProjects

plugins { id("org.gradlex.jvm-dependency-conflict-resolution") }

// Configure consistent resolution across the whole project
val consistentResolutionAttribute: Attribute<String> =
  Attribute.of("consistent-resolution", String::class.java)

configurations.create(
  "allDependencies",
  Action {
    isCanBeConsumed = true
    isCanBeResolved = false
    sourceSets.configureEach {
      extendsFrom(
        configurations[this.implementationConfigurationName],
        configurations[this.compileOnlyConfigurationName],
        configurations[this.runtimeOnlyConfigurationName],
        configurations[this.annotationProcessorConfigurationName],
      )
    }
    attributes {
      attribute(consistentResolutionAttribute, "global")
      attribute(Usage.USAGE_ATTRIBUTE, objects.named(Usage.JAVA_RUNTIME))
      attribute(Category.CATEGORY_ATTRIBUTE, objects.named(Category.LIBRARY))
      attribute(LibraryElements.LIBRARY_ELEMENTS_ATTRIBUTE, objects.named(LibraryElements.JAR))
      attribute(Bundling.BUNDLING_ATTRIBUTE, objects.named(Bundling.EXTERNAL))
    }
  },
)

jvmDependencyConflicts {
  // Configure build wide consistent resolution. That is, the versions that are used on the
  // runtime classpath of the web applications should also be used in all other places
  // (e.g. also when compiling a project at the bottom of the dependency graph that does not
  // see most of the other dependencies that may influence the version choices).

  consistentResolution {
    if (project.path == ":") {
      // single project build, e.g. for examples
      providesVersions(project.path)
    } else {
      val providedVersionsProject =
        project.findProject(DefaultProjects.aggregationPath)?.path ?: ":"
      providesVersions(providedVersionsProject)
      project.findProject(DefaultProjects.versionsPath)?.path?.let { platform(it) }
    }
  }

  // Configure logging capabilities plugin to default to Slf4JSimple
  logging { enforceSlf4JSimple() }
}

configurations.getByName("mainRuntimeClasspath") {
  attributes.attribute(consistentResolutionAttribute, "global")
}

// In case published versions of a module are also available, always prefer the local one
configurations.configureEach { resolutionStrategy.preferProjectModules() }
