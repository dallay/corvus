/*
 * Copyright 2018 the original author or authors.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

@file:Suppress(
    "unused",
    "nothing_to_inline",
    "useless_cast",
    "unchecked_cast",
    "extension_shadowed_by_member",
    "redundant_projection",
    "RemoveRedundantBackticks",
    "ObjectPropertyName",
    "deprecation",
    "detekt:all"
)
@file:org.gradle.api.Generated

package gradle.kotlin.dsl.plugins._3bfeff8ec3ef5aa8e7e94d27fe9b0fc1

import org.gradle.plugin.use.PluginDependenciesSpec
import org.gradle.plugin.use.PluginDependencySpec


/**
 * The `com` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com`.
 */
internal
val `PluginDependenciesSpec`.`com`: `ComPluginGroup`
    get() = `ComPluginGroup`(this)


/**
 * The `com.autonomousapps` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComAutonomousappsPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.autonomousapps`.
 */
internal
val `ComPluginGroup`.`autonomousapps`: `ComAutonomousappsPluginGroup`
    get() = `ComAutonomousappsPluginGroup`(plugins)


/**
 * The `com.autonomousapps.build-health` plugin implemented by [com.autonomousapps.BuildHealthPlugin].
 */
internal
val `ComAutonomousappsPluginGroup`.`build-health`: PluginDependencySpec
    get() = plugins.id("com.autonomousapps.build-health")


/**
 * The `com.autonomousapps.dependency-analysis` plugin implemented by [com.autonomousapps.DependencyAnalysisPlugin].
 */
internal
val `ComAutonomousappsPluginGroup`.`dependency-analysis`: PluginDependencySpec
    get() = plugins.id("com.autonomousapps.dependency-analysis")


/**
 * The `com.diffplug` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComDiffplugPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.diffplug`.
 */
internal
val `ComPluginGroup`.`diffplug`: `ComDiffplugPluginGroup`
    get() = `ComDiffplugPluginGroup`(plugins)


/**
 * The `com.diffplug.gradle` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComDiffplugGradlePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.diffplug.gradle`.
 */
internal
val `ComDiffplugPluginGroup`.`gradle`: `ComDiffplugGradlePluginGroup`
    get() = `ComDiffplugGradlePluginGroup`(plugins)


/**
 * The `com.diffplug.gradle.spotless` plugin implemented by [com.diffplug.gradle.spotless.SpotlessPluginRedirect].
 */
internal
val `ComDiffplugGradlePluginGroup`.`spotless`: PluginDependencySpec
    get() = plugins.id("com.diffplug.gradle.spotless")


/**
 * The `com.diffplug.spotless` plugin implemented by [com.diffplug.gradle.spotless.SpotlessPlugin].
 */
internal
val `ComDiffplugPluginGroup`.`spotless`: PluginDependencySpec
    get() = plugins.id("com.diffplug.spotless")


/**
 * The `com.github` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComGithubPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.github`.
 */
internal
val `ComPluginGroup`.`github`: `ComGithubPluginGroup`
    get() = `ComGithubPluginGroup`(plugins)


/**
 * The `com.github.johnrengelman` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComGithubJohnrengelmanPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.github.johnrengelman`.
 */
internal
val `ComGithubPluginGroup`.`johnrengelman`: `ComGithubJohnrengelmanPluginGroup`
    get() = `ComGithubJohnrengelmanPluginGroup`(plugins)


/**
 * The `com.github.johnrengelman.shadow` plugin implemented by [com.github.jengelman.gradle.plugins.shadow.legacy.LegacyShadowPlugin].
 */
internal
val `ComGithubJohnrengelmanPluginGroup`.`shadow`: PluginDependencySpec
    get() = plugins.id("com.github.johnrengelman.shadow")


/**
 * The `com.github.spotbugs` plugin implemented by [com.github.spotbugs.snom.SpotBugsPlugin].
 */
internal
val `ComGithubPluginGroup`.`spotbugs`: PluginDependencySpec
    get() = plugins.id("com.github.spotbugs")


/**
 * The `com.github.spotbugs-base` plugin implemented by [com.github.spotbugs.snom.SpotBugsBasePlugin].
 */
internal
val `ComGithubPluginGroup`.`spotbugs-base`: PluginDependencySpec
    get() = plugins.id("com.github.spotbugs-base")


/**
 * The `com.google` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComGooglePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.google`.
 */
internal
val `ComPluginGroup`.`google`: `ComGooglePluginGroup`
    get() = `ComGooglePluginGroup`(plugins)


/**
 * The `com.google.devtools` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComGoogleDevtoolsPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.google.devtools`.
 */
internal
val `ComGooglePluginGroup`.`devtools`: `ComGoogleDevtoolsPluginGroup`
    get() = `ComGoogleDevtoolsPluginGroup`(plugins)


/**
 * The `com.google.devtools.ksp` plugin implemented by [com.google.devtools.ksp.gradle.KspGradleSubplugin].
 */
internal
val `ComGoogleDevtoolsPluginGroup`.`ksp`: PluginDependencySpec
    get() = plugins.id("com.google.devtools.ksp")


/**
 * The `com.gradle` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComGradlePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.gradle`.
 */
internal
val `ComPluginGroup`.`gradle`: `ComGradlePluginGroup`
    get() = `ComGradlePluginGroup`(plugins)


/**
 * The `com.gradle.develocity` plugin implemented by [com.gradle.develocity.agent.gradle.DevelocityPlugin].
 */
internal
val `ComGradlePluginGroup`.`develocity`: PluginDependencySpec
    get() = plugins.id("com.gradle.develocity")


/**
 * The `com.gradleup` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComGradleupPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.gradleup`.
 */
internal
val `ComPluginGroup`.`gradleup`: `ComGradleupPluginGroup`
    get() = `ComGradleupPluginGroup`(plugins)


/**
 * The `com.gradleup.shadow` plugin implemented by [com.github.jengelman.gradle.plugins.shadow.ShadowPlugin].
 */
internal
val `ComGradleupPluginGroup`.`shadow`: PluginDependencySpec
    get() = plugins.id("com.gradleup.shadow")


