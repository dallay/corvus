/**
 * Precompiled [com.profiletailors.module.java.gradle.kts][Io_github_dallay_module_java_gradle] script plugin.
 *
 * @see Io_github_dallay_module_java_gradle
 */
public
class Io_github_dallay_module_javaPlugin : org.gradle.api.Plugin<org.gradle.api.Project> {
    override fun apply(target: org.gradle.api.Project) {
        try {
            Class
                .forName("Io_github_dallay_module_java_gradle")
                .getDeclaredConstructor(org.gradle.api.Project::class.java, org.gradle.api.Project::class.java)
                .newInstance(target, target)
        } catch (e: java.lang.reflect.InvocationTargetException) {
            throw e.targetException
        }
    }
}
