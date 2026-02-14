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
 * 生成 Java 启动脚本（Linux bash）
 *
 * ```
 * java [Java 可执行文件] [系统属性/参数 -D...] [JVM 参数 -X...] [特殊参数 -XX...] -jar app.jar [应用参数]
 * ```
 *
 * 用法示例：
 *
 * ```kotlin
 * tasks.register("generateStartScript", GenerateStartScript::class.java) {
 *   // 必填：Java 可执行文件路径
 *   javaCmd.set(File(System.getProperty("java.home"), "bin/java").invariantSeparatorsPath)
 *   // 可选：JVM 参数
 *   jvmOpts.addAll("-Duser.timezone=Asia/Shanghai", "-Xms512m", "-Xmx1024m")
 *   // 必填：可运行 jar 文件名
 *   appJar.set("app.jar")
 *   // 可选：Spring Profile
 *   springProfile.set("prod")
 *   // 可选：传给应用的参数（如 --server.port=8080）
 *   appArgs.addAll("--server.port=8080")
 *   // 可选：输出脚本文件，默认 build/archives/<jarName>.sh
 *   outputFile.set(layout.buildDirectory.file("scripts/start.sh"))
 * }
 * ```
 */
@CacheableTask
abstract class GenerateStartScript : DefaultTask(), Injected {

  /** 必填：Java 可执行文件路径（默认为当前 JDK 的 java） */
  @get:Input
  @get:Option(option = "java-cmd", description = "executable java path")
  val javaCmd =
    objects
      .property<String>()
      .convention(File(System.getProperty("java.home"), "bin/java").invariantSeparatorsPath)

  /** 可选：JVM 参数 */
  @get:Input
  @get:Optional
  val jvmOpts =
    objects
      .listProperty<String>()
      .convention(
        listOf(
          "-Duser.timezone=Asia/Shanghai", // 固定时区
          "-Xms512m",
          "-Xmx1024m", // 堆大小
          "-Xlog:gc*:file=./logs/gc.log:time,uptime,level,tags:filecount=5,filesize=50m", // GC 日志
          "-XX:+HeapDumpOnOutOfMemoryError", // OOM dump
          "-XX:+ExitOnOutOfMemoryError", // OOM 退出
        )
      )

  /** 必填：可运行 jar 文件名 */
  @get:Input
  val appJar = objects.property<String>().convention("app.jar").apply { finalizeValueOnRead() }

  /** 可选：Spring Profile */
  @get:Input @get:Optional val springProfile = objects.property<String>()

  /** 可选：传给应用的参数（如 --server.port=8080） */
  @get:Input @get:Optional val appArgs = objects.listProperty<String>().convention(emptyList())

  /** 可选：输出脚本文件，默认 build/archives/[appJar].sh */
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

  /** 对 Bash 安全转义参数 */
  private fun escapeBash(s: String): String {
    // 使用单引号包裹，单引号内部再处理单引号
    return if (s.isEmpty()) "''" else s.replace("'", "'\"'\"'")
  }
}
