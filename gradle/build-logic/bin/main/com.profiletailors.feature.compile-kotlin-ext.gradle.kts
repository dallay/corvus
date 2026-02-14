import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

plugins {
  java
  id("com.profiletailors.check.quality-detekt")
  id("org.jetbrains.kotlin.kapt")
  id("com.google.devtools.ksp")
  //  id("org.jetbrains.kotlin.plugin.lombok")
  id("org.jetbrains.kotlin.plugin.sam.with.receiver")
  id("org.jetbrains.kotlin.plugin.spring")
  id("org.jetbrains.kotlin.plugin.jpa")
}

dependencies {
  runtimeOnly(embeddedKotlin("metadata-jvm"))
  implementation(embeddedKotlin("reflect"))
}

val jepEnablePreview = project.getPropOrDefault(LocalConfig.Props.JEP_ENABLE_PREVIEW).toBoolean()

// kapt
kapt {
  keepJavacAnnotationProcessors = true
  if (jepEnablePreview) {
    javacOptions { option("--enable-preview") }
  }
}

// All-open

// No-arg
// https://www.baeldung.com/kotlin/instantiate-data-class-empty-constructor
noArg {
  invokeInitializers = true // 执行属性初始化
  annotation("lombok.NoArgsConstructor")
  annotation("io.swagger.v3.oas.annotations.media.Schema")
}