/**
 * The `com.vanniktech` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComVanniktechPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.vanniktech`.
 */
internal
val `ComPluginGroup`.`vanniktech`: `ComVanniktechPluginGroup`
    get() = `ComVanniktechPluginGroup`(plugins)


/**
 * The `com.vanniktech.maven` plugin group.
 */
@org.gradle.api.Generated
internal
class `ComVanniktechMavenPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `com.vanniktech.maven`.
 */
internal
val `ComVanniktechPluginGroup`.`maven`: `ComVanniktechMavenPluginGroup`
    get() = `ComVanniktechMavenPluginGroup`(plugins)


/**
 * The `com.vanniktech.maven.publish` plugin implemented by [com.vanniktech.maven.publish.MavenPublishPlugin].
 */
internal
val `ComVanniktechMavenPluginGroup`.`publish`: PluginDependencySpec
    get() = plugins.id("com.vanniktech.maven.publish")


/**
 * The `dev` plugin group.
 */
@org.gradle.api.Generated
internal
class `DevPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `dev`.
 */
internal
val `PluginDependenciesSpec`.`dev`: `DevPluginGroup`
    get() = `DevPluginGroup`(this)


/**
 * The `dev.detekt` plugin implemented by [dev.detekt.gradle.plugin.DetektPlugin].
 */
internal
val `DevPluginGroup`.`detekt`: PluginDependencySpec
    get() = plugins.id("dev.detekt")


/**
 * The `io` plugin group.
 */
@org.gradle.api.Generated
internal
class `IoPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `io`.
 */
internal
val `PluginDependenciesSpec`.`io`: `IoPluginGroup`
    get() = `IoPluginGroup`(this)


/**
 * The `io.freefair` plugin group.
 */
@org.gradle.api.Generated
internal
class `IoFreefairPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `io.freefair`.
 */
internal
val `IoPluginGroup`.`freefair`: `IoFreefairPluginGroup`
    get() = `IoFreefairPluginGroup`(plugins)


/**
 * The `io.freefair.lombok` plugin implemented by [io.freefair.gradle.plugins.lombok.LombokPlugin].
 */
internal
val `IoFreefairPluginGroup`.`lombok`: PluginDependencySpec
    get() = plugins.id("io.freefair.lombok")


/**
 * The `io.fuchs` plugin group.
 */
@org.gradle.api.Generated
internal
class `IoFuchsPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `io.fuchs`.
 */
internal
val `IoPluginGroup`.`fuchs`: `IoFuchsPluginGroup`
    get() = `IoFuchsPluginGroup`(plugins)


/**
 * The `io.fuchs.gradle` plugin group.
 */
@org.gradle.api.Generated
internal
class `IoFuchsGradlePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `io.fuchs.gradle`.
 */
internal
val `IoFuchsPluginGroup`.`gradle`: `IoFuchsGradlePluginGroup`
    get() = `IoFuchsGradlePluginGroup`(plugins)


/**
 * The `io.fuchs.gradle.classpath-collision-detector` plugin implemented by [io.fuchs.gradle.collisiondetector.CollisionDetectorPlugin].
 */
internal
val `IoFuchsGradlePluginGroup`.`classpath-collision-detector`: PluginDependencySpec
    get() = plugins.id("io.fuchs.gradle.classpath-collision-detector")


/**
 * The `kotlin` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinPluginWrapper].
 */
internal
val `PluginDependenciesSpec`.`kotlin`: PluginDependencySpec
    get() = this.id("kotlin")


/**
 * The `kotlin-allopen` plugin implemented by [org.jetbrains.kotlin.allopen.gradle.AllOpenGradleSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-allopen`: PluginDependencySpec
    get() = this.id("kotlin-allopen")


/**
 * The `kotlin-android` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinAndroidPluginWrapper].
 */
internal
val `PluginDependenciesSpec`.`kotlin-android`: PluginDependencySpec
    get() = this.id("kotlin-android")


/**
 * The `kotlin-android-extensions` plugin implemented by [org.jetbrains.kotlin.gradle.internal.AndroidExtensionsSubpluginIndicator].
 */
internal
val `PluginDependenciesSpec`.`kotlin-android-extensions`: PluginDependencySpec
    get() = this.id("kotlin-android-extensions")


/**
 * The `kotlin-jpa` plugin implemented by [org.jetbrains.kotlin.noarg.gradle.KotlinJpaSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-jpa`: PluginDependencySpec
    get() = this.id("kotlin-jpa")


/**
 * The `kotlin-kapt` plugin implemented by [org.jetbrains.kotlin.gradle.internal.Kapt3GradleSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-kapt`: PluginDependencySpec
    get() = this.id("kotlin-kapt")


/**
 * The `kotlin-lombok` plugin implemented by [org.jetbrains.kotlin.lombok.gradle.LombokSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-lombok`: PluginDependencySpec
    get() = this.id("kotlin-lombok")


/**
 * The `kotlin-multiplatform` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinMultiplatformPluginWrapper].
 */
internal
val `PluginDependenciesSpec`.`kotlin-multiplatform`: PluginDependencySpec
    get() = this.id("kotlin-multiplatform")


/**
 * The `kotlin-native-cocoapods` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.cocoapods.KotlinCocoapodsPlugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-native-cocoapods`: PluginDependencySpec
    get() = this.id("kotlin-native-cocoapods")


/**
 * The `kotlin-noarg` plugin implemented by [org.jetbrains.kotlin.noarg.gradle.NoArgGradleSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-noarg`: PluginDependencySpec
    get() = this.id("kotlin-noarg")


/**
 * The `kotlin-parcelize` plugin implemented by [org.jetbrains.kotlin.gradle.internal.ParcelizeSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-parcelize`: PluginDependencySpec
    get() = this.id("kotlin-parcelize")


/**
 * The `kotlin-sam-with-receiver` plugin implemented by [org.jetbrains.kotlin.samWithReceiver.gradle.SamWithReceiverGradleSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-sam-with-receiver`: PluginDependencySpec
    get() = this.id("kotlin-sam-with-receiver")


/**
 * The `kotlin-scripting` plugin implemented by [org.jetbrains.kotlin.gradle.scripting.internal.ScriptingGradleSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-scripting`: PluginDependencySpec
    get() = this.id("kotlin-scripting")


/**
 * The `kotlin-spring` plugin implemented by [org.jetbrains.kotlin.allopen.gradle.SpringGradleSubplugin].
 */
