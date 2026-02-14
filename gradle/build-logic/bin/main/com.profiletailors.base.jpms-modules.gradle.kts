import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault
import com.profiletailors.plugin.resetTaskGroup

plugins {
  java
  id("org.gradlex.extra-java-module-info")
  id("org.gradlex.java-module-dependencies")
  id("org.gradlex.java-module-testing")
  id("com.profiletailors.base.jvm-conflict")
}

val jpmsEnabled = project.getPropOrDefault(LocalConfig.Props.JPMS_ENABLED).toBoolean()

// Fix or enhance the metadata of third-party Modules. This is about the metadata in the
// repositories: '*.pom' and '*.module' files.
jvmDependencyConflicts.patch {
  // Avoid conflict between 'javax.activation' and 'jakarta.activation-api' in the
  // detachedConfiguration that is input to AndroidLintTask.lintTool.classpath
  // Because for 'detachedConfigurations' you cannot inject conflict resolution strategies
  // select(JAVAX_ACTIVATION_API, "jakarta.activation:jakarta.activation-api")
  // would not solve the issue.
  module("com.android.tools:repository") {
    removeDependency("com.sun.activation:javax.activation")
    addApiDependency("jakarta.activation:jakarta.activation-api")
  }
  // Register JARs with classifier as features
  module("io.netty:netty-transport-native-epoll") {
    addFeature("linux-x86_64") // refer to as 'io.netty.transport.epoll.linux.x86_64'
    addFeature("linux-aarch_64") // refer to as 'io.netty.transport.epoll.linux.aarch_64'
  }
  module("com.google.guava:guava") {
    // remove annotation libraries we do not need
    removeDependency("com.google.j2objc:j2objc-annotations")
  }
  module("com.github.racc:typesafeconfig-guice") {
    // remove and re-add due to 'no_aop' classifier
    removeDependency("com.google.inject:guice")
    addApiDependency("com.google.inject:guice")
  }
  // Remove transitive dependencies that are not used
  module("io.prometheus:simpleclient") {
    removeDependency("io.prometheus:simpleclient_tracer_otel") // not needed
    removeDependency("io.prometheus:simpleclient_tracer_otel_agent") // not needed
  }
  module("junit:junit") {
    removeDependency("org.hamcrest:hamcrest-core") // not needed
  }
  module("org.jetbrains.kotlin:kotlin-stdlib") {
    removeDependency("org.jetbrains.kotlin:kotlin-stdlib-common") // not needed
  }
  module("biz.aQute.bnd:biz.aQute.bnd.annotation") {
    removeDependency("org.osgi:org.osgi.resource") // split package
    removeDependency("org.osgi:org.osgi.service.serviceloader") // split package
  }
  // Add missing compile time dependencies
  module("org.hyperledger.besu:secp256k1") {
    addApiDependency("net.java.dev.jna:jna") // access annotation at compile time
  }
  module("uk.org.webcompere:system-stubs-jupiter") {
    addApiDependency("org.junit.jupiter:junit-jupiter-api") // needed for super class
  }
  // Add missing runtime dependencies
  module("org.rnorth.duct-tape:duct-tape") {
    addRuntimeOnlyDependency("org.slf4j:slf4j-api") // wrongly marked as provided
  }

  // https://github.com/apache/httpcomponents-client/blob/master/pom.xml
  align(
    "org.apache.httpcomponents.client5:httpclient5",
    "org.apache.httpcomponents.client5:httpclient5-fluent",
    "org.apache.httpcomponents.client5:httpclient5-cache",
    "org.apache.httpcomponents.client5:httpclient5-win",
  )
  // https://github.com/apache/poi/blob/trunk/settings.gradle
  align(
    "org.apache.poi:poi",
    "org.apache.poi:poi-ooxml",
    "org.apache.poi:poi-excelant",
    "org.apache.poi:poi-scratchpad",
  )

  // Reduce scope of transitively added annotation libraries
  val annotationLibrariesCompileTime =
    listOf("com.google.code.findbugs:jsr305", "org.jspecify:jspecify")
  val annotationLibrariesUnused =
    listOf(
      "com.google.android:annotations",
      "org.checkerframework:checker-compat-qual",
      "org.codehaus.mojo:animal-sniffer-annotations",
    )
  val modulesUsingAnnotationLibraries =
    listOf(
      "com.github.ben-manes.caffeine:caffeine",
      "com.google.dagger:dagger-compiler",
      "com.google.dagger:dagger-producers",
      "com.google.dagger:dagger-spi",
      "com.google.guava:guava",
      "com.google.protobuf:protobuf-java-util",
      "io.grpc:grpc-api",
      "io.grpc:grpc-context",
      "io.grpc:grpc-core",
      "io.grpc:grpc-netty",
      "io.grpc:grpc-netty-shaded",
      "io.grpc:grpc-protobuf",
      "io.grpc:grpc-protobuf-lite",
      "io.grpc:grpc-services",
      "io.grpc:grpc-stub",
      "io.grpc:grpc-testing",
      "io.grpc:grpc-util",
    )
  modulesUsingAnnotationLibraries.forEach { module ->
    module(module) {
      annotationLibrariesCompileTime.forEach { reduceToCompileOnlyApiDependency(it) }
      annotationLibrariesUnused.forEach { removeDependency(it) }
    }
  }
}

// Fix or enhance the 'module-info.class' of third-party Modules. This is about the
// 'module-info.class' inside the Jar files. In our full Java Modules setup every
// Jar needs to have this file. If it is missing, it is added by what is configured here.
extraJavaModuleInfo {
  skipLocalJars = true
  deriveAutomaticModuleNamesFromFileNames = true
  failOnMissingModuleInfo = false
  failOnAutomaticModules = false // Only allow Jars with 'module-info' on all module paths
  versionsProvidingConfiguration = "mainRuntimeClasspath"

  // Disable module Jar patching for the JMH runtime classpath.
  runCatching { sourceSets.named("jmh") }.getOrNull()?.let { deactivate(it) }
}

configurations.implementation {
  withDependencies {
    // If dependencies are not defined explicitly, auto-depend on all projects that are known to
    // contain Java Modules.
    if (isEmpty()) {
      javaModuleDependencies.allLocalModules().forEach { localModule ->
        project.dependencies.add("implementation", project.project(localModule.projectPath))
      }
    }
  }
}

if (jpmsEnabled) {
  // We assume that the module name (defined in module-info.java) corresponds to a combination of
  // group and project name, see: https://youtrack.jetbrains.com/issue/KT-55389
  val moduleName = "${project.group}.${project.name}"
  val testModuleName = "${moduleName}.test"

  tasks.withType<JavaCompile>().configureEach {
    options.apply {
      javaModuleVersion.set(project.version.toString())
      // Compiling module-info in the 'main/java' folder needs to see already compiled Kotlin code
      compilerArgs.addAll(
        listOf("--patch-module", "$moduleName=${sourceSets.main.get().output.asPath}")
      )
    }
  }

  tasks.compileTestJava {
    options.apply {
      // Compiling module-info in the 'test/java' folder needs to see already compiled Kotlin code
      compilerArgs.addAll(
        listOf("--patch-module", "$testModuleName=${sourceSets.test.get().output.asPath}")
      )
    }
  }
}

if (jpmsEnabled) {
  listOf(
      "moduleDependencies" to "module",
      "analyzeModulePath" to "module",
      "moduleDescriptorRecommendations" to "module",
      "recommendModuleVersions" to "module",
      ".*ModuleInfo.*".toRegex() to "module",
    )
    .forEach { resetTaskGroup(it.first, it.second) }
}
