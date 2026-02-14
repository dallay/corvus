/**
 * Precompiled [com.profiletailors.build.settings.gradle.kts][Io_github_dallay_build_settings_gradle] script plugin.
 *
 * @see Io_github_dallay_build_settings_gradle
 */
public
class Io_github_dallay_buildPlugin : org.gradle.api.Plugin<org.gradle.api.initialization.Settings> {
    override fun apply(target: org.gradle.api.initialization.Settings) {
        try {
            Class
                .forName("Io_github_dallay_build_settings_gradle")
                .getDeclaredConstructor(org.gradle.api.initialization.Settings::class.java, org.gradle.api.initialization.Settings::class.java)
                .newInstance(target, target)
        } catch (e: java.lang.reflect.InvocationTargetException) {
            throw e.targetException
        }
    }
}
