package com.profiletailors.plugin.utils

import java.io.PrintStream
import java.util.*
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.min
import kotlin.math.pow
import kotlin.system.measureTimeMillis

/** Spinner 风格枚举 */
enum class SpinnerStyle(val frames: List<String>) {
  CLASSIC(listOf("|", "/", "-", "\\", "|", "/", "-", "\\", "|", "/", "-", "\\")),
  PULSE(listOf("██", "▓▓", "▒▒", "░░")),
  EMOJI(listOf("🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘")),
  DOT_CIRCLE(
    listOf("● ○ ○ ○ ○ ○", "○ ● ○ ○ ○ ○", "○ ○ ● ○ ○ ○", "○ ○ ○ ● ○ ○", "○ ○ ○ ○ ● ○", "○ ○ ○ ○ ○ ●")
  ),
  PROGRESS_SIMPLE(
    listOf("[      ]", "[=     ]", "[==    ]", "[===   ]", "[====  ]", "[===== ]", "[======]")
  ),
  PROGRESS_BLOCK(listOf("▱▱▱▱▱▱", "▰▱▱▱▱▱", "▰▰▱▱▱▱", "▰▰▰▱▱▱", "▰▰▰▰▱▱", "▰▰▰▰▰▱", "▰▰▰▰▰▰")),
}

/** LoadingSpinner 工具类 */
object LoadingSpinner {

  private val renderingPaused = AtomicBoolean(false)
  private val spinnerRendered = AtomicBoolean(false)
  private val resetLine by lazy { "\r" + " ".repeat(500) + "\r" }

  /** 暂停 spinner 渲染 */
  private fun pauseRendering() = renderingPaused.set(true)

  /** 恢复 spinner 渲染 */
  private fun resumeRendering() = renderingPaused.set(false)

  /** 自定义 PrintStream，用于拦截输出 */
  private class SpinnerPrintStream(original: PrintStream, private val spinner: LoadingSpinner) :
    PrintStream(original, true) {

    private val lock = Any()

    private fun lockAndRun(block: () -> Unit) {
      synchronized(lock) {
        spinner.pauseRendering()
        if (spinnerRendered.get()) {
          super.write(resetLine.toByteArray(), 0, resetLine.length)
          spinnerRendered.set(false)
        }
        block()
        spinner.resumeRendering()
      }
    }

    override fun write(b: Int) = lockAndRun { super.write(b) }

    override fun write(buf: ByteArray, off: Int, len: Int) = lockAndRun {
      super.write(buf, off, len)
    }
  }

  /**
   * 显示加载动画，同时执行 block 任务
   *
   * @param message 显示消息
   * @param interval 帧间隔（ms）
   * @param style spinner 风格
   * @param showResult 是否在完成后输出最终结果
   * @param block 需要执行的任务
   * @return block 的返回值
   */
  fun <T> run(
    message: String = "",
    interval: Long = 80L,
    style: SpinnerStyle = SpinnerStyle.CLASSIC,
    showResult: Boolean = false,
    block: () -> T,
  ): T {
    val actualInterval = interval.coerceAtLeast(16L)
    val running = AtomicBoolean(true)

    var fakeProgress = 0.0

    val originalOut = System.out
    val originalErr = System.err

    // 替换全局输出
    System.setOut(SpinnerPrintStream(originalOut, this))
    System.setErr(SpinnerPrintStream(originalErr, this))

    val spinnerThread =
      Thread.ofVirtual()
        .unstarted {
          var frameIndex = 0
          val startTime = System.currentTimeMillis()
          while (running.get()) {
            if (!renderingPaused.get()) {
              val frame = style.frames[frameIndex % style.frames.size]
              val elapsedSec = (System.currentTimeMillis() - startTime) / 1000.0
              val formattedTime = "%.2f".format(elapsedSec)
              fakeProgress = min(99.0, fakeProgress + (100.0 - fakeProgress).pow(0.5) * 0.3)

              val line =
                if (message.isNotEmpty()) {
                  "$message $frame ⚡ ${fakeProgress.toInt()}% | ⏱ ${formattedTime}s"
                } else {
                  "$frame ⚡ ${fakeProgress.toInt()}% | ⏱ ${formattedTime}s"
                }

              print("\r$line" + " ".repeat(10) + "\r")
              System.out.flush()
              spinnerRendered.set(true)
            }
            try {
              Thread.sleep(actualInterval)
            } catch (_: InterruptedException) {
              break
            }
            frameIndex++
          }
        }
        .apply { start() }

    val result: T
    val elapsedTime = measureTimeMillis { result = block() }

    running.set(false)
    spinnerThread.join()

    System.setOut(originalOut)
    System.setErr(originalErr)

    if (showResult) {
      println("✓ Completed (${String.format(Locale.ENGLISH, "%.2f", elapsedTime / 1000.0)}s)")
    }

    return result
  }
}
