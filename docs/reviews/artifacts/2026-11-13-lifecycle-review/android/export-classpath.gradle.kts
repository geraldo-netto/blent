import org.gradle.api.tasks.testing.Test
import java.io.File

gradle.projectsEvaluated {
    val app = gradle.rootProject.project(":app")
    app.tasks.register("exportLifecycleReviewClasspath") {
        dependsOn("compileDebugUnitTestKotlin", "compileDebugUnitTestJavaWithJavac",
            "processDebugUnitTestJavaRes", "generateDebugUnitTestConfig")
        doLast {
            val task = app.tasks.getByName("testDebugUnitTest") as Test
            File(app.findProperty("blentLifecycleReviewClasspath").toString()).writeText(task.classpath.asPath)
        }
    }
}
