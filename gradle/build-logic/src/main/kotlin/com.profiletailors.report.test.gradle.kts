@file:Suppress("UnstableApiUsage")

plugins {
  java
  `test-report-aggregation`
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.base.jvm-conflict")
}

// Make aggregation "classpath" use the platform for versions (gradle/versions)
configurations.aggregateTestReportResults { extendsFrom(configurations["internal"]) }

fun testResultsFor(testSuiteName: String) =
  configurations.aggregateTestReportResults
    .get()
    .incoming
    .artifactView {
      withVariantReselection()
      attributes {
        attribute(Category.CATEGORY_ATTRIBUTE, objects.named(Category.VERIFICATION))
        attribute(TestSuiteName.TEST_SUITE_NAME_ATTRIBUTE, objects.named(testSuiteName))
        attribute(
          VerificationType.VERIFICATION_TYPE_ATTRIBUTE,
          objects.named(VerificationType.TEST_RESULTS),
        )
      }
    }
    .files

// Aggregate default unit test results and include end-to-end suite when present.
tasks.testAggregateTestReport {
  destinationDirectory = reporting.baseDirectory.dir("tests")
  testResults.from(testResultsFor("test"))
  testResults.from(testResultsFor("jvmTest"))
  testResults.from(testResultsFor("testEndToEnd"))
}

// Generate report when running 'check'
tasks.check { dependsOn(tasks.testAggregateTestReport) }
