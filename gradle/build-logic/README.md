### The Convention Plugins

- [gradle-project-setup-howto](https://github.com/jjohannes/gradle-project-setup-howto)
- [organizing_gradle_projects](https://docs.gradle.org/current/userguide/organizing_gradle_projects.html)

Convention plugins are used to configure each aspect of the build centrally. To keep it structured, we put
them into four categories: _Base_, _Feature_, _Check_, _Report_. Below you find all plugins listed. For more details,
inspect the corresponding plugin files.

DevSecOps toolchain:

- code-format
  - [Spotless](https://github.com/diffplug/spotless), [Detekt](https://github.com/detekt/detekt) [Checkstyle](https://github.com/checkstyle/checkstyle)
- code-quality
  - [Error Prone](https://github.com/PicnicSupermarket/error-prone-support), [Pmd](https://github.com/pmd/pmd), [Spotbugs](https://github.com/spotbugs/spotbugs)
- code-refactor
  - [OpenRewrite](https://github.com/openrewrite/rewrite)
- code-security(CVEs)
  - [Renovate](https://docs.renovatebot.com/), [DependencyCheck](https://github.com/dependency-check/DependencyCheck)
- code-coverage
  - [Jacoco](https://github.com/jacoco/jacoco), [kover](https://github.com/Kotlin/kotlinx-kover/issues/746)
- CI/CD(Continuous Integration/Continuous Deployment)
  - [Jenkins](https://www.jenkins.io/), [GitHub Actions](https://github.com/features/actions)
- Tracing, Logging, Reporting, Monitoring, Automation...
- DevSecOps
  - ([CodeQL](https://codeql.github.com/), [Mend](https://www.mend.io/)), ([Codecov](https://about.codecov.io/)), ([Codacy](https://www.codacy.com/), [SonarQube](https://www.sonarsource.com/), [Qodana](https://www.jetbrains.com/qodana/))

Code Quality: `Dependency Security` `Test Coverage Visualization` `Comprehensive Analysis`

#### Build Script

_Build_ script plugins are applied to the root project.

[com.profiletailors.build.feature.build-cache.settings.gradle.kts](src/main/kotlin/com.profiletailors.build.feature.build-cache.settings.gradle.kts)
[com.profiletailors.build.feature.catalogs.settings.gradle.kts](src/main/kotlin/com.profiletailors.build.feature.catalogs.settings.gradle.kts)
[com.profiletailors.build.feature.project-structure.settings.gradle.kts](src/main/kotlin/com.profiletailors.build.feature.project-structure.settings.gradle.kts)
[com.profiletailors.build.feature.repositories.settings.gradle.kts](src/main/kotlin/com.profiletailors.build.feature.repositories.settings.gradle.kts)
[com.profiletailors.build.report.develocity.settings.gradle.kts](src/main/kotlin/com.profiletailors.build.report.develocity.settings.gradle.kts)
[com.profiletailors.build.settings.gradle.kts](src/main/kotlin/com.profiletailors.build.settings.gradle.kts)

#### Base Plugins

_Base_ plugins need to be used in all Modules to establish a certain foundation for the setup.
For example, the same dependency management configuration should be applied everywhere to consistently use the same
3rd party libraries everywhere.

[com.profiletailors.base.identity.gradle.kts](src/main/kotlin/com.profiletailors.base.identity.gradle.kts)
[com.profiletailors.base.jpms-modules.gradle.kts](src/main/kotlin/com.profiletailors.base.jpms-modules.gradle.kts)
[com.profiletailors.base.jvm-conflict.gradle.kts](src/main/kotlin/com.profiletailors.base.jvm-conflict.gradle.kts)
[com.profiletailors.base.lifecycle.gradle.kts](src/main/kotlin/com.profiletailors.base.lifecycle.gradle.kts)

#### Check Plugins (optional)

Check plugins help with keeping the software maintainable over time.
They check things like the dependency setup or code formatting.
More style checkers or static code analysis tools could be added in this category.
Check plugins are not necessarily needed to build a working software.

[com.profiletailors.check.actionlint-root.gradle.kts](src/main/kotlin/com.profiletailors.check.actionlint-root.gradle.kts)
[com.profiletailors.check.dependencies.gradle.kts](src/main/kotlin/com.profiletailors.check.dependencies.gradle.kts)
[com.profiletailors.check.dependencies-root.gradle.kts](src/main/kotlin/com.profiletailors.check.dependencies-root.gradle.kts)
[com.profiletailors.check.format-base.gradle.kts](src/main/kotlin/com.profiletailors.check.format-base.gradle.kts)
[com.profiletailors.check.format-gradle.gradle.kts](src/main/kotlin/com.profiletailors.check.format-gradle.gradle.kts)
[com.profiletailors.check.format-gradle-root.gradle.kts](src/main/kotlin/com.profiletailors.check.format-gradle-root.gradle.kts)
[com.profiletailors.check.format-java.gradle.kts](src/main/kotlin/com.profiletailors.check.format-java.gradle.kts)
[com.profiletailors.check.format-kotlin.gradle.kts](src/main/kotlin/com.profiletailors.check.format-kotlin.gradle.kts)
[com.profiletailors.check.format-misc.gradle.kts](src/main/kotlin/com.profiletailors.check.format-misc.gradle.kts)
[com.profiletailors.check.quality-check-style.gradle.kts](src/main/kotlin/com.profiletailors.check.quality-check-style.gradle.kts)
[com.profiletailors.check.quality-detekt.gradle.kts](src/main/kotlin/com.profiletailors.check.quality-detekt.gradle.kts)
[com.profiletailors.check.quality-nullaway.gradle.kts](src/main/kotlin/com.profiletailors.check.quality-nullaway.gradle.kts)
[com.profiletailors.check.quality-pmd.gradle.kts](src/main/kotlin/com.profiletailors.check.quality-pmd.gradle.kts)
[com.profiletailors.check.quality-spotbugs.gradle.kts](src/main/kotlin/com.profiletailors.check.quality-spotbugs.gradle.kts)

#### Report Plugins (optional)

_Report_ plugins add reporting functionality to discover potential issues with the software or the build setup.
They may generate data that is picked up and displayed by external tools like
[Develocity](https://gradle.com/scans/gradle/) or [Dependency Track](https://dependencytrack.org/).
More reporting tools may be integrated in this category.
Report plugins are not necessarily needed to build a working software.

[com.profiletailors.report.code-coverage.gradle.kts](src/main/kotlin/com.profiletailors.report.code-coverage.gradle.kts)
[com.profiletailors.report.sbom.gradle.kts](src/main/kotlin/com.profiletailors.report.sbom.gradle.kts)
[com.profiletailors.report.test.gradle.kts](src/main/kotlin/com.profiletailors.report.test.gradle.kts)

#### Feature Plugins (optional)

Each _feature_ plugin configures one aspect of building the different software(java, android, kotlin, etc.) – like
_compiling code_ or _testing code_.

[com.profiletailors.feature.aggregation.gradle.kts](src/main/kotlin/com.profiletailors.feature.aggregation.gradle.kts)
[com.profiletailors.feature.benchmark.gradle.kts](src/main/kotlin/com.profiletailors.feature.benchmark.gradle.kts)
[com.profiletailors.feature.checksum.gradle.kts](src/main/kotlin/com.profiletailors.feature.checksum.gradle.kts)
[com.profiletailors.feature.compile-java.gradle.kts](src/main/kotlin/com.profiletailors.feature.compile-java.gradle.kts)
[com.profiletailors.feature.compile-java-ext.gradle.kts](src/main/kotlin/com.profiletailors.feature.compile-java-ext.gradle.kts)
[com.profiletailors.feature.compile-kotlin.gradle.kts](src/main/kotlin/com.profiletailors.feature.compile-kotlin.gradle.kts)
[com.profiletailors.feature.compile-kotlin-ext.gradle.kts](src/main/kotlin/com.profiletailors.feature.compile-kotlin-ext.gradle.kts)
[com.profiletailors.feature.copy-jar.gradle.kts](src/main/kotlin/com.profiletailors.feature.copy-jar.gradle.kts)
[com.profiletailors.feature.doc-java.gradle.kts](src/main/kotlin/com.profiletailors.feature.doc-java.gradle.kts)
[com.profiletailors.feature.doc-kotlin.gradle.kts](src/main/kotlin/com.profiletailors.feature.doc-kotlin.gradle.kts)
[com.profiletailors.feature.git-hook.gradle.kts](src/main/kotlin/com.profiletailors.feature.git-hook.gradle.kts)
[com.profiletailors.feature.openrewrite.gradle.kts](src/main/kotlin/com.profiletailors.feature.openrewrite.gradle.kts)
[com.profiletailors.feature.publish-base.gradle.kts](src/main/kotlin/com.profiletailors.feature.publish-base.gradle.kts)
[com.profiletailors.feature.publish-bom.gradle.kts](src/main/kotlin/com.profiletailors.feature.publish-bom.gradle.kts)
[com.profiletailors.feature.publish-library.gradle.kts](src/main/kotlin/com.profiletailors.feature.publish-library.gradle.kts)
[com.profiletailors.feature.publish-vanniktech.gradle.kts](src/main/kotlin/com.profiletailors.feature.publish-vanniktech.gradle.kts)
[com.profiletailors.feature.shadow.gradle.kts](src/main/kotlin/com.profiletailors.feature.shadow.gradle.kts)
[com.profiletailors.feature.test.gradle.kts](src/main/kotlin/com.profiletailors.feature.test.gradle.kts)
[com.profiletailors.feature.test-end2end.gradle.kts](src/main/kotlin/com.profiletailors.feature.test-end2end.gradle.kts)
[com.profiletailors.feature.test-fixtures.gradle.kts](src/main/kotlin/com.profiletailors.feature.test-fixtures.gradle.kts)
[com.profiletailors.feature.use-all-catalog-versions.gradle.kts](src/main/kotlin/com.profiletailors.feature.use-all-catalog-versions.gradle.kts)
[com.profiletailors.feature.war.gradle.kts](src/main/kotlin/com.profiletailors.feature.war.gradle.kts)
[com.profiletailors.tools.check-version.gradle.kts](src/main/kotlin/com.profiletailors.tools.check-version.gradle.kts)
[com.profiletailors.tools.spring-openapi.gradle.kts](src/main/kotlin/com.profiletailors.tools.spring-openapi.gradle.kts)

#### Module Plugins

_Module_ plugins combine plugins from all categories above to define _Module Types_ that are then used in the
`build.gradle.kts` files of the individual Modules of our software.

[com.profiletailors.module.android.gradle.kts](src/main/kotlin/com.profiletailors.module.android.gradle.kts)
[com.profiletailors.module.app.gradle.kts](src/main/kotlin/com.profiletailors.module.app.gradle.kts)
[com.profiletailors.module.bom.gradle.kts](src/main/kotlin/com.profiletailors.module.bom.gradle.kts)
[com.profiletailors.module.java.gradle.kts](src/main/kotlin/com.profiletailors.module.java.gradle.kts)
[com.profiletailors.module.kotlin.gradle.kts](src/main/kotlin/com.profiletailors.module.kotlin.gradle.kts)
[com.profiletailors.module.spring-boot.gradle.kts](src/main/kotlin/com.profiletailors.module.spring-boot.gradle.kts)
[com.profiletailors.module.war.gradle.kts](src/main/kotlin/com.profiletailors.module.war.gradle.kts)

#### Gradle Testing Plugins

The [Gradle TestKit](https://docs.gradle.org/current/userguide/test_kit.html) can be used to test plugins.
This can be helpful to enforce a certain structure, e.g. by testing if each plugin works on its own.
And if you add custom tasks and advanced logic, you can add tests for that.

[ConventionPluginTest.kt](src/test/kotlin/com/profiletailors/ConventionPluginTest.kt)

### Continuously build and report using GitHub Actions and Dependabot

- `build.yaml` Configure GitHub to run builds and produce reports (
  👉[inspect](https://github.com/jjohannes/gradle-project-setup-howto/actions/workflows/build.yaml)). Integrates with:
  - [Develocity Build Scans](https://gradle.com/scans/gradle/) (👉[inspect](https://scans.gradle.com/s/h3odwhbjjd2qm))
  - [Gradle Remote Build Cache](https://docs.gradle.com/develocity/bcn/current/)
  - [Reposilite](https://reposilite.com/) (👉[inspect](https://repo.onepiece.software/#/snapshots))
  - [Dependency Track](https://dependencytrack.org/)
- `dependabot.yml` Configure [Dependabot](https://github.com/dependabot) to automatically get
  version updates (👉[inspect](https://github.com/jjohannes/gradle-project-setup-howto/pulls/app%2Fdependabot))

## Links
