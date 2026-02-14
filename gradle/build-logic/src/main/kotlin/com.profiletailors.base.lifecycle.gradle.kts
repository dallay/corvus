import com.profiletailors.plugin.resetTaskGroup
import com.profiletailors.plugin.utils.Ansi

plugins { base }

// Convenience for local development: when running './gradlew' without parameters show the
// tasks..., also you can use '--task-graph' to print the task dependency graph.
// './gradlew' = './gradlew tasks'
defaultTasks("tasks")

tasks.register("qualityCheck") {
  group = "verification"
  description = "Runs checks (without executing tests)"
  dependsOn(tasks.assemble)
}

tasks.register("qualityGate") {
  group = "build"
  description = "Runs checks and auto-corrects (without executing tests)"
  dependsOn(tasks.assemble)
}

tasks.check { dependsOn(tasks.named("qualityCheck")) }

val groups =
  mapOf(
    "build" to setOf("assemble", "build", "clean", "qualityGate"),
    "docs" to setOf("doc.*".toRegex()),
    "help" to
      setOf(
        "help",
        "projects",
        "properties",
        "tasks",
        "dependencies",
        "buildEnvironment",
        "kotlinDslAccessorsReport",
      ),
    "others" to setOf(".*".toRegex()),
    "publishing" to
      setOf(
        "generatePomFileFor.*Publication".toRegex(),
        "publish",
        "publishAll.*".toRegex(),
        "publishTo.*".toRegex(),
        "publishPluginMaven.*".toRegex(),
      ),
    "toolbox" to setOf(".*".toRegex()),
    "verification" to setOf("check", "test.*".toRegex(), "qualityCheck"),
  )
val groupRegex = Regex(""" \[group = (.*)]""")

// Cleanup the task group by removing all tasks developers usually do not need to call
// directly
gradle.projectsEvaluated { tasks.configureEach { configureGroup(groupRegex, groups) } }

listOf(
    "run" to "build",
    "buildDependents" to "toolbox",
    "distZip" to "toolbox",
    "publishPlugins" to "publishing",
  )
  .forEach { resetTaskGroup(it.first, it.second) }

@Suppress("detekt:CyclomaticComplexMethod")
fun Task.configureGroup(groupRegex: Regex, groupMap: Map<String, Set<Any>>) {
  val printInfo = false
  val printReset = false

  val taskClz = this::class.java.name

  fun printTaskGroup() {
    println(
      "${Ansi.color("task group => ${group}\n  $name", "36")}${Ansi.color("\n  $taskClz", "31")}"
    )
  }
  if (printInfo) printTaskGroup()
  fun resetTaskGroup() {
    if (printReset) printTaskGroup()
    description = description.let { if (!it.isNullOrBlank()) "$it [from = $group]" else "$it" }
    group = null
  }
  val reGroup = groupRegex.find(description.orEmpty())
  if (reGroup != null) {
    group = reGroup.groupValues[1]
  } else if (group != null) {
    if (!groupMap.keys.contains(group)) {
      resetTaskGroup()
    } else {
      if (
        groupMap[group!!]!!.none {
          when (it) {
            is String -> name == it
            is Regex -> name.matches(it)
            else -> false
          }
        }
      ) {
        resetTaskGroup()
      }
    }
  }
  if (!description.orEmpty().contains("[taskClz = ")) {
    description = "$description [taskClz = $taskClz]"
  }
}
