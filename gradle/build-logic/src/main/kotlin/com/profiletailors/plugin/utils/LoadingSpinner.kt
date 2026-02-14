package com.profiletailors.plugin.utils

import java.io.PrintStream
import java.util.*
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.min
import kotlin.math.pow
import kotlin.system.measureTimeMillis

/** Spinner style enum */
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

/** LoadingSpinner utility class */
object LoadingSpinner {

  private val renderingPaused = AtomicBoolean(false)
  private val spinnerRendered = AtomicBoolean(false)
  private val resetLine by lazy { "\r" + " ".repeat(500) + "\r" }

  /** Pause spinner rendering */
  private fun pauseRendering() = renderingPaused.set(true)

  /** Resume spinner rendering */
  private fun resumeRendering() = renderingPaused.set(false)

  /** Custom PrintStream for intercepting output */
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
   * Show loading animation while executing block task
   *
   * @param message Display message
   * @param interval Frame interval (ms)
   * @param style Spinner style
   * @param showResult Whether to output final result after completion
   * @param block Task to execute
   * @return Return value of block
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

    // Replace global output
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
