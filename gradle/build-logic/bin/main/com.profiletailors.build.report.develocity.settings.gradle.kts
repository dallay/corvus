import com.profiletailors.plugin.environment.EnvAccess

plugins { id("com.gradle.develocity") }

// Configure Build Scans (local builds have to opt-in via --scan)
develocity {
  buildScan {
    val isCI = EnvAccess.isCi(providers)
    termsOfUseUrl = "https://gradle.com/help/legal-terms-of-use"
    termsOfUseAgree = "yes"
    publishing.onlyIf { isCI } // only publish with explicit '--scan'
  }
}
