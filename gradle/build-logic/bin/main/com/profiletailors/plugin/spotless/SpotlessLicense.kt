@file:Suppress("detekt:MaxLineLength")

package com.profiletailors.plugin.spotless

import com.profiletailors.plugin.gradle.eagerSharedCache
import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault
import com.profiletailors.plugin.spotless.SpotlessLicense.License.Apache_2_0
import com.profiletailors.plugin.spotless.SpotlessLicense.License.MIT
import org.gradle.api.Project

/**
 * Spotless license header
 *
 * [manage you license
 * header](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository#disclaimer)
 */
@Suppress("EnumEntryName", "UnstableApiUsage")
object SpotlessLicense {

  enum class License(val value: String) {
    MIT(
      $$"""
      |The MIT License (MIT)
      |
      |Copyright © $YEAR-PRESENT CDY.
      |
      |Permission is hereby granted, free of charge, to any person obtaining a copy
      |of this software and associated documentation files (the “Software”), to deal
      |in the Software without restriction, including without limitation the rights
      |to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
      |copies of the Software, and to permit persons to whom the Software is
      |furnished to do so, subject to the following conditions:
      |
      |The above copyright notice and this permission notice shall be included
      |in all copies or substantial portions of the Software.
      |
      |THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
      |IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
      |FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
      |AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
      |LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
      |OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
      |SOFTWARE.
      """
        .trimMargin()
    ),
    @Suppress("HttpUrlsUsage")
    Apache_2_0(
      $$"""
      |Apache-2.0
      |
      |Copyright $YEAR-PRESENT the original author or authors.
      |
      |Licensed under the Apache License, Version 2.0 (the "License");
      |you may not use this file except in compliance with the License.
      |You may obtain a copy of the License at
      |
      |    http://www.apache.org/licenses/LICENSE-2.0
      |
      |Unless required by applicable law or agreed to in writing, software
      |distributed under the License is distributed on an "AS IS" BASIS,
      |WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
      |See the License for the specific language governing permissions and
      |limitations under the License.
      """
        .trimIndent()
    ),
  }

  private fun getLicense(project: Project): String {
    val name = project.getPropOrDefault(LocalConfig.Props.LICENSE).lowercase()

    return if (name.isNotBlank()) {
      when (name) {
        "NONE".lowercase() -> ""
        "MIT".lowercase() -> MIT.value
        "Apache-2.0".lowercase() -> Apache_2_0.value
        "REPO".lowercase() -> getLicenseFileContent(project, false)
        "ROOT".lowercase() -> getLicenseFileContent(project, true)
        else -> throw IllegalArgumentException("Unknown license: $name")
      }
    } else ""
  }

  private fun getLicenseFileContent(project: Project, fromRoot: Boolean = true): String {
    return project.eagerSharedCache<String>("licenseFileContent") {
      val licenseFile =
        if (fromRoot) {
          project.isolated.rootProject.projectDirectory.file("LICENSE").asFile
        } else {
          project.isolated.projectDirectory.file("LICENSE").asFile
        }
      if (licenseFile.exists()) {
        licenseFile.readText().let {
          if (it.endsWith("\n")) {
            it.dropLast(1)
          } else it
        }
      } else {
        ""
      }
    }
  }

  fun getTxt(project: Project, projectLicense: String = ""): String {
    val license = projectLicense.ifBlank { getLicense(project) }
    if (license.isBlank()) {
      return ""
    }
    return """
      |${license}
      |
      """
      .trimMargin()
  }

  fun getComment(project: Project, projectLicense: String = ""): String {
    val license = projectLicense.ifBlank { getLicense(project) }
    if (license.isBlank()) {
      return ""
    }
    return """
      |/*
      |${license.lines().joinToString("\n") { if (it.isBlank()) " *" else " * $it"}}
      | */
      |
      """
      .trimMargin()
  }

  fun getXml(project: Project, projectLicense: String = ""): String {
    val license = projectLicense.ifBlank { getLicense(project) }
    if (license.isBlank()) {
      return ""
    }
    return """
      |<?xml version="1.0" encoding="UTF-8" ?>
      |<!--
      |${license.lines().joinToString("\n") { if (it.isBlank()) "" else "  $it"}}
      |-->
      |
      """
      .trimMargin()
  }
}
