package com.profiletailors.plugin.utils

import java.io.File
import java.util.*
import kotlin.io.path.createTempDirectory
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/** Gitignore 单元测试类 */
class GitignorePatternsTest : GitignoreGitCommandTest() {

  /** 测试通配符 * 的行为 */
  @Test
  fun testAsteriskPattern() {
    val gitignore =
      """
      *.txt
      *.log
      temp*
      """
        .trimIndent()

    val cases =
      mapOf(
        "a.txt" to true, // *.txt 忽略
        "b.log" to true, // *.log 忽略
        "temp123" to true, // temp* 忽略
      )

    runGitignoreTests(gitignore, cases)
  }

  @Test
  fun testNoSlashPattern() {
    // 任意层级的名为 build 的文件或目录
    val gitignore =
      """
      build
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build" to true, // 忽略
        "app/build" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/" to true, // 忽略
        "app/build/" to true, // 忽略
        "app/build/data.txt" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testSlashAtStartPattern() {
    // 仅根目录下的 build（文件或目录）
    val gitignore =
      """
      /build
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build" to true, // 忽略
        "app/build" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/" to true, // 忽略
        "app/build/" to false, // 不忽略
        "app/build/data.txt" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testSlashAtMiddlePatterns() {
    // 仅根目录下的 build/cache 路径
    val gitignore =
      """
      build/cache
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build/cache" to true, // 忽略
        "app/build/cache" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/cache/" to true, // 忽略
        "build/cache/data.txt" to true, // 忽略
        "build/cache/data/data.txt" to true, // 忽略
        "app/build/cache/" to false, // 不忽略
        "app/build/cache/data.txt" to false, // 不忽略
        "app/build/cache/data/data.txt" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testSlashAtEndPattern() {
    // 任意层级的名为 build 的目录及其内容（不包含同名文件）
    val gitignore =
      """
      build/
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build" to false, // 不忽略
        "app/build" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/" to true, // 忽略
        "app/build/" to true, // 忽略
        "app/build/data.txt" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testStartWithDoubleAsteriskPattern() {
    val gitignore =
      """
      **build
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build" to true, // 忽略
        "app/build" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/" to true, // 忽略
        "app/build/" to true, // 忽略
        "app/build/data.txt" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testEndWithDoubleAsteriskPattern() {
    val gitignore =
      """
      build**
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build" to true, // 忽略
        "app/build" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/" to true, // 忽略
        "build/data.txt" to true, // 忽略
        "app/build/" to true, // 忽略
        "app/build/data.txt" to true, // 忽略
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testEndWithAsteriskPattern() {
    // 相对根目录
    val gitignore =
      """
      build/*
      """
        .trimIndent()

    // 文件
    val cases =
      mapOf(
        "build" to false, // 不忽略
        "app/build" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases)

    // 目录
    val cases1 =
      mapOf(
        "build/" to true, // 忽略
        "build/data.txt" to true, // 忽略
        "build/sub/data.txt" to true, // 忽略
        "app/build/" to false, // 不忽略
        "app/build/data.txt" to false, // 不忽略
        "app/build/sub/data.txt" to false, // 不忽略
      )
    runGitignoreTests(gitignore, cases1)
  }
}

/** @see [PATTERN FORMAT](https://git-scm.com/docs/gitignore#_pattern_format) */
open class GitignoreGitCommandTest {

  @Suppress("detekt:all")
  /**
   * @see <a
   *   href="https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners">GitHub
   *   定义负责存储库中代码的个人或团队</a>
   */
  private val codeOwnerRules =
    listOf(
      "*                @global-owner1 @global-owner2",
      "*.js             @js-owner",
      "*.go             docs@example.com",
      "*.txt            @octo-org/octocats",
      "/build/logs/     @doctocat",
      "docs/*           docs@example.com",
      "apps/            @octocat",
      "/docs/           @doctocat",
      "/scripts/        @doctocat @octocat",
      "**/logs          @octocat",
      "/apps/           @octocat",
      "/apps/github     @doctocat",
    )

  /** 创建临时 Git 仓库并写入 .gitignore */
  private fun setupRepo(gitignoreContent: String): File {
    val repoDir = createTempDirectory("gitignore_" + UUID.randomUUID().toString()).toFile()

    try { // 初始化 Git 仓库
      runGitCommand(repoDir, "git", "init")

      // 设置必要的 Git 配置
      runGitCommand(repoDir, "git", "config", "user.email", "test@example.com")
      runGitCommand(repoDir, "git", "config", "user.name", "Test User")

      // 创建并提交 .gitignore 文件
      File(repoDir, ".gitignore").writeText(gitignoreContent)
      runGitCommand(repoDir, "git", "add", ".gitignore")
      runGitCommand(repoDir, "git", "commit", "-m", "Add gitignore")
    } catch (e: Exception) {
      repoDir.deleteRecursively()
      throw e
    }

    return repoDir
  }

  /** 执行 Git 命令并等待完成 */
  private fun runGitCommand(repoDir: File, vararg command: String) {
    val process = ProcessBuilder(*command).directory(repoDir).redirectErrorStream(true).start()
    if (process.waitFor() != 0) {
      val output = process.inputReader().readText()
      error("Command failed: ${command.joinToString(" ")}\n$output")
    }
  }

  /** 判断路径是否被 Git 忽略 */
  private fun isIgnored(repoDir: File, relativePath: String): Boolean {
    val process =
      ProcessBuilder("git", "check-ignore", "-v", relativePath)
        .directory(repoDir)
        .redirectErrorStream(true)
        .start()
    val output = process.inputStream.bufferedReader().readText()
    val exitCode = process.waitFor()
    if (exitCode == 0) {
      println("🚫 $relativePath\n  ${output.trim()}")
    } else {
      println("✅ $relativePath")
    }
    return exitCode == 0 // 0 表示被忽略，1 表示不被忽略
  }

  /**
   * Gitignore 单元测试类
   *
   * Git 会按行从上到下检查 .gitignore，如果多个规则都能匹配某路径，则 最后一个匹配规则 生效。
   *
   * [Git 规则](https://git-scm.com/docs/gitignore#_pattern_format)：
   * - 开头带 `#` → 注释行
   * - 开头带 `!` → 否定规则（取消忽略）
   * - 开头带 `/` → 从.gitignore 文件所在目录开始匹配
   * - 中间带 `/` → 必须匹配对应的路径层级（不是任意层级）
   * - 结尾带 `/` → 只匹配目录
   * - 不带 `/` → 匹配同名文件或目录，任意层级均可
   * - 通配符
   *     - `*` → 匹配多个字符，不含 `/`
   *     - `**` → 匹配多个字符，包含 `/`
   *     - `?` → 匹配单个字符，不含 `/`
   *     - `[...]` → 匹配任意一个字符，不含 `/`
   */
  protected fun runGitignoreTests(
    gitignoreContent: String,
    testCases: Map<String, Boolean>,
    runTests: Boolean = false,
  ) {
    if (!runTests) return
    println(Ansi.color("============Start Test Rule============", Ansi.Color.GREEN.code))
    println(gitignoreContent)
    println("----------")
    val repoDir = setupRepo(gitignoreContent)

    try {
      for ((relativePath, expected) in testCases) {
        val file = File(repoDir, relativePath)
        if (relativePath.endsWith("/")) {
          file.mkdirs()
        } else {
          file.parentFile?.mkdirs()
          file.createNewFile()
        }
        // 确保文件系统状态稳定
        Thread.sleep(5)

        val result = isIgnored(repoDir, relativePath)
        assertEquals(expected, result, "path: $relativePath")
      }
    } finally {
      println(Ansi.color("============End Test Rule============\n\n", Ansi.Color.GREEN.code))
      // 清理临时目录
      repoDir.deleteRecursively()
    }
  }
}
