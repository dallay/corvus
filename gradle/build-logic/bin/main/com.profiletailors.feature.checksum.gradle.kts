import com.profiletailors.plugin.tasks.DirectoryChecksum

plugins { java }

// Generate additional resources required at application runtime
// This is an example for creating and integrating a custom task implementation.
val resourcesChecksum =
  tasks.register<DirectoryChecksum>("resourcesChecksum") {
    inputDirectory.set(layout.projectDirectory.dir("src/main/resources"))
    checksumFile.set(layout.buildDirectory.file("generated-resources/sha256/resources.sha256"))
  }

tasks.processResources { from(resourcesChecksum) }
