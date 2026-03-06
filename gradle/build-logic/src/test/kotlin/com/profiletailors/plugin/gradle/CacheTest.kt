package com.profiletailors.plugin.gradle

import org.gradle.testfixtures.ProjectBuilder
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class CacheTest {

  private val project = ProjectBuilder.builder().build()

  @Test
  fun projectCachedProviderSupportsScalarCollectionsAndSets() {
    val scalar = project.cachedProvider { "value" }
    val list = project.cachedProvider { listOf("a", "b") }
    val map = project.cachedProvider { mapOf("k" to 1) }
    val set = project.cachedProvider { setOf("x") }

    assertEquals("value", scalar.get())
    assertEquals(listOf("a", "b"), list.get())
    assertEquals(mapOf("k" to 1), map.get())
    assertEquals(setOf("x"), set.get())
  }

  @Test
  fun providerFactoryCachedProviderSupportsScalarCollectionsAndSets() {
    val scalar = project.providers.cachedProvider(project.objects) { "value" }
    val list = project.providers.cachedProvider(project.objects) { listOf("a", "b") }
    val map = project.providers.cachedProvider(project.objects) { mapOf("k" to 1) }
    val set = project.providers.cachedProvider(project.objects) { setOf("x") }

    assertEquals("value", scalar.get())
    assertEquals(listOf("a", "b"), list.get())
    assertEquals(mapOf("k" to 1), map.get())
    assertEquals(setOf("x"), set.get())
  }

  @Test
  fun providerTransformsSupportMapFlatMapAndZip() {
    val base = project.provider { 2 }
    val right = project.provider { 3 }

    val mappedScalar = base.cachedMap(project.objects) { it * 2 }
    val mappedList = base.cachedMap(project.objects) { listOf(it, it + 1) }
    val mappedMap = base.cachedMap(project.objects) { mapOf("v" to it) }
    val mappedSet = base.cachedMap(project.objects) { setOf(it) }

    val flatMappedScalar = base.cachedFlatMap(project.objects) { project.provider { it * 3 } }
    val flatMappedList =
      base.cachedFlatMap(project.objects) { project.provider { listOf(it, it + 2) } }
    val flatMappedMap =
      base.cachedFlatMap(project.objects) { project.provider { mapOf("v" to it) } }
    val flatMappedSet = base.cachedFlatMap(project.objects) { project.provider { setOf(it) } }

    val zippedScalar = base.cachedZip(project.objects, right) { left, r -> left + r }
    val zippedList = base.cachedZip(project.objects, right) { left, r -> listOf(left, r) }
    val zippedMap = base.cachedZip(project.objects, right) { left, r -> mapOf(left to r) }
    val zippedSet = base.cachedZip(project.objects, right) { left, r -> setOf(left + r) }

    assertEquals(4, mappedScalar.get())
    assertEquals(listOf(2, 3), mappedList.get())
    assertEquals(mapOf("v" to 2), mappedMap.get())
    assertEquals(setOf(2), mappedSet.get())

    assertEquals(6, flatMappedScalar.get())
    assertEquals(listOf(2, 4), flatMappedList.get())
    assertEquals(mapOf("v" to 2), flatMappedMap.get())
    assertEquals(setOf(2), flatMappedSet.get())

    assertEquals(5, zippedScalar.get())
    assertEquals(listOf(2, 3), zippedList.get())
    assertEquals(mapOf(2 to 3), zippedMap.get())
    assertEquals(setOf(5), zippedSet.get())
  }

  @Test
  fun nullUncheckedCastsNullableType() {
    val value = NullUnchecked.markAsNullable("ok")

    assertEquals("ok", value)
  }
}
