import org.jetbrains.kotlin.gradle.dsl.JvmDefaultMode
import org.jetbrains.kotlin.gradle.dsl.jvm.JvmTargetValidationMode
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile
import org.jetbrains.kotlin.gradle.tasks.KotlinJvmCompile

plugins {
  java
  id("com.profiletailors.base.lifecycle")
}

tasks.withType<KotlinJvmCompile>().configureEach {
  jvmTargetValidationMode.set(JvmTargetValidationMode.WARNING)
}

tasks.withType<KotlinCompile>().configureEach {
  compilerOptions {
    javaParameters = true // Retain parameter names
    jvmDefault = JvmDefaultMode.NO_COMPATIBILITY // Java 8 interface default implementation
  }
  compilerOptions.freeCompilerArgs.addAll(
    "-Xjsr305=strict", // Strict null safety
    "-Xemit-jvm-type-annotations", // Retain type annotations on generics
  )
}
