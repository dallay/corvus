@file:Suppress("FunctionName")

package com.profiletailors.config

import com.profiletailors.fixtures.consoleLog
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.MethodOrderer
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Order
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.TestMethodOrder
import org.junit.jupiter.api.assertAll
import org.junit.jupiter.api.assertThrows
import org.yaml.snakeyaml.Yaml
import java.io.File
import java.io.FileNotFoundException

class CodeRabbitConfigTest {

  private val configFile = File(".coderabbit.yaml")
  private val yaml = Yaml()

  private fun loadConfig(): Map<String, Any> {
    if (!configFile.exists()) {
      throw FileNotFoundException("CodeRabbit config file not found at: ${configFile.absolutePath}")
    }
    return yaml.load(configFile.inputStream()) as Map<String, Any>
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class FileExistenceTests {

    @Test
    @Order(0)
    fun `file existence tests`() {
      consoleLog("===== File Existence Tests =====")
    }

    @Test
    @Order(1)
    fun `config file exists at repository root`() {
      assertTrue(configFile.exists(), "CodeRabbit config file should exist at repository root")
      consoleLog("✅ PASS: Config file exists")
    }

    @Test
    @Order(2)
    fun `config file is readable`() {
      assertTrue(configFile.canRead(), "CodeRabbit config file should be readable")
      consoleLog("✅ PASS: Config file is readable")
    }

    @Test
    @Order(3)
    fun `config file is not empty`() {
      assertTrue(configFile.length() > 0, "CodeRabbit config file should not be empty")
      consoleLog("✅ PASS: Config file is not empty")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class YamlStructureTests {

    @Test
    @Order(0)
    fun `yaml structure tests`() {
      consoleLog("===== YAML Structure Tests =====")
    }

    @Test
    @Order(1)
    fun `parses as valid YAML`() {
      val config = loadConfig()
      assertNotNull(config, "Config should parse as valid YAML")
      consoleLog("✅ PASS: Valid YAML structure")
    }

    @Test
    @Order(2)
    fun `contains top-level keys`() {
      val config = loadConfig()
      val expectedKeys = setOf("language", "early_access", "reviews", "chat", "knowledge_base", "code_generation", "issue_enrichment")
      val actualKeys = config.keys
      assertTrue(expectedKeys.all { it in actualKeys }, "Config should contain all expected top-level keys")
      consoleLog("✅ PASS: Contains required top-level keys")
    }

    @Test
    @Order(3)
    fun `language is set correctly`() {
      val config = loadConfig()
      assertEquals("en-US", config["language"], "Language should be set to en-US")
      consoleLog("✅ PASS: Language configuration")
    }

    @Test
    @Order(4)
    fun `early_access is boolean`() {
      val config = loadConfig()
      assertTrue(config["early_access"] is Boolean, "early_access should be a boolean")
      consoleLog("✅ PASS: early_access is boolean")
    }

    @Test
    @Order(5)
    fun `tone_instructions is present`() {
      val config = loadConfig()
      val toneInstructions = config["tone_instructions"] as? String
      assertNotNull(toneInstructions, "tone_instructions should be present")
      assertTrue(toneInstructions!!.isNotEmpty(), "tone_instructions should not be empty")
      consoleLog("✅ PASS: tone_instructions is present and non-empty")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class ReviewsConfigTests {

    @Test
    @Order(0)
    fun `reviews config tests`() {
      consoleLog("===== Reviews Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `reviews section exists`() {
      val config = loadConfig()
      val reviews = config["reviews"] as? Map<*, *>
      assertNotNull(reviews, "reviews section should exist")
      consoleLog("✅ PASS: reviews section exists")
    }

    @Test
    @Order(2)
    fun `reviews profile is set`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      assertEquals("assertive", reviews["profile"], "reviews profile should be assertive")
      consoleLog("✅ PASS: reviews profile is assertive")
    }

    @Test
    @Order(3)
    fun `reviews has auto_review section`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val autoReview = reviews["auto_review"] as? Map<*, *>
      assertNotNull(autoReview, "auto_review section should exist")
      assertTrue(autoReview!!["enabled"] as Boolean, "auto_review should be enabled")
      consoleLog("✅ PASS: auto_review configuration")
    }

    @Test
    @Order(4)
    fun `auto_review has base_branches`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val autoReview = reviews["auto_review"] as Map<*, *>
      val baseBranches = autoReview["base_branches"] as? List<*>
      assertNotNull(baseBranches, "base_branches should exist")
      assertTrue(baseBranches!!.contains("main"), "base_branches should include main")
      assertTrue(baseBranches.contains("develop"), "base_branches should include develop")
      consoleLog("✅ PASS: auto_review base_branches configuration")
    }

    @Test
    @Order(5)
    fun `reviews has finishing_touches configuration`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val finishingTouches = reviews["finishing_touches"] as? Map<*, *>
      assertNotNull(finishingTouches, "finishing_touches section should exist")

      val docstrings = finishingTouches!!["docstrings"] as? Map<*, *>
      val unitTests = finishingTouches["unit_tests"] as? Map<*, *>

      assertNotNull(docstrings, "docstrings section should exist")
      assertNotNull(unitTests, "unit_tests section should exist")
      assertTrue(docstrings!!["enabled"] as Boolean, "docstrings should be enabled")
      assertTrue(unitTests!!["enabled"] as Boolean, "unit_tests should be enabled")
      consoleLog("✅ PASS: finishing_touches configuration")
    }

    @Test
    @Order(6)
    fun `reviews has pre_merge_checks configuration`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val preMergeChecks = reviews["pre_merge_checks"] as? Map<*, *>
      assertNotNull(preMergeChecks, "pre_merge_checks section should exist")

      val title = preMergeChecks!!["title"] as? Map<*, *>
      val description = preMergeChecks["description"] as? Map<*, *>
      val issueAssessment = preMergeChecks["issue_assessment"] as? Map<*, *>

      assertNotNull(title, "title check should exist")
      assertNotNull(description, "description check should exist")
      assertNotNull(issueAssessment, "issue_assessment check should exist")
      assertEquals("warning", title!!["mode"], "title mode should be warning")
      consoleLog("✅ PASS: pre_merge_checks configuration")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class LabelingConfigTests {

    @Test
    @Order(0)
    fun `labeling config tests`() {
      consoleLog("===== Labeling Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `suggested_labels is enabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      assertTrue(reviews["suggested_labels"] as Boolean, "suggested_labels should be enabled")
      consoleLog("✅ PASS: suggested_labels is enabled")
    }

    @Test
    @Order(2)
    fun `labeling_instructions exists and has entries`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val labelingInstructions = reviews["labeling_instructions"] as? List<*>
      assertNotNull(labelingInstructions, "labeling_instructions should exist")
      assertTrue(labelingInstructions!!.isNotEmpty(), "labeling_instructions should have entries")
      consoleLog("✅ PASS: labeling_instructions exists with entries")
    }

    @Test
    @Order(3)
    fun `has security risk label`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val labelingInstructions = reviews["labeling_instructions"] as List<Map<*, *>>
      val securityLabel = labelingInstructions.find { (it["label"] as String) == "risk:security" }
      assertNotNull(securityLabel, "Should have risk:security label")
      assertNotNull(securityLabel!!["instructions"], "Security label should have instructions")
      consoleLog("✅ PASS: risk:security label configured")
    }

    @Test
    @Order(4)
    fun `has performance risk label`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val labelingInstructions = reviews["labeling_instructions"] as List<Map<*, *>>
      val perfLabel = labelingInstructions.find { (it["label"] as String) == "risk:performance" }
      assertNotNull(perfLabel, "Should have risk:performance label")
      consoleLog("✅ PASS: risk:performance label configured")
    }

    @Test
    @Order(5)
    fun `has area labels for kotlin and rust`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val labelingInstructions = reviews["labeling_instructions"] as List<Map<*, *>>
      val labels = labelingInstructions.map { it["label"] as String }

      assertTrue(labels.contains("area:kotlin"), "Should have area:kotlin label")
      assertTrue(labels.contains("area:rust"), "Should have area:rust label")
      consoleLog("✅ PASS: area:kotlin and area:rust labels configured")
    }

    @Test
    @Order(6)
    fun `auto_apply_labels is enabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      assertTrue(reviews["auto_apply_labels"] as Boolean, "auto_apply_labels should be enabled")
      consoleLog("✅ PASS: auto_apply_labels is enabled")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class ToolsConfigTests {

    @Test
    @Order(0)
    fun `tools config tests`() {
      consoleLog("===== Tools Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `tools section exists`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val tools = reviews["tools"] as? Map<*, *>
      assertNotNull(tools, "tools section should exist")
      consoleLog("✅ PASS: tools section exists")
    }

    @Test
    @Order(2)
    fun `rust linter clippy is enabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val tools = reviews["tools"] as Map<*, *>
      val clippy = tools["clippy"] as? Map<*, *>
      assertNotNull(clippy, "clippy tool should be configured")
      assertTrue(clippy!!["enabled"] as Boolean, "clippy should be enabled")
      consoleLog("✅ PASS: clippy (Rust) is enabled")
    }

