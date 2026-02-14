@file:Suppress("UnstableApiUsage")

package com.profiletailors.plugin.gradle

import com.profiletailors.plugin.sharedGradle
import java.io.File
import java.util.*
import org.gradle.api.Project
import org.gradle.api.Task
import org.gradle.api.initialization.Settings
import org.gradle.api.plugins.PluginAware
import org.gradle.api.provider.Provider
import org.gradle.api.services.BuildService
import org.gradle.api.services.BuildServiceParameters
import org.jetbrains.kotlin.gradle.internal.ensureParentDirsCreated

/**
 * Computes (registers if absent) a global share cache.
 *
 * @param key The key of the value.
 * @param loader The loader to create the value if not exists.
 * @return The registered value.
 */
inline fun <reified T : Any> PluginAware.eagerSharedCache(
  key: String,
  noinline loader: (() -> T)? = null,
): T {
  val cache = sharedCacheProvider.get().parameters.storage
  val value = if (loader == null) cache[key] else cache.getOrPut(key) { loader() }
  return value as T
}

/**
 * Computes (registers if absent) a global share cache.
 *
 * @param key The key of the value.
 * @param loader The loader to create the value if not exists.
 * @return The registered value provider.
 */
inline fun <reified T : Any> PluginAware.lazySharedCache(
  key: String,
  noinline loader: (() -> T)? = null,
): Provider<T> {
  return sharedCacheProvider.map {
    val cache = it.parameters.storage
    val value = if (loader == null) cache.get(key) else cache.getOrPut(key) { loader() }
    value as T
  }
}

fun Task.useSharedCache(service: Provider<SharedBuildService> = project.sharedCacheProvider) =
  usesService(service)

/**
 * Computes (registers if absent) a global share cache.
 *
 * @return The registered global share cache provider.
 */
val PluginAware.sharedCacheProvider: Provider<SharedBuildService>
  get() = computedSharedBuildService()

/**
 * Computes (registers if absent) a generic SharedBuildService.
 *
 * @param name The name of the service.
 * @return The registered SharedBuildService provider.
 */
fun PluginAware.computedSharedBuildService(
  name: String = "dySharedBuildService"
): Provider<SharedBuildService> {
  return sharedGradle.sharedServices.registerIfAbsent(name, SharedBuildService::class.java) {
    parameters.storage = LinkedHashMap()
  }
}

/** A generic BuildService that stores arbitrary values in a map. */
abstract class SharedBuildService : BuildService<SharedBuildService.Params>, AutoCloseable {

  interface Params : BuildServiceParameters {
    var storage: LinkedHashMap<String, Any>
  }

  override fun close() {
    parameters.storage.clear()
  }
}

/**
 * Computes (registers if absent) a global share cache.
 *
 * @param key The key of the value.
 * @param loader The loader to create the value if not exists.
 * @return The registered value.
 */
fun PluginAware.eagerDiskCache(key: String, loader: (() -> String)? = null): String {
  val cache = diskCacheProvider.get().parameters.storage
  return if (loader == null) cache[key]!! else cache.getOrPut(key) { loader() }
}

/**
 * Computes (registers if absent) a global share cache.
 *
 * @param key The key of the value.
 * @param loader The loader to create the value if not exists.
 * @return The registered value provider.
 */
fun PluginAware.lazyDiskCache(key: String, loader: (() -> String)? = null): Provider<String> {
  val cache = diskCacheProvider.map { it.parameters.storage }
  return cache.map { if (loader == null) it.get(key)!! else it.getOrPut(key) { loader() } }
}

/**
 * Computes (registers if absent) a global share cache.
 *
 * @return The registered global share cache provider.
 */
val PluginAware.diskCacheProvider: Provider<DiskBuildService>
  get() = computedDiskBuildService()

/**
 * Computes (registers if absent) a generic DiskBuildService.
 *
 * @param name The name of the service.
 * @return The registered DiskBuildService provider.
 */
fun PluginAware.computedDiskBuildService(
  name: String = "dyDiskBuildService"
): Provider<DiskBuildService> {
  val plugin = this
  return sharedGradle.sharedServices.registerIfAbsent(name, DiskBuildService::class.java) {
    val cacheFile =
      when (plugin) {
        is Project -> File(plugin.rootDir, ".gradle/.diskCache/cache.properties")
        is Settings -> File(plugin.rootDir, ".gradle/.diskCache/cache.properties")
        else -> null
      }
    val cache = LinkedHashMap<String, String>()
    if (cacheFile != null) {
      cacheFile.ensureParentDirsCreated()
      cache.putAll(DiskCache.loadCache(cacheFile))
      cache["DISK_CACHE"] = cacheFile.absolutePath
    }
    parameters.disk = cache
    parameters.storage = cache
  }
}

/** A generic BuildService that stores string values in a disk cache. */
abstract class DiskBuildService : BuildService<DiskBuildService.Params>, AutoCloseable {

  interface Params : BuildServiceParameters {
    var disk: LinkedHashMap<String, String>
    var storage: LinkedHashMap<String, String>
  }

  override fun close() {
    val diskCache = parameters.disk
    val cache = parameters.storage
    val cacheDir = cache.get("DISK_CACHE")
    if (cacheDir.isNullOrBlank().not()) {
      if (diskCache != cache) {
        DiskCache.saveCache(cache, File(cacheDir))
      }
    }
    diskCache.clear()
    cache.clear()
  }
}

internal object DiskCache {
  fun saveCache(map: Map<String, String>, file: File) {
    val props = Properties()
    map.forEach { (k, v) -> props[k] = v }
    file.parentFile?.mkdirs()
    file.outputStream().use { props.store(it, null) }
  }

  fun loadCache(file: File): Map<String, String> {
    val props = Properties()
    if (file.exists()) {
      file.inputStream().use { props.load(it) }
    }
    return props.entries.associate { (k, v) -> k.toString() to v.toString() }
  }
}
