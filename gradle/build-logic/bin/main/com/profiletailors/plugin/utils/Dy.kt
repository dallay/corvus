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
 * 分批执行虚拟线程
 *
 * [虚拟线程](https://docs.oracle.com/en/java/javase/21/core/virtual-threads.html)，适合 IO 密集型操作.
 *
 * 最多浪费 size 个虚拟线程，浪费 timeout 时间.
 *
 * @param size 分批大小 默认 20
 * @param timeout 分批超时时间，默认 100 分钟
 * @param consumer 消费者函数
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
