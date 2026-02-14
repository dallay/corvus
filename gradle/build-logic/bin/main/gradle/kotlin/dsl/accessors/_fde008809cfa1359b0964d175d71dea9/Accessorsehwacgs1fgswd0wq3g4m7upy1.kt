
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

package gradle.kotlin.dsl.accessors._fde008809cfa1359b0964d175d71dea9


import org.gradle.api.Action
import org.gradle.api.Incubating
import org.gradle.api.NamedDomainObjectProvider
import org.gradle.api.Project
import org.gradle.api.Task
import org.gradle.api.artifacts.Configuration
import org.gradle.api.artifacts.ConfigurablePublishArtifact
import org.gradle.api.artifacts.ConfigurationContainer
import org.gradle.api.artifacts.Dependency
import org.gradle.api.artifacts.DependencyConstraint
import org.gradle.api.artifacts.ExternalModuleDependency
import org.gradle.api.artifacts.ModuleDependency
import org.gradle.api.artifacts.PublishArtifact
import org.gradle.api.artifacts.dsl.ArtifactHandler
import org.gradle.api.artifacts.dsl.DependencyConstraintHandler
import org.gradle.api.artifacts.dsl.DependencyHandler
import org.gradle.api.initialization.SharedModelDefaults
import org.gradle.api.provider.Provider
import org.gradle.api.provider.ProviderConvertible
import org.gradle.api.tasks.TaskContainer
import org.gradle.api.tasks.TaskProvider

import org.gradle.kotlin.dsl.*
import org.gradle.kotlin.dsl.accessors.runtime.*


/**
 * Retrieves the [javaModuleDependencies][org.gradlex.javamodule.dependencies.JavaModuleDependenciesExtension] extension.
 */
internal
val org.gradle.api.Project.`javaModuleDependencies`: org.gradlex.javamodule.dependencies.JavaModuleDependenciesExtension get() =
    (this as org.gradle.api.plugins.ExtensionAware).extensions.getByName("javaModuleDependencies") as org.gradlex.javamodule.dependencies.JavaModuleDependenciesExtension

/**
 * Configures the [javaModuleDependencies][org.gradlex.javamodule.dependencies.JavaModuleDependenciesExtension] extension.
 */
internal
fun org.gradle.api.Project.`javaModuleDependencies`(configure: Action<org.gradlex.javamodule.dependencies.JavaModuleDependenciesExtension>): Unit =
    (this as org.gradle.api.plugins.ExtensionAware).extensions.configure("javaModuleDependencies", configure)



