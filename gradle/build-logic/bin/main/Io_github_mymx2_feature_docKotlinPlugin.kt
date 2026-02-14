/**
 * Precompiled [com.profiletailors.feature.doc-kotlin.gradle.kts][Io_github_dallay_feature_doc_kotlin_gradle] script plugin.
 *
 * @see Io_github_dallay_feature_doc_kotlin_gradle
 */
public
class Io_github_dallay_feature_docKotlinPlugin : org.gradle.api.Plugin<org.gradle.api.Project> {
    override fun apply(target: org.gradle.api.Project) {
        try {
            Class
                .forName("Io_github_dallay_feature_doc_kotlin_gradle")
                .getDeclaredConstructor(org.gradle.api.Project::class.java, org.gradle.api.Project::class.java)
                .newInstance(target, target)
        } catch (e: java.lang.reflect.InvocationTargetException) {
            throw e.targetException
        }
    }
}
