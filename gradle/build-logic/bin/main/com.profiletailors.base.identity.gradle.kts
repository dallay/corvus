import com.profiletailors.plugin.environment.EnvAccess
import com.profiletailors.plugin.environment.buildProperties
import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import net.swiftzer.semver.SemVer

plugins { base }

val buildProperties = project.buildProperties()

// Set the group required to refer to a Module "from outside".
// I.e., when it is published or used in Included Builds.
group = project.getPropOrDefault(LocalConfig.Props.GROUP)

val isCI = EnvAccess.isCi(providers)
val currVer =
  buildProperties
    .getProperty("version", "")
    .ifBlank { project.getPropOrDefault(LocalConfig.Props.VERSION) }
    .let { SemVer.parse(it) }

currVer.preRelease?.also {
  val preReleasePattern = "SNAPSHOT|dev\\d*|preview\\d*|alpha\\d*|beta\\d*|m\\d+|rc\\d+"
  require(currVer.preRelease!!.matches(preReleasePattern.toRegex())) {
    "Pre-release should match: $preReleasePattern"
  }
}

val githubEventName = System.getenv("GITHUB_EVENT_NAME") ?: ""
val githubRefName = System.getenv("GITHUB_REF_NAME") ?: ""
val isScheduled = githubEventName == "schedule"
val isManual = githubEventName == "workflow_dispatch"
val isGithubTag = githubRefName.startsWith("v")

// release check: push tag should match code version
if (isCI && githubEventName == "push" && isGithubTag) {
  require(currVer.toString() == githubRefName.removePrefix("v")) {
    "CI Release: GitHub tag ($githubRefName) must match Code version (${currVer})"
  }
}

version =
  if (isCI && (isScheduled || isManual)) {
    SemVer(
        currVer.major,
        currVer.minor,
        LocalDate.now().format(DateTimeFormatter.ofPattern("yyMMdd")).toInt(),
        "SNAPSHOT",
      )
      .toString()
  } else {
    // local build
    SemVer(currVer.major, currVer.minor, currVer.patch, currVer.preRelease, null).toString()
  }