internal
val `PluginDependenciesSpec`.`kotlin-spring`: PluginDependencySpec
    get() = this.id("kotlin-spring")


/**
 * The `me` plugin group.
 */
@org.gradle.api.Generated
internal
class `MePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `me`.
 */
internal
val `PluginDependenciesSpec`.`me`: `MePluginGroup`
    get() = `MePluginGroup`(this)


/**
 * The `me.champeau` plugin group.
 */
@org.gradle.api.Generated
internal
class `MeChampeauPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `me.champeau`.
 */
internal
val `MePluginGroup`.`champeau`: `MeChampeauPluginGroup`
    get() = `MeChampeauPluginGroup`(plugins)


/**
 * The `me.champeau.jmh` plugin implemented by [me.champeau.jmh.JMHPlugin].
 */
internal
val `MeChampeauPluginGroup`.`jmh`: PluginDependencySpec
    get() = plugins.id("me.champeau.jmh")


/**
 * The `net` plugin group.
 */
@org.gradle.api.Generated
internal
class `NetPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `net`.
 */
internal
val `PluginDependenciesSpec`.`net`: `NetPluginGroup`
    get() = `NetPluginGroup`(this)


/**
 * The `net.ltgt` plugin group.
 */
@org.gradle.api.Generated
internal
class `NetLtgtPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `net.ltgt`.
 */
internal
val `NetPluginGroup`.`ltgt`: `NetLtgtPluginGroup`
    get() = `NetLtgtPluginGroup`(plugins)


/**
 * The `net.ltgt.errorprone` plugin implemented by [net.ltgt.gradle.errorprone.ErrorPronePlugin].
 */
internal
val `NetLtgtPluginGroup`.`errorprone`: PluginDependencySpec
    get() = plugins.id("net.ltgt.errorprone")


/**
 * The `net.ltgt.nullaway` plugin implemented by [net.ltgt.gradle.nullaway.NullAwayPlugin].
 */
internal
val `NetLtgtPluginGroup`.`nullaway`: PluginDependencySpec
    get() = plugins.id("net.ltgt.nullaway")


/**
 * The `org` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org`.
 */
internal
val `PluginDependenciesSpec`.`org`: `OrgPluginGroup`
    get() = `OrgPluginGroup`(this)


/**
 * The `org.cyclonedx` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgCyclonedxPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.cyclonedx`.
 */
internal
val `OrgPluginGroup`.`cyclonedx`: `OrgCyclonedxPluginGroup`
    get() = `OrgCyclonedxPluginGroup`(plugins)


/**
 * The `org.cyclonedx.bom` plugin implemented by [org.cyclonedx.gradle.CyclonedxPlugin].
 */
internal
val `OrgCyclonedxPluginGroup`.`bom`: PluginDependencySpec
    get() = plugins.id("org.cyclonedx.bom")


/**
 * The `org.gradle` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgGradlePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.gradle`.
 */
internal
val `OrgPluginGroup`.`gradle`: `OrgGradlePluginGroup`
    get() = `OrgGradlePluginGroup`(plugins)


/**
 * The `org.gradle.antlr` plugin implemented by [org.gradle.api.plugins.antlr.AntlrPlugin].
 */
internal
val `OrgGradlePluginGroup`.`antlr`: PluginDependencySpec
    get() = plugins.id("org.gradle.antlr")


/**
 * The `org.gradle.application` plugin implemented by [org.gradle.api.plugins.ApplicationPlugin].
 */
internal
val `OrgGradlePluginGroup`.`application`: PluginDependencySpec
    get() = plugins.id("org.gradle.application")


/**
 * The `org.gradle.assembler` plugin implemented by [org.gradle.language.assembler.plugins.AssemblerPlugin].
 */
internal
val `OrgGradlePluginGroup`.`assembler`: PluginDependencySpec
    get() = plugins.id("org.gradle.assembler")


/**
 * The `org.gradle.assembler-lang` plugin implemented by [org.gradle.language.assembler.plugins.AssemblerLangPlugin].
 */
internal
val `OrgGradlePluginGroup`.`assembler-lang`: PluginDependencySpec
    get() = plugins.id("org.gradle.assembler-lang")


/**
 * The `org.gradle.base` plugin implemented by [org.gradle.api.plugins.BasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`base`: PluginDependencySpec
    get() = plugins.id("org.gradle.base")


/**
 * The `org.gradle.binary-base` plugin implemented by [org.gradle.platform.base.plugins.BinaryBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`binary-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.binary-base")


/**
 * The `org.gradle.build-dashboard` plugin implemented by [org.gradle.api.reporting.plugins.BuildDashboardPlugin].
 */
internal
val `OrgGradlePluginGroup`.`build-dashboard`: PluginDependencySpec
    get() = plugins.id("org.gradle.build-dashboard")


/**
 * The `org.gradle.build-init` plugin implemented by [org.gradle.buildinit.plugins.BuildInitPlugin].
 */
internal
val `OrgGradlePluginGroup`.`build-init`: PluginDependencySpec
    get() = plugins.id("org.gradle.build-init")


/**
 * The `org.gradle.c` plugin implemented by [org.gradle.language.c.plugins.CPlugin].
 */
internal
val `OrgGradlePluginGroup`.`c`: PluginDependencySpec
    get() = plugins.id("org.gradle.c")


/**
 * The `org.gradle.c-lang` plugin implemented by [org.gradle.language.c.plugins.CLangPlugin].
 */
internal
val `OrgGradlePluginGroup`.`c-lang`: PluginDependencySpec
    get() = plugins.id("org.gradle.c-lang")


/**
 * The `org.gradle.checkstyle` plugin implemented by [org.gradle.api.plugins.quality.CheckstylePlugin].
 */
internal
val `OrgGradlePluginGroup`.`checkstyle`: PluginDependencySpec
    get() = plugins.id("org.gradle.checkstyle")


