package com.profiletailors.app

import java.io.OutputStream
import java.io.PrintStream
import java.util.concurrent.TimeUnit
import org.openjdk.jmh.annotations.Benchmark
import org.openjdk.jmh.annotations.BenchmarkMode
import org.openjdk.jmh.annotations.Fork
import org.openjdk.jmh.annotations.Measurement
import org.openjdk.jmh.annotations.Mode
import org.openjdk.jmh.annotations.OutputTimeUnit
import org.openjdk.jmh.annotations.Scope
import org.openjdk.jmh.annotations.Setup
import org.openjdk.jmh.annotations.State
import org.openjdk.jmh.annotations.Warmup

/**
 * application benchmark
 *
 * @author dy
 */
@BenchmarkMode(Mode.Throughput)
@Fork(1)
@Measurement(iterations = 5)
@OutputTimeUnit(TimeUnit.MILLISECONDS)
@State(Scope.Benchmark)
@Warmup(iterations = 2)
open class ApplicationBenchmark {

  private lateinit var module: MainModule

  @Setup
  fun setup() {
    System.setOut(PrintStream(OutputStream.nullOutputStream()))
    System.setErr(PrintStream(OutputStream.nullOutputStream()))
    module = MainModule()
  }

  @Benchmark
  fun testRun() {
    module.run(arrayOf("benchmark", "test"))
  }
}
