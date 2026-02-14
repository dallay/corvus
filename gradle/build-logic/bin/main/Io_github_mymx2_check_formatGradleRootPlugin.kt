/**
 * Precompiled [com.profiletailors.check.format-gradle-root.gradle.kts][Io_github_dallay_check_format_gradle_root_gradle] script plugin.
 *
 * @see Io_github_dallay_check_format_gradle_root_gradle
 */
public
class Io_github_dallay_check_formatGradleRootPlugin : org.gradle.api.Plugin<org.gradle.api.Project> {
    override fun apply(target: org.gradle.api.Project) {
        try {
            Class
                .forName("Io_github_dallay_check_format_gradle_root_gradle")
                .getDeclaredConstructor(org.gradle.api.Project::class.java, org.gradle.api.Project::class.java)
                .newInstance(target, target)
        } catch (e: java.lang.reflect.InvocationTargetException) {
            throw e.targetException
        }
    }
}
