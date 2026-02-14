@file:Suppress("detekt:TooManyFunctions")

package com.profiletailors.plugin.gradle

import kotlin.experimental.ExperimentalTypeInference
import kotlin.reflect.full.isSubclassOf
import org.gradle.api.Project
import org.gradle.api.file.Directory
import org.gradle.api.file.RegularFile
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.HasConfigurableValue
import org.gradle.api.provider.Property
import org.gradle.api.provider.Provider
import org.gradle.api.provider.ProviderFactory
import org.gradle.kotlin.dsl.listProperty
import org.gradle.kotlin.dsl.mapProperty
import org.gradle.kotlin.dsl.property
import org.gradle.kotlin.dsl.setProperty

/** ================ project ================ */
@OptIn(ExperimentalTypeInference::class)
@OverloadResolutionByLambdaReturnType
inline fun <reified T : Any> Project.cachedProvider(crossinline block: () -> T?): Provider<out T> =
  provider { block() }.cached(objects)

@JvmName("cachedListProvider")
inline fun <reified T : Any> Project.cachedProvider(
  crossinline block: () -> List<T>?
): Provider<out List<T>> = provider { block() }.cached(objects)

@JvmName("cachedMapProvider")
inline fun <reified K : Any, reified V : Any> Project.cachedProvider(
  crossinline block: () -> Map<K, V>?
): Provider<out Map<K, V>> = provider { block() }.cached(objects)

@JvmName("cachedSetProvider")
inline fun <reified T : Any> Project.cachedProvider(
  crossinline block: () -> Set<T>?
): Provider<out Set<T>> = provider { block() }.cached(objects)

/** ================ providers ================ */
@OptIn(ExperimentalTypeInference::class)
@OverloadResolutionByLambdaReturnType
inline fun <reified T : Any> ProviderFactory.cachedProvider(
  objects: ObjectFactory,
  crossinline block: () -> T?,
): Provider<out T> = provider { block() }.cached(objects)

@JvmName("cachedListProvider")
inline fun <reified T : Any> ProviderFactory.cachedProvider(
  objects: ObjectFactory,
  crossinline block: () -> List<T>?,
): Provider<out List<T>> = provider { block() }.cached(objects)

@JvmName("cachedMapProvider")
inline fun <reified K : Any, reified V : Any> ProviderFactory.cachedProvider(
  objects: ObjectFactory,
  crossinline block: () -> Map<K, V>?,
): Provider<out Map<K, V>> = provider { block() }.cached(objects)

@JvmName("cachedSetProvider")
inline fun <reified T : Any> ProviderFactory.cachedProvider(
  objects: ObjectFactory,
  crossinline block: () -> Set<T>?,
): Provider<out Set<T>> = provider { block() }.cached(objects)

/** ================ provider ================ */
inline fun <reified T : Any> Provider<out T>.cached(objects: ObjectFactory): Provider<out T> =
  @Suppress("UNCHECKED_CAST")
  when {
      T::class.isSubclassOf(RegularFile::class) -> objects.fileProperty() as Property<T>
      T::class.isSubclassOf(Directory::class) -> objects.directoryProperty() as Property<T>
      else -> objects.property()
    }
    .value(this)
    .asImmutable()

@JvmName("cachedList")
inline fun <reified T : Any> Provider<out List<T>>.cached(
  objects: ObjectFactory
): Provider<out List<T>> = objects.listProperty<T>().value(this).asImmutable()

@JvmName("cachedSet")
inline fun <reified T : Any> Provider<Set<T>>.cached(objects: ObjectFactory): Provider<out Set<T>> =
  objects.setProperty<T>().value(this).asImmutable()

@JvmName("cachedMap")
inline fun <reified K : Any, reified V : Any> Provider<out Map<K, V>>.cached(
  objects: ObjectFactory
): Provider<out Map<K, V>> = objects.mapProperty<K, V>().value(this).asImmutable()

fun <T : HasConfigurableValue> T.asImmutable() = apply {
  disallowChanges()
  finalizeValueOnRead()
}

/** ================ provider transform ================ */
@OptIn(ExperimentalTypeInference::class)
@OverloadResolutionByLambdaReturnType
inline fun <T : Any, reified S : Any> Provider<out T>.cachedMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> S?,
): Provider<out S> = map { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@JvmName("cachedMapToList")
inline fun <T : Any, reified S : Any> Provider<out T>.cachedMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> List<S>?,
): Provider<out List<S>> = map { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@JvmName("cachedMapToMap")
inline fun <T : Any, reified K : Any, reified V : Any> Provider<out T>.cachedMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> Map<K, V>?,
): Provider<out Map<K, V>> = map { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@JvmName("cachedMapToSet")
inline fun <T : Any, reified S : Any> Provider<out T>.cachedMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> Set<S>?,
): Provider<out Set<S>> = map { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@OptIn(ExperimentalTypeInference::class)
@OverloadResolutionByLambdaReturnType
inline fun <T : Any, reified S : Any> Provider<out T>.cachedFlatMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> Provider<out S>?,
): Provider<out S> = flatMap { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@JvmName("cachedFlatMapToList")
inline fun <T : Any, reified S : Any> Provider<out T>.cachedFlatMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> Provider<out List<S>>?,
): Provider<out List<S>> = flatMap { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@JvmName("cachedFlatMapToMap")
inline fun <T : Any, reified K : Any, reified V : Any> Provider<out T>.cachedFlatMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> Provider<out Map<K, V>>?,
): Provider<out Map<K, V>> =
  flatMap { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@JvmName("cachedFlatMapToSet")
inline fun <T : Any, reified S : Any> Provider<out T>.cachedFlatMap(
  objects: ObjectFactory,
  crossinline transformer: (T) -> Provider<out Set<S>>?,
): Provider<out Set<S>> = flatMap { NullUnchecked.markAsNullable(transformer(it)) }.cached(objects)

@OptIn(ExperimentalTypeInference::class)
@OverloadResolutionByLambdaReturnType
inline fun <T : Any, U : Any, reified R : Any> Provider<out T>.cachedZip(
  objects: ObjectFactory,
  right: Provider<out U>,
  crossinline combiner: (T, U) -> R?,
): Provider<out R> =
  zip(right) { leftValue, rightValue -> combiner(leftValue, rightValue) }.cached(objects)

@JvmName("cachedZipToList")
inline fun <T : Any, U : Any, reified R : Any> Provider<out T>.cachedZip(
  objects: ObjectFactory,
  right: Provider<out U>,
  crossinline combiner: (T, U) -> List<R>?,
): Provider<out List<R>> =
  zip(right) { leftValue, rightValue -> combiner(leftValue, rightValue) }.cached(objects)

@JvmName("cachedZipToMap")
inline fun <T : Any, U : Any, reified K : Any, reified V : Any> Provider<out T>.cachedZip(
  objects: ObjectFactory,
  right: Provider<out U>,
  crossinline combiner: (T, U) -> Map<K, V>?,
): Provider<out Map<K, V>> =
  zip(right) { leftValue, rightValue -> combiner(leftValue, rightValue) }.cached(objects)

@JvmName("cachedZipToSet")
inline fun <T : Any, U : Any, reified R : Any> Provider<out T>.cachedZip(
  objects: ObjectFactory,
  right: Provider<out U>,
  crossinline combiner: (T, U) -> Set<R>?,
): Provider<out Set<R>> =
  zip(right) { leftValue, rightValue -> combiner(leftValue, rightValue) }.cached(objects)

object NullUnchecked {

  @Suppress("UNCHECKED_CAST") fun <T> markAsNullable(value: T?): T = value as T
}
