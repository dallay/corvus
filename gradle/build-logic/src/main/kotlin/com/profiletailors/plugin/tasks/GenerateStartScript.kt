package com.profiletailors.plugin.tasks

import com.profiletailors.plugin.Injected
import java.io.File
import org.gradle.api.DefaultTask
import org.gradle.api.tasks.CacheableTask
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.options.Option
import org.gradle.kotlin.dsl.listProperty
import org.gradle.kotlin.dsl.property

/**
 * Generate Java startup script (Linux bash)
 *
 * ```
 * java [Java executable] [System properties/args -D...] [JVM args -X...] [Special args -XX...] -jar app.jar [App args]
 * ```
 *
 * Usage example:
 * ```kotlin
 * tasks.register("generateStartScript", GenerateStartScript::class.java) {
 *   // Required: Java executable path
 *   javaCmd.set(File(System.getProperty("java.home"), "bin/java").invariantSeparatorsPath)
 *   // Optional: JVM arguments
 *   jvmOpts.addAll("-Duser.timezone=Asia/Shanghai", "-Xms512m", "-Xmx1024m")
 *   // Required: Runnable jar filename
 *   appJar.set("app.jar")
 *   // Optional: Spring Profile
 *   springProfile.set("prod")
 *   // Optional: Arguments passed to the application (e.g. --server.port=8080)
 *   appArgs.addAll("--server.port=8080")
 *   // Optional: Output script file, default build/archives/<jarName>.sh
 *   outputFile.set(layout.buildDirectory.file("scripts/start.sh"))
 * }
 * ```
 */
@CacheableTask
abstract class GenerateStartScript : DefaultTask(), Injected {

  /** Required: Java executable path (defaults to current JDK java) */
  @get:Input
  @get:Option(option = "java-cmd", description = "executable java path")
  val javaCmd =
    objects
      .property<String>()
      .convention(File(System.getProperty("java.home"), "bin/java").invariantSeparatorsPath)

  /** Optional: JVM arguments */
  @get:Input
  @get:Optional
  val jvmOpts =
    objects
      .listProperty<String>()
      .convention(
        listOf(
          "-Duser.timezone=Asia/Shanghai", // Fixed timezone
          "-Xms512m",
          "-Xmx1024m", // Heap size
          "-Xlog:gc*:file=./logs/gc.log:time,uptime,level,tags:filecount=5,filesize=50m", // GC log
          "-XX:+HeapDumpOnOutOfMemoryError", // OOM dump
          "-XX:+ExitOnOutOfMemoryError", // OOM exit
        )
      )

  /** Required: Runnable jar filename */
  @get:Input
  val appJar = objects.property<String>().convention("app.jar").apply { finalizeValueOnRead() }

  /** Optional: Spring Profile */
  @get:Input @get:Optional val springProfile = objects.property<String>()

  /** Optional: Arguments passed to the application (e.g. --server.port=8080) */
  @get:Input @get:Optional val appArgs = objects.listProperty<String>().convention(emptyList())

  /** Optional: Output script file, default build/archives/[appJar].sh */
  @get:OutputFile
  val outputFile =
    objects.fileProperty().convention(layout.buildDirectory.file(appJar.map { "archives/$it.sh" }))

  init {
    group = "other"
    description = "Generate java start script"
  }

  @TaskAction
  fun generate() {
    val out = outputFile.get().asFile
    val newContent = genScript()
    if (!out.exists() || out.readText() != newContent) {
      out.writeText(newContent)
      out.setExecutable(true)
    }
  }

  @Suppress("detekt:LongMethod")
  private fun genScript(): String {
    val jar = appJar.get()
    val jarArgs = appArgs.get().joinToString(" ") { """"${escapeBash(it)}"""" }

    val sysProps =
      listOf("-Dname=${jar}") +
        springProfile.orNull?.let { listOf("-Dspring.profiles.active=$it") }.orEmpty()

    val jvmOptsArray = (sysProps + jvmOpts.get()).joinToString(" ") { """"${escapeBash(it)}"""" }

    return $$"""
      |#!/bin/bash
      |set -Eeuo pipefail
      |
      |JAVA_CMD="$${javaCmd.get()}"
      |JVM_OPTS=($$jvmOptsArray)
      |APP_JAR="$$jar"
      |APP_ARGS=($$jarArgs)
      |
      |APP_HOME=$(cd "$(dirname "$0")" && pwd)
      |PID_FILE="$APP_HOME/$${jar}.pid"
      |LOG_DIR="$APP_HOME/logs"
      |HEAPDUMP_PATH="$LOG_DIR/heapdump.hprof"
      |LOG_PATH="$LOG_DIR/$${jar}.log"
      |
      |mkdir -p "$LOG_DIR"
      |
      |start() {
      |  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
      |    echo "$APP_JAR is already running with PID $(cat "$PID_FILE")."
      |    exit 0
      |  fi
      |  echo "Starting $APP_JAR ..."
      |  echo "Logs will be written to $LOG_PATH"
      |  nohup "$JAVA_CMD" "${JVM_OPTS[@]}" -XX:HeapDumpPath="$HEAPDUMP_PATH" -jar "$APP_HOME/$APP_JAR" "${APP_ARGS[@]}" >> "$LOG_PATH" 2>&1 &
      |  echo "$!" > "$PID_FILE"
      |  echo "Started $APP_JAR with PID $(cat "$PID_FILE")."
      |}
      |
      |stop() {
      |  if [ ! -f "$PID_FILE" ]; then
      |    echo "$APP_JAR is not running (no PID file)."
      |    exit 0
      |  fi
      |
      |  PID=$(cat "$PID_FILE")
      |  if ! kill -0 "$PID" 2>/dev/null; then
      |    echo "Stale PID file found; removing."
      |    rm -f "$PID_FILE"
      |    exit 0
      |  fi
      |
      |  echo "Stopping $APP_JAR (PID=$PID) ..."
      |  kill -TERM "$PID"
      |
      |  for i in {1..30}; do
      |    if kill -0 "$PID" 2>/dev/null; then
      |      sleep 1
      |    else
      |      break
      |    fi
      |  done
      |
      |  if kill -0 "$PID" 2>/dev/null; then
      |    echo "Force killing $APP_JAR ..."
      |    kill -KILL "$PID" || true
      |  fi
      |
      |  rm -f "$PID_FILE"
      |  echo "Stopped."
      |}
      |
      |status() {
      |  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
      |    echo "$APP_JAR is running with PID $(cat "$PID_FILE")."
      |  else
      |    echo "$APP_JAR is not running."
      |  fi
      |}
      |
      |restart() { stop; sleep 2; start; }
      |
      |usage() { echo "Usage: $0 {start|stop|restart|status}"; }
      |
      |case "${1:-}" in
      |  start) start ;;
      |  stop) stop ;;
      |  restart) restart ;;
      |  status) status ;;
      |  *) usage; exit 1 ;;
      |esac
      |
        """
      .trimMargin()
  }

  /** Escape arguments safely for Bash */
  private fun escapeBash(s: String): String {
    // Wrap in single quotes, handle single quotes inside
    return if (s.isEmpty()) "''" else s.replace("'", "'\"'\"'")
  }
}