/**
 * The `org.gradle.clang-compiler` plugin implemented by [org.gradle.nativeplatform.toolchain.plugins.ClangCompilerPlugin].
 */
internal
val `OrgGradlePluginGroup`.`clang-compiler`: PluginDependencySpec
    get() = plugins.id("org.gradle.clang-compiler")


/**
 * The `org.gradle.codenarc` plugin implemented by [org.gradle.api.plugins.quality.CodeNarcPlugin].
 */
internal
val `OrgGradlePluginGroup`.`codenarc`: PluginDependencySpec
    get() = plugins.id("org.gradle.codenarc")


/**
 * The `org.gradle.component-base` plugin implemented by [org.gradle.platform.base.plugins.ComponentBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`component-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.component-base")


/**
 * The `org.gradle.component-model-base` plugin implemented by [org.gradle.language.base.plugins.ComponentModelBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`component-model-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.component-model-base")


/**
 * The `org.gradle.component-reporting-tasks` plugin implemented by [org.gradle.api.plugins.internal.ComponentReportingTasksPlugin].
 */
internal
val `OrgGradlePluginGroup`.`component-reporting-tasks`: PluginDependencySpec
    get() = plugins.id("org.gradle.component-reporting-tasks")


/**
 * The `org.gradle.cpp` plugin implemented by [org.gradle.language.cpp.plugins.CppPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cpp`: PluginDependencySpec
    get() = plugins.id("org.gradle.cpp")


/**
 * The `org.gradle.cpp-application` plugin implemented by [org.gradle.language.cpp.plugins.CppApplicationPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cpp-application`: PluginDependencySpec
    get() = plugins.id("org.gradle.cpp-application")


/**
 * The `org.gradle.cpp-lang` plugin implemented by [org.gradle.language.cpp.plugins.CppLangPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cpp-lang`: PluginDependencySpec
    get() = plugins.id("org.gradle.cpp-lang")


/**
 * The `org.gradle.cpp-library` plugin implemented by [org.gradle.language.cpp.plugins.CppLibraryPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cpp-library`: PluginDependencySpec
    get() = plugins.id("org.gradle.cpp-library")


/**
 * The `org.gradle.cpp-unit-test` plugin implemented by [org.gradle.nativeplatform.test.cpp.plugins.CppUnitTestPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cpp-unit-test`: PluginDependencySpec
    get() = plugins.id("org.gradle.cpp-unit-test")


/**
 * The `org.gradle.cunit` plugin implemented by [org.gradle.nativeplatform.test.cunit.plugins.CUnitConventionPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cunit`: PluginDependencySpec
    get() = plugins.id("org.gradle.cunit")


/**
 * The `org.gradle.cunit-test-suite` plugin implemented by [org.gradle.nativeplatform.test.cunit.plugins.CUnitPlugin].
 */
internal
val `OrgGradlePluginGroup`.`cunit-test-suite`: PluginDependencySpec
    get() = plugins.id("org.gradle.cunit-test-suite")


/**
 * The `org.gradle.distribution` plugin implemented by [org.gradle.api.distribution.plugins.DistributionPlugin].
 */
internal
val `OrgGradlePluginGroup`.`distribution`: PluginDependencySpec
    get() = plugins.id("org.gradle.distribution")


/**
 * The `org.gradle.distribution-base` plugin implemented by [org.gradle.api.distribution.plugins.DistributionBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`distribution-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.distribution-base")


/**
 * The `org.gradle.ear` plugin implemented by [org.gradle.plugins.ear.EarPlugin].
 */
internal
val `OrgGradlePluginGroup`.`ear`: PluginDependencySpec
    get() = plugins.id("org.gradle.ear")


/**
 * The `org.gradle.eclipse` plugin implemented by [org.gradle.plugins.ide.eclipse.EclipsePlugin].
 */
internal
val `OrgGradlePluginGroup`.`eclipse`: PluginDependencySpec
    get() = plugins.id("org.gradle.eclipse")


/**
 * The `org.gradle.eclipse-wtp` plugin implemented by [org.gradle.plugins.ide.eclipse.EclipseWtpPlugin].
 */
internal
val `OrgGradlePluginGroup`.`eclipse-wtp`: PluginDependencySpec
    get() = plugins.id("org.gradle.eclipse-wtp")


/**
 * The `org.gradle.gcc-compiler` plugin implemented by [org.gradle.nativeplatform.toolchain.plugins.GccCompilerPlugin].
 */
internal
val `OrgGradlePluginGroup`.`gcc-compiler`: PluginDependencySpec
    get() = plugins.id("org.gradle.gcc-compiler")


/**
 * The `org.gradle.google-test` plugin implemented by [org.gradle.nativeplatform.test.googletest.plugins.GoogleTestConventionPlugin].
 */
internal
val `OrgGradlePluginGroup`.`google-test`: PluginDependencySpec
    get() = plugins.id("org.gradle.google-test")


/**
 * The `org.gradle.google-test-test-suite` plugin implemented by [org.gradle.nativeplatform.test.googletest.plugins.GoogleTestPlugin].
 */
internal
val `OrgGradlePluginGroup`.`google-test-test-suite`: PluginDependencySpec
    get() = plugins.id("org.gradle.google-test-test-suite")


/**
 * The `org.gradle.groovy` plugin implemented by [org.gradle.api.plugins.GroovyPlugin].
 */
internal
val `OrgGradlePluginGroup`.`groovy`: PluginDependencySpec
    get() = plugins.id("org.gradle.groovy")


/**
 * The `org.gradle.groovy-base` plugin implemented by [org.gradle.api.plugins.GroovyBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`groovy-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.groovy-base")


/**
 * The `org.gradle.groovy-gradle-plugin` plugin implemented by [org.gradle.plugin.devel.internal.precompiled.PrecompiledGroovyPluginsPlugin].
 */
internal
val `OrgGradlePluginGroup`.`groovy-gradle-plugin`: PluginDependencySpec
    get() = plugins.id("org.gradle.groovy-gradle-plugin")


/**
 * The `org.gradle.help-tasks` plugin implemented by [org.gradle.api.plugins.HelpTasksPlugin].
 */
