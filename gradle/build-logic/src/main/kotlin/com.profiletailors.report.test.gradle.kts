@file:Suppress("UnstableApiUsage")

plugins {
  java
  `test-report-aggregation`
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.base.jvm-conflict")
}

// Make aggregation "classpath" use the platform for versions (gradle/versions)
configurations.aggregateTestReportResults { extendsFrom(configurations["internal"]) }

// Integrate testEndToEnd results into the aggregated UNIT_TEST test results
tasks.testAggregateTestReport {
  destinationDirectory = reporting.baseDirectory.dir("tests")
  testResults.from(
    configurations.aggregateTestReportResults
      .get()
      .incoming
      .artifactView {
        withVariantReselection()
        attributes {
          attribute(Category.CATEGORY_ATTRIBUTE, objects.named(Category.VERIFICATION))
          attribute(TestSuiteName.TEST_SUITE_NAME_ATTRIBUTE, objects.named("testEndToEnd"))
          attribute(
            VerificationType.VERIFICATION_TYPE_ATTRIBUTE,
            objects.named(VerificationType.TEST_RESULTS),
          )
        }
      }
      .files
  )
}

// Generate report when running 'check'
tasks.check { dependsOn(tasks.testAggregateTestReport) }
