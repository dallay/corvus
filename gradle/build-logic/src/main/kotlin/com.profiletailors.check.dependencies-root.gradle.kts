@file:Suppress("UnstableApiUsage")

plugins { id("com.autonomousapps.dependency-analysis") }

if (path == ":") {
  dependencyAnalysis {
    // https://github.com/autonomousapps/dependency-analysis-gradle-plugin/issues/1234
    // https://github.com/autonomousapps/dependency-analysis-gradle-plugin/issues/1485
    structure {
      bundle("spring-boot-starter-webmvc") {
        primary("org.springframework.boot:spring-boot-starter-webmvc")
        includeDependency("org.springframework.boot:spring-boot")
        includeDependency("org.springframework.boot:spring-boot-autoconfigure")
        includeDependency("org.springframework:spring-web")
        includeDependency("org.springframework:spring-context")
        includeDependency("org.springframework:spring-aop")
        includeDependency("org.springframework:spring-beans")
        includeDependency("org.springframework:spring-expression")
        includeDependency("org.springframework:spring-core")
      }
      bundle("spring-boot-starter-webmvc-test") {
        primary("org.springframework.boot:spring-boot-starter-webmvc-test")
        includeDependency("org.springframework.boot:spring-boot-webmvc-test")
        includeDependency("org.springframework.boot:spring-boot-resttestclient")
        includeDependency("org.springframework.boot:spring-boot-test")
        includeDependency("org.springframework:spring-test")
        includeDependency("org.junit.jupiter:junit-jupiter-api")
        includeDependency("org.assertj:assertj-core")
      }
    }
  }
}