internal
val `OrgGradlePluginGroup`.`help-tasks`: PluginDependencySpec
    get() = plugins.id("org.gradle.help-tasks")


/**
 * The `org.gradle.idea` plugin implemented by [org.gradle.plugins.ide.idea.IdeaPlugin].
 */
internal
val `OrgGradlePluginGroup`.`idea`: PluginDependencySpec
    get() = plugins.id("org.gradle.idea")


/**
 * The `org.gradle.ivy-publish` plugin implemented by [org.gradle.api.publish.ivy.plugins.IvyPublishPlugin].
 */
internal
val `OrgGradlePluginGroup`.`ivy-publish`: PluginDependencySpec
    get() = plugins.id("org.gradle.ivy-publish")


/**
 * The `org.gradle.jacoco` plugin implemented by [org.gradle.testing.jacoco.plugins.JacocoPlugin].
 */
internal
val `OrgGradlePluginGroup`.`jacoco`: PluginDependencySpec
    get() = plugins.id("org.gradle.jacoco")


/**
 * The `org.gradle.jacoco-report-aggregation` plugin implemented by [org.gradle.testing.jacoco.plugins.JacocoReportAggregationPlugin].
 */
internal
val `OrgGradlePluginGroup`.`jacoco-report-aggregation`: PluginDependencySpec
    get() = plugins.id("org.gradle.jacoco-report-aggregation")


/**
 * The `org.gradle.java` plugin implemented by [org.gradle.api.plugins.JavaPlugin].
 */
internal
val `OrgGradlePluginGroup`.`java`: PluginDependencySpec
    get() = plugins.id("org.gradle.java")


/**
 * The `org.gradle.java-base` plugin implemented by [org.gradle.api.plugins.JavaBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`java-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.java-base")


/**
 * The `org.gradle.java-gradle-plugin` plugin implemented by [org.gradle.plugin.devel.plugins.JavaGradlePluginPlugin].
 */
internal
val `OrgGradlePluginGroup`.`java-gradle-plugin`: PluginDependencySpec
    get() = plugins.id("org.gradle.java-gradle-plugin")


/**
 * The `org.gradle.java-library` plugin implemented by [org.gradle.api.plugins.JavaLibraryPlugin].
 */
internal
val `OrgGradlePluginGroup`.`java-library`: PluginDependencySpec
    get() = plugins.id("org.gradle.java-library")


/**
 * The `org.gradle.java-library-distribution` plugin implemented by [org.gradle.api.plugins.JavaLibraryDistributionPlugin].
 */
internal
val `OrgGradlePluginGroup`.`java-library-distribution`: PluginDependencySpec
    get() = plugins.id("org.gradle.java-library-distribution")


/**
 * The `org.gradle.java-platform` plugin implemented by [org.gradle.api.plugins.JavaPlatformPlugin].
 */
internal
val `OrgGradlePluginGroup`.`java-platform`: PluginDependencySpec
    get() = plugins.id("org.gradle.java-platform")


/**
 * The `org.gradle.java-test-fixtures` plugin implemented by [org.gradle.api.plugins.JavaTestFixturesPlugin].
 */
internal
val `OrgGradlePluginGroup`.`java-test-fixtures`: PluginDependencySpec
    get() = plugins.id("org.gradle.java-test-fixtures")


/**
 * The `org.gradle.jvm-ecosystem` plugin implemented by [org.gradle.api.plugins.JvmEcosystemPlugin].
 */
internal
val `OrgGradlePluginGroup`.`jvm-ecosystem`: PluginDependencySpec
    get() = plugins.id("org.gradle.jvm-ecosystem")


/**
 * The `org.gradle.jvm-test-suite` plugin implemented by [org.gradle.api.plugins.JvmTestSuitePlugin].
 */
internal
val `OrgGradlePluginGroup`.`jvm-test-suite`: PluginDependencySpec
    get() = plugins.id("org.gradle.jvm-test-suite")


/**
 * The `org.gradle.jvm-toolchain-management` plugin implemented by [org.gradle.api.plugins.JvmToolchainManagementPlugin].
 */
internal
val `OrgGradlePluginGroup`.`jvm-toolchain-management`: PluginDependencySpec
    get() = plugins.id("org.gradle.jvm-toolchain-management")


/**
 * The `org.gradle.jvm-toolchains` plugin implemented by [org.gradle.api.plugins.JvmToolchainsPlugin].
 */
internal
val `OrgGradlePluginGroup`.`jvm-toolchains`: PluginDependencySpec
    get() = plugins.id("org.gradle.jvm-toolchains")


/**
 * The `org.gradle.language-base` plugin implemented by [org.gradle.language.base.plugins.LanguageBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`language-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.language-base")


/**
 * The `org.gradle.lifecycle-base` plugin implemented by [org.gradle.language.base.plugins.LifecycleBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`lifecycle-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.lifecycle-base")


/**
 * The `org.gradle.maven-publish` plugin implemented by [org.gradle.api.publish.maven.plugins.MavenPublishPlugin].
 */
internal
val `OrgGradlePluginGroup`.`maven-publish`: PluginDependencySpec
    get() = plugins.id("org.gradle.maven-publish")


/**
 * The `org.gradle.microsoft-visual-cpp-compiler` plugin implemented by [org.gradle.nativeplatform.toolchain.plugins.MicrosoftVisualCppCompilerPlugin].
 */
internal
val `OrgGradlePluginGroup`.`microsoft-visual-cpp-compiler`: PluginDependencySpec
    get() = plugins.id("org.gradle.microsoft-visual-cpp-compiler")


/**
 * The `org.gradle.model-reporting-tasks` plugin implemented by [org.gradle.api.plugins.internal.ModelReportingTasksPlugin].
 */
internal
val `OrgGradlePluginGroup`.`model-reporting-tasks`: PluginDependencySpec
    get() = plugins.id("org.gradle.model-reporting-tasks")


/**
 * The `org.gradle.native-component` plugin implemented by [org.gradle.nativeplatform.plugins.NativeComponentPlugin].
 */
