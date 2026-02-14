@file:Suppress("UnstableApiUsage")

import org.openapitools.generator.gradle.plugin.tasks.GenerateTask
import org.springframework.boot.gradle.tasks.buildinfo.BuildInfo

plugins {
  id("com.profiletailors.module.spring-boot")
  id("org.openapi.generator")
}

val resourcesDir: Directory = layout.settingsDirectory.dir("build/open_api/${project.name}")
val inputFile: RegularFile = resourcesDir.file("oas-api.json")
val outputDirectory: Directory = resourcesDir.dir("openapi")

val templateDirectory: File =
  layout.projectDirectory.file("configs/openapi/template").asFile.takeIf { it.exists() }
    ?: isolated.rootProject.projectDirectory
      .file("gradle/configs/openapi/scripts/typescript-fetch")
      .asFile

tasks.register<GenerateTask>("openapiTypescriptFetch") {
  group = "toolbox"
  cleanupOutput = true
  templateDir.set(templateDirectory.path)
  inputSpec.set(inputFile.asFile.invariantSeparatorsPath)
  outputDir.set(outputDirectory.asFile.invariantSeparatorsPath)

  // https://openapi-generator.tech/docs/generators/typescript-fetch
  generatorName.set("typescript-fetch")
  apiPackage.set("apis")
  modelPackage.set("models")
  configOptions.put("disallowAdditionalPropertiesIfNotPresent", "false")
  configOptions.put("enumPropertyNaming", "original")
  configOptions.put("enumUnknownDefaultCase", "true")
  configOptions.put("enumPropertyNamingReplaceSpecialChar", "true")
  configOptions.put("legacyDiscriminatorBehavior", "false")
  configOptions.put("modelPropertyNaming", "original")
  configOptions.put("nullSafeAdditionalProps", "true")
  configOptions.put("paramNaming", "original")
  configOptions.put("prependFormOrBodyParameters", "true")
  configOptions.put("supportsES6", "true")
  configOptions.put("useSingleRequestParameter", "true")
  configOptions.put("useSquareBracketsInArrayNames", "true")
  configOptions.put("validationAttributes", "true")
  configOptions.put("withInterfaces", "true")
  configOptions.put("withSeparateModelsAndApi", "true")
  // configOptions.put("withoutRuntimeChecks", "true")
  typeMappings.put("date", "string")
  typeMappings.put("DateTime", "string")
  typeMappings.put("Set", "Array")
  typeMappings.put("set", "Array")
  outputs.cacheIf { false }
}

tasks.withType<BuildInfo>().configureEach {
  properties {
    additional.put("openapi.generatorName", "typescript-fetch")
    additional.put("openapi.resourcesDir", resourcesDir.asFile.invariantSeparatorsPath)
    additional.put("openapi.inputSpec", inputFile.asFile.invariantSeparatorsPath)
  }
}
