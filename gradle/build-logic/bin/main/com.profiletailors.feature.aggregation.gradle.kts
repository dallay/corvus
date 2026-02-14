plugins { java }

// The 'aggregation' project is not supposed to have source code. The 'java' plugin is applied only
// for its dependency management features. Therefore, the following tasks which are part of
// 'assemble' are disabled.
tasks.compileJava {
  classpath = files()
  enabled = false
}

tasks.processResources { enabled = false }

tasks.jar { enabled = false }