internal
val `OrgGradlePluginGroup`.`native-component`: PluginDependencySpec
    get() = plugins.id("org.gradle.native-component")


/**
 * The `org.gradle.native-component-model` plugin implemented by [org.gradle.nativeplatform.plugins.NativeComponentModelPlugin].
 */
internal
val `OrgGradlePluginGroup`.`native-component-model`: PluginDependencySpec
    get() = plugins.id("org.gradle.native-component-model")


/**
 * The `org.gradle.objective-c` plugin implemented by [org.gradle.language.objectivec.plugins.ObjectiveCPlugin].
 */
internal
val `OrgGradlePluginGroup`.`objective-c`: PluginDependencySpec
    get() = plugins.id("org.gradle.objective-c")


/**
 * The `org.gradle.objective-c-lang` plugin implemented by [org.gradle.language.objectivec.plugins.ObjectiveCLangPlugin].
 */
internal
val `OrgGradlePluginGroup`.`objective-c-lang`: PluginDependencySpec
    get() = plugins.id("org.gradle.objective-c-lang")


/**
 * The `org.gradle.objective-cpp` plugin implemented by [org.gradle.language.objectivecpp.plugins.ObjectiveCppPlugin].
 */
internal
val `OrgGradlePluginGroup`.`objective-cpp`: PluginDependencySpec
    get() = plugins.id("org.gradle.objective-cpp")


/**
 * The `org.gradle.objective-cpp-lang` plugin implemented by [org.gradle.language.objectivecpp.plugins.ObjectiveCppLangPlugin].
 */
internal
val `OrgGradlePluginGroup`.`objective-cpp-lang`: PluginDependencySpec
    get() = plugins.id("org.gradle.objective-cpp-lang")


/**
 * The `org.gradle.pmd` plugin implemented by [org.gradle.api.plugins.quality.PmdPlugin].
 */
internal
val `OrgGradlePluginGroup`.`pmd`: PluginDependencySpec
    get() = plugins.id("org.gradle.pmd")


/**
 * The `org.gradle.project-report` plugin implemented by [org.gradle.api.plugins.ProjectReportsPlugin].
 */
internal
val `OrgGradlePluginGroup`.`project-report`: PluginDependencySpec
    get() = plugins.id("org.gradle.project-report")


/**
 * The `org.gradle.project-reports` plugin implemented by [org.gradle.api.plugins.ProjectReportsPlugin].
 */
internal
val `OrgGradlePluginGroup`.`project-reports`: PluginDependencySpec
    get() = plugins.id("org.gradle.project-reports")


/**
 * The `org.gradle.publishing` plugin implemented by [org.gradle.api.publish.plugins.PublishingPlugin].
 */
internal
val `OrgGradlePluginGroup`.`publishing`: PluginDependencySpec
    get() = plugins.id("org.gradle.publishing")


/**
 * The `org.gradle.reporting-base` plugin implemented by [org.gradle.api.plugins.ReportingBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`reporting-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.reporting-base")


/**
 * The `org.gradle.scala` plugin implemented by [org.gradle.api.plugins.scala.ScalaPlugin].
 */
internal
val `OrgGradlePluginGroup`.`scala`: PluginDependencySpec
    get() = plugins.id("org.gradle.scala")


/**
 * The `org.gradle.scala-base` plugin implemented by [org.gradle.api.plugins.scala.ScalaBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`scala-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.scala-base")


/**
 * The `org.gradle.signing` plugin implemented by [org.gradle.plugins.signing.SigningPlugin].
 */
internal
val `OrgGradlePluginGroup`.`signing`: PluginDependencySpec
    get() = plugins.id("org.gradle.signing")


/**
 * The `org.gradle.software-reporting-tasks` plugin implemented by [org.gradle.api.plugins.SoftwareReportingTasksPlugin].
 */
internal
val `OrgGradlePluginGroup`.`software-reporting-tasks`: PluginDependencySpec
    get() = plugins.id("org.gradle.software-reporting-tasks")


/**
 * The `org.gradle.standard-tool-chains` plugin implemented by [org.gradle.nativeplatform.toolchain.internal.plugins.StandardToolChainsPlugin].
 */
internal
val `OrgGradlePluginGroup`.`standard-tool-chains`: PluginDependencySpec
    get() = plugins.id("org.gradle.standard-tool-chains")


/**
 * The `org.gradle.swift-application` plugin implemented by [org.gradle.language.swift.plugins.SwiftApplicationPlugin].
 */
internal
val `OrgGradlePluginGroup`.`swift-application`: PluginDependencySpec
    get() = plugins.id("org.gradle.swift-application")


/**
 * The `org.gradle.swift-library` plugin implemented by [org.gradle.language.swift.plugins.SwiftLibraryPlugin].
 */
internal
val `OrgGradlePluginGroup`.`swift-library`: PluginDependencySpec
    get() = plugins.id("org.gradle.swift-library")


/**
 * The `org.gradle.swiftpm-export` plugin implemented by [org.gradle.swiftpm.plugins.SwiftPackageManagerExportPlugin].
 */
internal
val `OrgGradlePluginGroup`.`swiftpm-export`: PluginDependencySpec
    get() = plugins.id("org.gradle.swiftpm-export")


/**
 * The `org.gradle.test-report-aggregation` plugin implemented by [org.gradle.api.plugins.TestReportAggregationPlugin].
 */
internal
val `OrgGradlePluginGroup`.`test-report-aggregation`: PluginDependencySpec
    get() = plugins.id("org.gradle.test-report-aggregation")


/**
 * The `org.gradle.test-suite-base` plugin implemented by [org.gradle.testing.base.plugins.TestSuiteBasePlugin].
 */
internal
val `OrgGradlePluginGroup`.`test-suite-base`: PluginDependencySpec
    get() = plugins.id("org.gradle.test-suite-base")


/**
 * The `org.gradle.version-catalog` plugin implemented by [org.gradle.api.plugins.catalog.VersionCatalogPlugin].
 */
internal
val `OrgGradlePluginGroup`.`version-catalog`: PluginDependencySpec
    get() = plugins.id("org.gradle.version-catalog")


