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
    javaParameters = true // 保留参数名
    jvmDefault = JvmDefaultMode.NO_COMPATIBILITY // Java8 接口默认实现
  }
  compilerOptions.freeCompilerArgs.addAll(
    "-Xjsr305=strict", // 严格空安全
    "-Xemit-jvm-type-annotations", // 在泛型上保留类型注解
  )
}