    @Test
    @Order(3)
    fun `kotlin linter detekt is enabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val tools = reviews["tools"] as Map<*, *>
      val detekt = tools["detekt"] as? Map<*, *>
      assertNotNull(detekt, "detekt tool should be configured")
      assertTrue(detekt!!["enabled"] as Boolean, "detekt should be enabled")
      consoleLog("✅ PASS: detekt (Kotlin) is enabled")
    }

    @Test
    @Order(4)
    fun `security tools are enabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val tools = reviews["tools"] as Map<*, *>

      assertAll(
        { assertTrue((tools["gitleaks"] as Map<*, *>)["enabled"] as Boolean, "gitleaks should be enabled") },
        { assertTrue((tools["trufflehog"] as Map<*, *>)["enabled"] as Boolean, "trufflehog should be enabled") },
        { assertTrue((tools["osvScanner"] as Map<*, *>)["enabled"] as Boolean, "osvScanner should be enabled") }
      )
      consoleLog("✅ PASS: Security tools (gitleaks, trufflehog, osvScanner) are enabled")
    }

    @Test
    @Order(5)
    fun `linting tools are enabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val tools = reviews["tools"] as Map<*, *>

      assertAll(
        { assertTrue((tools["biome"] as Map<*, *>)["enabled"] as Boolean, "biome should be enabled") },
        { assertTrue((tools["markdownlint"] as Map<*, *>)["enabled"] as Boolean, "markdownlint should be enabled") },
        { assertTrue((tools["actionlint"] as Map<*, *>)["enabled"] as Boolean, "actionlint should be enabled") },
        { assertTrue((tools["hadolint"] as Map<*, *>)["enabled"] as Boolean, "hadolint should be enabled") },
        { assertTrue((tools["checkmake"] as Map<*, *>)["enabled"] as Boolean, "checkmake should be enabled") }
      )
      consoleLog("✅ PASS: All linting tools are enabled")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class PathConfigTests {

    @Test
    @Order(0)
    fun `path config tests`() {
      consoleLog("===== Path Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `path_filters exist`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val pathFilters = reviews["path_filters"] as? List<*>
      assertNotNull(pathFilters, "path_filters should exist")
      assertTrue(pathFilters!!.isNotEmpty(), "path_filters should have entries")
      consoleLog("✅ PASS: path_filters exist")
    }

    @Test
    @Order(2)
    fun `filters exclude build artifacts`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val pathFilters = reviews["path_filters"] as List<String>

      assertTrue(pathFilters.any { it.contains("target") }, "Should filter target directory")
      assertTrue(pathFilters.any { it.contains("build") }, "Should filter build directory")
      assertTrue(pathFilters.any { it.contains("node_modules") }, "Should filter node_modules")
      consoleLog("✅ PASS: Build artifacts are filtered")
    }

    @Test
    @Order(3)
    fun `path_instructions exist`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val pathInstructions = reviews["path_instructions"] as? List<*>
      assertNotNull(pathInstructions, "path_instructions should exist")
      assertTrue(pathInstructions!!.isNotEmpty(), "path_instructions should have entries")
      consoleLog("✅ PASS: path_instructions exist")
    }

    @Test
    @Order(4)
    fun `has rust-specific path instructions`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val pathInstructions = reviews["path_instructions"] as List<Map<*, *>>
      val rustInstructions = pathInstructions.find { (it["path"] as String).contains("*.rs") }
      assertNotNull(rustInstructions, "Should have instructions for Rust files")
      val instructions = rustInstructions!!["instructions"] as String
      assertTrue(instructions.contains("ownership"), "Rust instructions should mention ownership")
      assertTrue(instructions.contains("memory safety"), "Rust instructions should mention memory safety")
      consoleLog("✅ PASS: Rust-specific path instructions configured")
    }

    @Test
    @Order(5)
    fun `has kotlin-specific path instructions`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val pathInstructions = reviews["path_instructions"] as List<Map<*, *>>
      val kotlinInstructions = pathInstructions.find { (it["path"] as String).contains("*.kt") }
      assertNotNull(kotlinInstructions, "Should have instructions for Kotlin files")
      val instructions = kotlinInstructions!!["instructions"] as String
      assertTrue(instructions.contains("null safety"), "Kotlin instructions should mention null safety")
      assertTrue(instructions.contains("structured concurrency"), "Kotlin instructions should mention structured concurrency")
      consoleLog("✅ PASS: Kotlin-specific path instructions configured")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class CodeGenerationConfigTests {

    @Test
    @Order(0)
    fun `code generation config tests`() {
      consoleLog("===== Code Generation Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `code_generation section exists`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as? Map<*, *>
      assertNotNull(codeGeneration, "code_generation section should exist")
      consoleLog("✅ PASS: code_generation section exists")
    }

    @Test
    @Order(2)
    fun `docstrings configuration exists`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as Map<*, *>
      val docstrings = codeGeneration["docstrings"] as? Map<*, *>
      assertNotNull(docstrings, "docstrings section should exist")
      assertEquals("en-US", docstrings!!["language"], "docstrings language should be en-US")
      consoleLog("✅ PASS: docstrings configuration")
    }

    @Test
    @Order(3)
    fun `docstrings has kotlin path instructions`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as Map<*, *>
      val docstrings = codeGeneration["docstrings"] as Map<*, *>
      val pathInstructions = docstrings["path_instructions"] as? List<Map<*, *>>
      assertNotNull(pathInstructions, "docstrings path_instructions should exist")

      val kotlinInstructions = pathInstructions!!.find { (it["path"] as String).contains("*.kt") }
      assertNotNull(kotlinInstructions, "Should have docstring instructions for Kotlin")
      val instructions = kotlinInstructions!!["instructions"] as String
      assertTrue(instructions.contains("KDoc"), "Kotlin docstring instructions should mention KDoc")
      consoleLog("✅ PASS: Kotlin docstring path instructions")
    }

    @Test
    @Order(4)
    fun `docstrings has rust path instructions`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as Map<*, *>
      val docstrings = codeGeneration["docstrings"] as Map<*, *>
      val pathInstructions = docstrings["path_instructions"] as List<Map<*, *>>

      val rustInstructions = pathInstructions.find { (it["path"] as String).contains("*.rs") }
      assertNotNull(rustInstructions, "Should have docstring instructions for Rust")
      val instructions = rustInstructions!!["instructions"] as String
      assertTrue(instructions.contains("rustdoc"), "Rust docstring instructions should mention rustdoc")
      consoleLog("✅ PASS: Rust docstring path instructions")
    }

    @Test
    @Order(5)
    fun `unit_tests has path instructions`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as Map<*, *>
      val unitTests = codeGeneration["unit_tests"] as? Map<*, *>
      assertNotNull(unitTests, "unit_tests section should exist")

      val pathInstructions = unitTests!!["path_instructions"] as? List<Map<*, *>>
      assertNotNull(pathInstructions, "unit_tests path_instructions should exist")
      assertTrue(pathInstructions!!.size >= 2, "Should have instructions for at least 2 languages")
      consoleLog("✅ PASS: unit_tests path instructions configured")
    }

    @Test
    @Order(6)
    fun `unit_tests kotlin instructions mention TDD`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as Map<*, *>
      val unitTests = codeGeneration["unit_tests"] as Map<*, *>
      val pathInstructions = unitTests["path_instructions"] as List<Map<*, *>>

      val kotlinInstructions = pathInstructions.find { (it["path"] as String).contains("*.kt") }
      assertNotNull(kotlinInstructions, "Should have unit test instructions for Kotlin")
      val instructions = kotlinInstructions!!["instructions"] as String
      assertTrue(instructions.contains("Red-Green-Refactor"), "Kotlin test instructions should mention TDD/Red-Green-Refactor")
      consoleLog("✅ PASS: Kotlin unit test instructions mention TDD")
    }

    @Test
    @Order(7)
    fun `unit_tests rust instructions mention ownership`() {
      val config = loadConfig()
      val codeGeneration = config["code_generation"] as Map<*, *>
      val unitTests = codeGeneration["unit_tests"] as Map<*, *>
      val pathInstructions = unitTests["path_instructions"] as List<Map<*, *>>

      val rustInstructions = pathInstructions.find { (it["path"] as String).contains("*.rs") }
      assertNotNull(rustInstructions, "Should have unit test instructions for Rust")
      val instructions = rustInstructions!!["instructions"] as String
      assertTrue(instructions.contains("ownership"), "Rust test instructions should mention ownership")
      consoleLog("✅ PASS: Rust unit test instructions mention ownership")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class KnowledgeBaseConfigTests {

    @Test
    @Order(0)
    fun `knowledge base config tests`() {
      consoleLog("===== Knowledge Base Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `knowledge_base section exists`() {
      val config = loadConfig()
      val knowledgeBase = config["knowledge_base"] as? Map<*, *>
      assertNotNull(knowledgeBase, "knowledge_base section should exist")
      consoleLog("✅ PASS: knowledge_base section exists")
    }

    @Test
    @Order(2)
    fun `code_guidelines is enabled`() {
      val config = loadConfig()
      val knowledgeBase = config["knowledge_base"] as Map<*, *>
      val codeGuidelines = knowledgeBase["code_guidelines"] as? Map<*, *>
      assertNotNull(codeGuidelines, "code_guidelines section should exist")
      assertTrue(codeGuidelines!!["enabled"] as Boolean, "code_guidelines should be enabled")
      consoleLog("✅ PASS: code_guidelines is enabled")
    }

    @Test
    @Order(3)
    fun `code_guidelines references agent documentation`() {
      val config = loadConfig()
      val knowledgeBase = config["knowledge_base"] as Map<*, *>
      val codeGuidelines = knowledgeBase["code_guidelines"] as Map<*, *>
      val filePatterns = codeGuidelines["filePatterns"] as? List<String>
      assertNotNull(filePatterns, "filePatterns should exist")
      assertTrue(filePatterns!!.contains("AGENTS.md"), "Should reference AGENTS.md")
      consoleLog("✅ PASS: code_guidelines references agent documentation")
    }

    @Test
    @Order(4)
    fun `web_search is enabled`() {
      val config = loadConfig()
      val knowledgeBase = config["knowledge_base"] as Map<*, *>
      val webSearch = knowledgeBase["web_search"] as? Map<*, *>
      assertNotNull(webSearch, "web_search section should exist")
      assertTrue(webSearch!!["enabled"] as Boolean, "web_search should be enabled")
      consoleLog("✅ PASS: web_search is enabled")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class ChatConfigTests {

    @Test
    @Order(0)
    fun `chat config tests`() {
      consoleLog("===== Chat Configuration Tests =====")
    }

    @Test
    @Order(1)
    fun `chat section exists`() {
      val config = loadConfig()
      val chat = config["chat"] as? Map<*, *>
      assertNotNull(chat, "chat section should exist")
      consoleLog("✅ PASS: chat section exists")
    }

    @Test
    @Order(2)
    fun `auto_reply is enabled`() {
      val config = loadConfig()
      val chat = config["chat"] as Map<*, *>
      assertTrue(chat["auto_reply"] as Boolean, "auto_reply should be enabled")
      consoleLog("✅ PASS: auto_reply is enabled")
    }

    @Test
    @Order(3)
    fun `art is disabled`() {
      val config = loadConfig()
      val chat = config["chat"] as Map<*, *>
      assertFalse(chat["art"] as Boolean, "art should be disabled for professional context")
      consoleLog("✅ PASS: art is disabled")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class EdgeCaseTests {

    @Test
    @Order(0)
    fun `edge case tests`() {
      consoleLog("===== Edge Case Tests =====")
    }

    @Test
    @Order(1)
    fun `config is not empty map`() {
      val config = loadConfig()
      assertTrue(config.isNotEmpty(), "Config should not be empty")
      assertTrue(config.size > 5, "Config should have substantial content")
      consoleLog("✅ PASS: Config has substantial content")
    }

    @Test
    @Order(2)
    fun `all list values are non-empty where expected`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>

      assertAll(
        {
          val pathFilters = reviews["path_filters"] as List<*>
          assertTrue(pathFilters.isNotEmpty(), "path_filters should not be empty")
        },
        {
          val pathInstructions = reviews["path_instructions"] as List<*>
          assertTrue(pathInstructions.isNotEmpty(), "path_instructions should not be empty")
        },
        {
          val labelingInstructions = reviews["labeling_instructions"] as List<*>
          assertTrue(labelingInstructions.isNotEmpty(), "labeling_instructions should not be empty")
        }
      )
      consoleLog("✅ PASS: All expected list values are non-empty")
    }

    @Test
    @Order(3)
    fun `security-first priority is reflected in config`() {
      val config = loadConfig()
      val toneInstructions = config["tone_instructions"] as String
      assertTrue(
        toneInstructions.contains("security", ignoreCase = true),
        "tone_instructions should mention security priority"
      )

      val reviews = config["reviews"] as Map<*, *>
      val labelingInstructions = reviews["labeling_instructions"] as List<Map<*, *>>
      val hasSecurityLabel = labelingInstructions.any {
        (it["label"] as String).contains("security", ignoreCase = true)
      }
      assertTrue(hasSecurityLabel, "Should have security-related labels")
      consoleLog("✅ PASS: Security-first priority is reflected")
    }

    @Test
    @Order(4)
    fun `all boolean flags are actually boolean type`() {
      val config = loadConfig()
      assertAll(
        { assertTrue(config["early_access"] is Boolean, "early_access should be Boolean") },
        {
          val reviews = config["reviews"] as Map<*, *>
          assertTrue(reviews["suggested_labels"] is Boolean, "suggested_labels should be Boolean")
        },
        {
          val reviews = config["reviews"] as Map<*, *>
          assertTrue(reviews["auto_apply_labels"] is Boolean, "auto_apply_labels should be Boolean")
        },
        {
          val chat = config["chat"] as Map<*, *>
          assertTrue(chat["auto_reply"] is Boolean, "auto_reply should be Boolean")
        }
      )
      consoleLog("✅ PASS: All boolean flags are proper Boolean type")
    }

    @Test
    @Order(5)
    fun `no null values in critical paths`() {
      val config = loadConfig()
      assertAll(
        { assertNotNull(config["language"], "language should not be null") },
        { assertNotNull(config["reviews"], "reviews should not be null") },
        { assertNotNull(config["code_generation"], "code_generation should not be null") },
        {
          val reviews = config["reviews"] as Map<*, *>
          assertNotNull(reviews["tools"], "tools should not be null")
        }
      )
      consoleLog("✅ PASS: No null values in critical configuration paths")
    }

    @Test
    @Order(6)
    fun `monorepo-specific configurations are present`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      val pathFilters = reviews["path_filters"] as List<String>

      // Monorepo typically has build, gradle, cargo directories
      val hasMonorepoFilters = pathFilters.any { it.contains("gradle") || it.contains("cargo") }
      assertTrue(hasMonorepoFilters, "Should have monorepo-specific filters")

      val labelingInstructions = reviews["labeling_instructions"] as List<Map<*, *>>
      val hasMultipleAreaLabels = labelingInstructions.count {
        (it["label"] as String).startsWith("area:")
      } >= 3
      assertTrue(hasMultipleAreaLabels, "Should have multiple area labels for monorepo")
      consoleLog("✅ PASS: Monorepo-specific configurations are present")
    }

    @Test
    @Order(7)
    fun `poem feature is disabled`() {
      val config = loadConfig()
      val reviews = config["reviews"] as Map<*, *>
      assertFalse(reviews["poem"] as Boolean, "poem should be disabled for professional reviews")
      consoleLog("✅ PASS: poem feature is appropriately disabled")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class NegativeTests {

    @Test
    @Order(0)
    fun `negative tests`() {
      consoleLog("===== Negative Tests =====")
    }

    @Test
    @Order(1)
    fun `handles missing file gracefully`() {
      val nonExistentFile = File("nonexistent.yaml")
      val yaml = Yaml()

      val exception = assertThrows<FileNotFoundException> {
        if (!nonExistentFile.exists()) {
          throw FileNotFoundException("File not found")
        }
        yaml.load(nonExistentFile.inputStream())
      }
      assertNotNull(exception, "Should throw FileNotFoundException for missing file")
      consoleLog("✅ PASS: Handles missing file gracefully")
    }

    @Test
    @Order(2)
    fun `rejects invalid yaml with clear error`() {
      val invalidYaml = """
        invalid: yaml: content:
        - broken
        structure
      """.trimIndent()

      val yaml = Yaml()
      val exception = assertThrows<Exception> {
        yaml.load(invalidYaml)
      }
      assertNotNull(exception, "Should throw exception for invalid YAML")
      consoleLog("✅ PASS: Rejects invalid YAML - ${exception.message}")
    }

    @Test
    @Order(3)
    fun `config does not have unexpected top-level keys`() {
      val config = loadConfig()
      val validTopLevelKeys = setOf(
        "language", "early_access", "tone_instructions", "reviews",
        "chat", "knowledge_base", "code_generation", "issue_enrichment"
      )

      val unexpectedKeys = config.keys.filter { it !in validTopLevelKeys }
      assertTrue(
        unexpectedKeys.isEmpty() || unexpectedKeys.all { it.startsWith("_") },
        "Config should only have expected top-level keys or extension keys starting with _. Found: $unexpectedKeys"
      )
      consoleLog("✅ PASS: No unexpected top-level keys")
    }
  }
}