/**
 * The `org.gradle.visual-studio` plugin implemented by [org.gradle.ide.visualstudio.plugins.VisualStudioPlugin].
 */
internal
val `OrgGradlePluginGroup`.`visual-studio`: PluginDependencySpec
    get() = plugins.id("org.gradle.visual-studio")


/**
 * The `org.gradle.war` plugin implemented by [org.gradle.api.plugins.WarPlugin].
 */
internal
val `OrgGradlePluginGroup`.`war`: PluginDependencySpec
    get() = plugins.id("org.gradle.war")


/**
 * The `org.gradle.windows-resource-script` plugin implemented by [org.gradle.language.rc.plugins.WindowsResourceScriptPlugin].
 */
internal
val `OrgGradlePluginGroup`.`windows-resource-script`: PluginDependencySpec
    get() = plugins.id("org.gradle.windows-resource-script")


/**
 * The `org.gradle.windows-resources` plugin implemented by [org.gradle.language.rc.plugins.WindowsResourcesPlugin].
 */
internal
val `OrgGradlePluginGroup`.`windows-resources`: PluginDependencySpec
    get() = plugins.id("org.gradle.windows-resources")


/**
 * The `org.gradle.wrapper` plugin implemented by [org.gradle.buildinit.plugins.WrapperPlugin].
 */
internal
val `OrgGradlePluginGroup`.`wrapper`: PluginDependencySpec
    get() = plugins.id("org.gradle.wrapper")


/**
 * The `org.gradle.xcode` plugin implemented by [org.gradle.ide.xcode.plugins.XcodePlugin].
 */
internal
val `OrgGradlePluginGroup`.`xcode`: PluginDependencySpec
    get() = plugins.id("org.gradle.xcode")


/**
 * The `org.gradle.xctest` plugin implemented by [org.gradle.nativeplatform.test.xctest.plugins.XCTestConventionPlugin].
 */
internal
val `OrgGradlePluginGroup`.`xctest`: PluginDependencySpec
    get() = plugins.id("org.gradle.xctest")


/**
 * The `org.gradlex` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgGradlexPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.gradlex`.
 */
internal
val `OrgPluginGroup`.`gradlex`: `OrgGradlexPluginGroup`
    get() = `OrgGradlexPluginGroup`(plugins)


/**
 * The `org.gradlex.extra-java-module-info` plugin implemented by [org.gradlex.javamodule.moduleinfo.ExtraJavaModuleInfoPlugin].
 */
internal
val `OrgGradlexPluginGroup`.`extra-java-module-info`: PluginDependencySpec
    get() = plugins.id("org.gradlex.extra-java-module-info")


/**
 * The `org.gradlex.java-module-dependencies` plugin implemented by [org.gradlex.javamodule.dependencies.JavaModuleDependenciesPlugin].
 */
internal
val `OrgGradlexPluginGroup`.`java-module-dependencies`: PluginDependencySpec
    get() = plugins.id("org.gradlex.java-module-dependencies")


/**
 * The `org.gradlex.java-module-testing` plugin implemented by [org.gradlex.javamodule.testing.JavaModuleTestingPlugin].
 */
internal
val `OrgGradlexPluginGroup`.`java-module-testing`: PluginDependencySpec
    get() = plugins.id("org.gradlex.java-module-testing")


/**
 * The `org.gradlex.java-module-versions` plugin implemented by [org.gradlex.javamodule.dependencies.JavaModuleVersionsPlugin].
 */
internal
val `OrgGradlexPluginGroup`.`java-module-versions`: PluginDependencySpec
    get() = plugins.id("org.gradlex.java-module-versions")


/**
 * The `org.gradlex.jvm-dependency-conflict-detection` plugin implemented by [org.gradlex.jvm.dependency.conflict.detection.JvmDependencyConflictDetectionPlugin].
 */
internal
val `OrgGradlexPluginGroup`.`jvm-dependency-conflict-detection`: PluginDependencySpec
    get() = plugins.id("org.gradlex.jvm-dependency-conflict-detection")


/**
 * The `org.gradlex.jvm-dependency-conflict-resolution` plugin implemented by [org.gradlex.jvm.dependency.conflict.resolution.JvmDependencyConflictResolutionPlugin].
 */
internal
val `OrgGradlexPluginGroup`.`jvm-dependency-conflict-resolution`: PluginDependencySpec
    get() = plugins.id("org.gradlex.jvm-dependency-conflict-resolution")


/**
 * The `org.jetbrains` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains`.
 */
internal
val `OrgPluginGroup`.`jetbrains`: `OrgJetbrainsPluginGroup`
    get() = `OrgJetbrainsPluginGroup`(plugins)


/**
 * The `org.jetbrains.dokka` plugin implemented by [org.jetbrains.dokka.gradle.DokkaPlugin].
 */
internal
val `OrgJetbrainsPluginGroup`.`dokka`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.dokka")


/**
 * The `org.jetbrains.dokka-javadoc` plugin implemented by [org.jetbrains.dokka.gradle.formats.DokkaJavadocPlugin].
 */
internal
val `OrgJetbrainsPluginGroup`.`dokka-javadoc`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.dokka-javadoc")


/**
 * The `org.jetbrains.kotlin` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsKotlinPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains.kotlin`.
 */
internal
val `OrgJetbrainsPluginGroup`.`kotlin`: `OrgJetbrainsKotlinPluginGroup`
    get() = `OrgJetbrainsKotlinPluginGroup`(plugins)


/**
 * The `org.jetbrains.kotlin.android` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinAndroidPluginWrapper].
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`android`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.android")


/**
 * The `org.jetbrains.kotlin.fus-statistics-gradle-plugin` plugin implemented by [org.jetbrains.kotlin.gradle.fus.FusStatisticsPlugin].
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`fus-statistics-gradle-plugin`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.fus-statistics-gradle-plugin")


/**
 * The `org.jetbrains.kotlin.js` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinJsPluginWrapper].
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`js`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.js")


/**
 * The `org.jetbrains.kotlin.jvm` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinPluginWrapper].
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`jvm`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.jvm")


/**
 * The `org.jetbrains.kotlin.kapt` plugin implemented by [org.jetbrains.kotlin.gradle.internal.Kapt3GradleSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`kapt`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.kapt")


/**
 * The `org.jetbrains.kotlin.multiplatform` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.KotlinMultiplatformPluginWrapper].
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`multiplatform`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.multiplatform")


/**
 * The `org.jetbrains.kotlin.native` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsKotlinNativePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains.kotlin.native`.
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`native`: `OrgJetbrainsKotlinNativePluginGroup`
    get() = `OrgJetbrainsKotlinNativePluginGroup`(plugins)


/**
 * The `org.jetbrains.kotlin.native.cocoapods` plugin implemented by [org.jetbrains.kotlin.gradle.plugin.cocoapods.KotlinCocoapodsPlugin].
 */
