@file:Suppress("UnstableApiUsage")

plugins { java }

// Specific API fixtures used for testing without live service
val mockApi: SourceSet = sourceSets.create("mockApi")

java.registerFeature(mockApi.name) { usingSourceSet(mockApi) }

// end-to-end tests
testing.suites.register<JvmTestSuite>("testEndToEnd") {
  targets.named("testEndToEnd") {
    testTask {
      group = "verification"
      options {
        this as JUnitPlatformOptions
        excludeTags("slow")
      }
    }
    tasks.check { dependsOn(testTask) }
  }
  // Add a second task for the endToEndTest suite
  targets.register("testEndToEndSlow") {
    testTask {
      group = "verification"
      options {
        this as JUnitPlatformOptions
        includeTags("slow")
      }
    }
  }
}
