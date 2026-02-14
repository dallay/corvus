package com.profiletailors.plugin.utils

import java.time.Duration
import java.util.concurrent.Callable
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.TimeUnit

class CastConcurrentHashMap<K : Any, V : Any> : ConcurrentHashMap<K, V>() {

  inline fun <reified T : Any> getByType(key: K): T? {
    return get(key) as? T
  }
}

/**
 * Batch execution of virtual threads
 *
 * [Virtual threads](https://docs.oracle.com/en/java/javase/21/core/virtual-threads.html), suitable
 * for IO-intensive operations.
 *
 * Wastes at most size virtual threads, and timeout time.
 *
 * @param size Batch size, default 20
 * @param timeout Batch timeout, default 100 minutes
 * @param consumer Consumer function
 */
fun <T, E> Iterable<T>.chunkedVirtual(
  size: Int = 20,
  timeout: Duration = Duration.ofMinutes(100),
  consumer: (item: T) -> E,
): List<E> {
  val results = ArrayList<E>(this.count())

  this.chunked(size).forEachIndexed { batchIndex, batch ->
    Executors.newVirtualThreadPerTaskExecutor().use { executor ->
      val consumers = batch.map { item -> Callable { consumer(item) } }

      val subTasks = executor.invokeAll(consumers, timeout.toNanos(), TimeUnit.NANOSECONDS)

      subTasks.forEachIndexed { index, f ->
        if (f.state() == Future.State.SUCCESS) {
          results.add(f.resultNow())
        }
        if (f.state() == Future.State.FAILED) {
          throw f.exceptionNow()
        }
        if (f.state() == Future.State.CANCELLED) {
          error("Task - ${(batchIndex * size) + index} cancelled due to timeout")
        }
      }

      // executor.shutdownNow()
    }
  }

  return results
}