internal
val `OrgJetbrainsKotlinNativePluginGroup`.`cocoapods`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.native.cocoapods")


/**
 * The `org.jetbrains.kotlin.plugin` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsKotlinPluginPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains.kotlin.plugin`.
 */
internal
val `OrgJetbrainsKotlinPluginGroup`.`plugin`: `OrgJetbrainsKotlinPluginPluginGroup`
    get() = `OrgJetbrainsKotlinPluginPluginGroup`(plugins)


/**
 * The `org.jetbrains.kotlin.plugin.allopen` plugin implemented by [org.jetbrains.kotlin.allopen.gradle.AllOpenGradleSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`allopen`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.allopen")


/**
 * The `org.jetbrains.kotlin.plugin.jpa` plugin implemented by [org.jetbrains.kotlin.noarg.gradle.KotlinJpaSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`jpa`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.jpa")


/**
 * The `org.jetbrains.kotlin.plugin.lombok` plugin implemented by [org.jetbrains.kotlin.lombok.gradle.LombokSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`lombok`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.lombok")


/**
 * The `org.jetbrains.kotlin.plugin.noarg` plugin implemented by [org.jetbrains.kotlin.noarg.gradle.NoArgGradleSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`noarg`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.noarg")


/**
 * The `org.jetbrains.kotlin.plugin.parcelize` plugin implemented by [org.jetbrains.kotlin.gradle.internal.ParcelizeSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`parcelize`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.parcelize")


/**
 * The `org.jetbrains.kotlin.plugin.sam` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsKotlinPluginSamPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains.kotlin.plugin.sam`.
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`sam`: `OrgJetbrainsKotlinPluginSamPluginGroup`
    get() = `OrgJetbrainsKotlinPluginSamPluginGroup`(plugins)


/**
 * The `org.jetbrains.kotlin.plugin.sam.with` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsKotlinPluginSamWithPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains.kotlin.plugin.sam.with`.
 */
internal
val `OrgJetbrainsKotlinPluginSamPluginGroup`.`with`: `OrgJetbrainsKotlinPluginSamWithPluginGroup`
    get() = `OrgJetbrainsKotlinPluginSamWithPluginGroup`(plugins)


/**
 * The `org.jetbrains.kotlin.plugin.sam.with.receiver` plugin implemented by [org.jetbrains.kotlin.samWithReceiver.gradle.SamWithReceiverGradleSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginSamWithPluginGroup`.`receiver`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.sam.with.receiver")


/**
 * The `org.jetbrains.kotlin.plugin.scripting` plugin implemented by [org.jetbrains.kotlin.gradle.scripting.internal.ScriptingGradleSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`scripting`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.scripting")


/**
 * The `org.jetbrains.kotlin.plugin.spring` plugin implemented by [org.jetbrains.kotlin.allopen.gradle.SpringGradleSubplugin].
 */
internal
val `OrgJetbrainsKotlinPluginPluginGroup`.`spring`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlin.plugin.spring")


/**
 * The `org.jetbrains.kotlinx` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgJetbrainsKotlinxPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.jetbrains.kotlinx`.
 */
internal
val `OrgJetbrainsPluginGroup`.`kotlinx`: `OrgJetbrainsKotlinxPluginGroup`
    get() = `OrgJetbrainsKotlinxPluginGroup`(plugins)


/**
 * The `org.jetbrains.kotlinx.kover` plugin implemented by [kotlinx.kover.gradle.plugin.KoverGradlePlugin].
 */
internal
val `OrgJetbrainsKotlinxPluginGroup`.`kover`: PluginDependencySpec
    get() = plugins.id("org.jetbrains.kotlinx.kover")


/**
 * The `org.openapi` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgOpenapiPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.openapi`.
 */
internal
val `OrgPluginGroup`.`openapi`: `OrgOpenapiPluginGroup`
    get() = `OrgOpenapiPluginGroup`(plugins)


/**
 * The `org.openapi.generator` plugin implemented by [org.openapitools.generator.gradle.plugin.OpenApiGeneratorPlugin].
 */
internal
val `OrgOpenapiPluginGroup`.`generator`: PluginDependencySpec
    get() = plugins.id("org.openapi.generator")


/**
 * The `org.openrewrite` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgOpenrewritePluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.openrewrite`.
 */
internal
val `OrgPluginGroup`.`openrewrite`: `OrgOpenrewritePluginGroup`
    get() = `OrgOpenrewritePluginGroup`(plugins)


/**
 * The `org.openrewrite.rewrite` plugin implemented by [org.openrewrite.gradle.RewritePlugin].
 */
internal
val `OrgOpenrewritePluginGroup`.`rewrite`: PluginDependencySpec
    get() = plugins.id("org.openrewrite.rewrite")


/**
 * The `org.springframework` plugin group.
 */
@org.gradle.api.Generated
internal
class `OrgSpringframeworkPluginGroup`(internal val plugins: PluginDependenciesSpec)


/**
 * Plugin ids starting with `org.springframework`.
 */
internal
val `OrgPluginGroup`.`springframework`: `OrgSpringframeworkPluginGroup`
    get() = `OrgSpringframeworkPluginGroup`(plugins)


/**
 * The `org.springframework.boot` plugin implemented by [org.springframework.boot.gradle.plugin.SpringBootPlugin].
 */
internal
val `OrgSpringframeworkPluginGroup`.`boot`: PluginDependencySpec
    get() = plugins.id("org.springframework.boot")
