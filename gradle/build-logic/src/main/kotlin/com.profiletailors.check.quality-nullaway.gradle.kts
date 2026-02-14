import com.profiletailors.plugin.InternalDependencies
import com.profiletailors.plugin.libs
import net.ltgt.gradle.errorprone.errorprone
import net.ltgt.gradle.nullaway.nullaway

plugins {
  java
  // https://github.com/tbroyer/gradle-errorprone-plugin
  id("net.ltgt.errorprone")
  // https://github.com/tbroyer/gradle-nullaway-plugin
  id("net.ltgt.nullaway")
}

dependencies {
  compileOnly(
    runCatching { libs.findLibrary("jspecify").get().get() }
      .getOrElse { InternalDependencies.useLibrary("jspecify") }
  )
  // https://github.com/PicnicSupermarket/error-prone-support/tree/master/error-prone-contrib/src/main/java/tech/picnic/errorprone/bugpatterns
  // https://github.com/PicnicSupermarket/error-prone-support/blob/master/error-prone-contrib/src/main/java/tech/picnic/errorprone/refasterrules/
  listOf("errorProneCore", "errorProneContrib", "refasterRunner", "nullaway").forEach { name ->
    errorprone(
      runCatching { libs.findLibrary(name).get().get() }
        .getOrElse { InternalDependencies.useLibrary(name) }
    )
  }
}

// https://github.com/google/error-prone/issues/623
// default excludes.
val defaultErrorProneExcludes = "(.*/)?(nocheck|autogen|generated)/.*\\.java"

tasks.withType<JavaCompile>().configureEach {
  options.errorprone {
    excludedPaths = defaultErrorProneExcludes
    isEnabled = true
    allSuggestionsAsWarnings = true
    allDisabledChecksAsWarnings = true
    disableWarningsInGeneratedCode = true

    // https://errorprone.info/docs/flags
    // -Xep:<checkName>[:severity] severity is one of {“OFF”, “WARN”, “ERROR”}
    errorproneArgs.add(
      buildString {
        append("-XepOpt:Refaster:NamePattern=^")
        defaultDisabledRules().forEach { rule ->
          append("(?!")
          append(rule)
          append(".*)")
        }
        append(".*")
      }
    )
    defaultDisabledChecks().forEach { disable(it) }

    nullaway {
      // Use JSpecify semantics
      jspecifyMode = true
      // Only analyze @NullMarked code
      onlyNullMarked = true
      // Do not strongly validate generated code
      treatGeneratedAsUnannotated = true
      // Optional.get() check
      checkOptionalEmptiness = true
      // Support assert / Truth / AssertJ
      assertsEnabled = true
      handleTestAssertionLibraries = true
      // Contract analysis (@Contract)
      checkContracts = true
      // Suggest suppressions
      suggestSuppressions = true
      suppressionNameAliases.add("NullAway")
      // Force as error
      error()
    }
  }
}

tasks.compileTestJava { options.errorprone { isEnabled = false } }

/*
 * Add other Error Prone flags here. See:
 * - https://github.com/tbroyer/gradle-errorprone-plugin#configuration
 * - https://errorprone.info/docs/flags
 * - https://github.com/ben-manes/caffeine/blob/master/gradle/plugins/src/main/kotlin/quality/errorprone.caffeine.gradle.kts
 */

fun defaultDisabledChecks(): List<String> {
  return listOf("AddNullMarkedToClass", "MissingSummary", "Java8ApiChecker")
}

@Suppress("CanConvertToMultiDollarString")
fun defaultDisabledRules() = listOf("ImmutableTableRules\\\$ImmutableTableBuilder")
