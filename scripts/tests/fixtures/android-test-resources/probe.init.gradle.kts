import org.gradle.api.tasks.testing.Test

gradle.projectsEvaluated {
    rootProject.tasks.register("t1302Probe") {
        doLast {
            val rows = rootProject.project(":app").tasks.withType(Test::class.java).map {
                "${it.name}\t${it.maxHeapSize}\t${it.forkEvery}\t${it.systemProperties["blent.test.heapMiB"]}"
            }
            rootProject.file(rootProject.property("blentTestProbeOutput").toString())
                .writeText(rows.joinToString("\n"))
        }
    }
}
