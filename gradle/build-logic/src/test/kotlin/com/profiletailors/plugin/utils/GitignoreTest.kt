package com.profiletailors.plugin.utils

import java.io.File
import java.util.*
import kotlin.io.path.createTempDirectory
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/** Gitignore unit test class */
class GitignorePatternsTest : GitignoreGitCommandTest() {

  /** Test asterisk wildcard behavior */
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
        "a.txt" to true, // *.txt ignored
        "b.log" to true, // *.log ignored
        "temp123" to true, // temp* ignored
      )

    runGitignoreTests(gitignore, cases)
  }

  @Test
  fun testNoSlashPattern() {
    // File or directory named build at any level
    val gitignore =
      """
      build
      """
        .trimIndent()

    // Files
    val cases =
      mapOf(
        "build" to true, // ignored
        "app/build" to true, // ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/" to true, // ignored
        "app/build/" to true, // ignored
        "app/build/data.txt" to true, // ignored
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testSlashAtStartPattern() {
    // build (file or directory) only under the root directory
    val gitignore =
      """
      /build
      """
        .trimIndent()

    // Files
    val cases =
      mapOf(
        "build" to true, // ignored
        "app/build" to false, // not ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/" to true, // ignored
        "app/build/" to false, // not ignored
        "app/build/data.txt" to false, // not ignored
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testSlashAtMiddlePatterns() {
    // build/cache path only under the root directory
    val gitignore =
      """
      build/cache
      """
        .trimIndent()

    // Files
    val cases =
      mapOf(
        "build/cache" to true, // ignored
        "app/build/cache" to false, // not ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/cache/" to true, // ignored
        "build/cache/data.txt" to true, // ignored
        "build/cache/data/data.txt" to true, // ignored
        "app/build/cache/" to false, // not ignored
        "app/build/cache/data.txt" to false, // not ignored
        "app/build/cache/data/data.txt" to false, // not ignored
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testSlashAtEndPattern() {
    // Directory named build at any level and its content (excluding files with the same name)
    val gitignore =
      """
      build/
      """
        .trimIndent()

    // Files
    val cases =
      mapOf(
        "build" to false, // not ignored
        "app/build" to false, // not ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/" to true, // ignored
        "app/build/" to true, // ignored
        "app/build/data.txt" to true, // ignored
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

    // Files
    val cases =
      mapOf(
        "build" to true, // ignored
        "app/build" to true, // ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/" to true, // ignored
        "app/build/" to true, // ignored
        "app/build/data.txt" to true, // ignored
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

    // Files
    val cases =
      mapOf(
        "build" to true, // ignored
        "app/build" to true, // ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/" to true, // ignored
        "build/data.txt" to true, // ignored
        "app/build/" to true, // ignored
        "app/build/data.txt" to true, // ignored
      )
    runGitignoreTests(gitignore, cases1)
  }

  @Test
  fun testEndWithAsteriskPattern() {
    // Relative to root directory
    val gitignore =
      """
      build/*
      """
        .trimIndent()

    // Files
    val cases =
      mapOf(
        "build" to false, // not ignored
        "app/build" to false, // not ignored
      )
    runGitignoreTests(gitignore, cases)

    // Directories
    val cases1 =
      mapOf(
        "build/" to true, // ignored
        "build/data.txt" to true, // ignored
        "build/sub/data.txt" to true, // ignored
        "app/build/" to false, // not ignored
        "app/build/data.txt" to false, // not ignored
        "app/build/sub/data.txt" to false, // not ignored
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
   *   Define individuals or teams responsible for code in a repository</a>
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

  /** Create temporary Git repository and write .gitignore */
  private fun setupRepo(gitignoreContent: String): File {
    val repoDir = createTempDirectory("gitignore_" + UUID.randomUUID().toString()).toFile()

    try { // Initialize Git repository
      runGitCommand(repoDir, "git", "init")

      // Set necessary Git config
      runGitCommand(repoDir, "git", "config", "user.email", "test@example.com")
      runGitCommand(repoDir, "git", "config", "user.name", "Test User")

      // Create and commit .gitignore file
      File(repoDir, ".gitignore").writeText(gitignoreContent)
      runGitCommand(repoDir, "git", "add", ".gitignore")
      runGitCommand(repoDir, "git", "commit", "-m", "Add gitignore")
    } catch (e: Exception) {
      repoDir.deleteRecursively()
      throw e
    }

    return repoDir
  }

  /** Execute Git command and wait for completion */
  private fun runGitCommand(repoDir: File, vararg command: String) {
    val process = ProcessBuilder(*command).directory(repoDir).redirectErrorStream(true).start()
    if (process.waitFor() != 0) {
      val output = process.inputReader().readText()
      error("Command failed: ${command.joinToString(" ")}\n$output")
    }
  }

  /** Check if path is ignored by Git */
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
    return exitCode == 0 // 0 means ignored, 1 means not ignored
  }

  /**
   * Gitignore unit tests
   *
   * Git checks .gitignore line by line from top to bottom. If multiple rules match a path, the LAST
   * matching rule applies.
   *
   * [Git Rules](https://git-scm.com/docs/gitignore#_pattern_format)：
   * - Starts with `#` -> Comment line
   * - Starts with `!` -> Negation rule (un-ignore)
   * - Starts with `/` -> Match from the directory where .gitignore is located
   * - Contains `/` -> Must match corresponding path hierarchy (not arbitrary levels)
   * - Ends with `/` -> Only matches directories
   * - No `/` -> Matches files or directories with the same name at any level
   * - Wildcards
   *     - `*` -> Matches multiple characters, excluding `/`
   *     - `**` -> Matches multiple characters, including `/`
   *     - `?` -> Matches a single character, excluding `/`
   *     - `[...]` -> Matches any single character, excluding `/`
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
        // Ensure file system state is stable
        Thread.sleep(5)

        val result = isIgnored(repoDir, relativePath)
        assertEquals(expected, result, "path: $relativePath")
      }
    } finally {
      println(Ansi.color("============End Test Rule============\n\n", Ansi.Color.GREEN.code))
      // Clean up temporary directory
      repoDir.deleteRecursively()
    }
  }
}
