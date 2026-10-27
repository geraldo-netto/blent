import java.util.Properties
import java.security.MessageDigest

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// T1302: worker budgets belong to Test processes, independently of the daemon.
fun testResourceProperty(name: String, fallback: Int, range: IntRange): Int {
    val raw = providers.gradleProperty(name).orElse(fallback.toString()).get()
    val value = raw.toIntOrNull()
    require(value != null && value in range) { "$name must be within ${range.first}..${range.last}" }
    return value
}

val testHeapMiB = testResourceProperty("blentTestHeapMiB", 2048, 128..16384)
val testForkEvery = testResourceProperty("blentTestForkEvery", 40, 1..1000)

// Local signing properties stay outside Git. Official release tooling verifies
// the APK against the designated public certificate (docs/release-signing.md).
val keystoreProps = Properties().apply {
    val f = rootProject.file(System.getenv("BLENT_KEYSTORE_PROPERTIES") ?: "keystore.properties")
    if (f.exists()) f.inputStream().use { load(it) }
}

// Stable across launches; source or build-input changes invalidate cached profiles.
val sourceDigest = MessageDigest.getInstance("SHA-256")
val identityInputs = fileTree("src/main").files + fileTree("src/profile").files +
    fileTree("../../scripts/benchmarks/android-allocations").files + listOf(
    file("build.gradle.kts"), file("proguard-rules.pro"), rootProject.file("build.gradle.kts"),
    rootProject.file("gradle.properties"),
    rootProject.file("settings.gradle.kts"), rootProject.file("gradle/wrapper/gradle-wrapper.properties")
)
val accessoryIdentityInputs = fileTree("../../scripts/benchmarks/android-accessory").files +
    fileTree("../../scripts/benchmarks/android-accessory-contract").files
(identityInputs + accessoryIdentityInputs).sortedBy { it.path }.forEach { input ->
    val data = input.readBytes()
    sourceDigest.update(input.relativeTo(rootProject.projectDir).path.toByteArray())
    sourceDigest.update(0.toByte())
    sourceDigest.update(data.size.toString().toByteArray())
    sourceDigest.update(0.toByte())
    sourceDigest.update(data)
}
val sourceId = sourceDigest.digest().joinToString("") { "%02x".format(it) }


android {
    namespace = "com.blent"
    compileSdk = 34

    defaultConfig {
        applicationId = "io.github.geraldo_netto.blent"
        minSdk = 27
        targetSdk = 34
        versionCode = 12
        versionName = "1.2.3"
    }

    if (keystoreProps.isNotEmpty()) {
        signingConfigs {
            create("release") {
                storeFile = rootProject.file(keystoreProps.getProperty("storeFile"))
                storePassword = keystoreProps.getProperty("storePassword")
                keyAlias = keystoreProps.getProperty("keyAlias")
                keyPassword = keystoreProps.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        create("profile") {
            initWith(getByName("debug"))
            applicationIdSuffix = ".profile"
            matchingFallbacks += listOf("debug")
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            if (keystoreProps.isNotEmpty()) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
        create("optimized") {
            initWith(getByName("release"))
            applicationIdSuffix = ".optimized"
            signingConfig = signingConfigs.getByName("debug")
            matchingFallbacks += listOf("release")
        }
    }

    sourceSets.getByName("profile").java.srcDir("../../scripts/benchmarks/android-allocations")
    sourceSets.getByName("profile").java.srcDir("../../scripts/benchmarks/android-accessory")
    sourceSets.getByName("profile").java.srcDir("../../scripts/benchmarks/android-accessory-contract")
    sourceSets.getByName("testDebug").java.srcDir("../../scripts/benchmarks/android-accessory-contract")
    sourceSets.getByName("testRelease").java.srcDir("../../scripts/benchmarks/android-accessory-contract")
    sourceSets.getByName("optimized").apply {
        manifest.srcFile("src/profile/AndroidManifest.xml")
        java.srcDir("../../scripts/benchmarks/android-allocations")
        java.srcDir("../../scripts/benchmarks/android-accessory")
        java.srcDir("../../scripts/benchmarks/android-accessory-contract")
        res.srcDir("src/profile/res")
    }
    sourceSets.getByName("testProfile").java.srcDir("../../scripts/benchmarks/android-allocation-tests")
    // T959: permanent compile regression for exact disposable worker fixtures.
    // These remain test-only; never ship the simulated native factory.
    sourceSets.getByName("test").java.srcDir("../../scripts/benchmarks/android-worker-tests")
    sourceSets.getByName("test").resources.srcDir("../../testdata")
    // Compiled with the normal test friend paths; executed only by tlsInteropTest.
    sourceSets.getByName("testDebug").java.srcDir("src/tlsInterop/tests")
    // Exercise the isolated replay's transport lifetime in the normal suite.
    sourceSets.getByName("test").java.srcDir("../../scripts/benchmarks/android-decoder/input")

    lint {
        // T258: a successful scan must not silently omit library checks.
        fatal += "ObsoleteLintCustomCheck"
    }

    testOptions {
        unitTests.isIncludeAndroidResources = true
    }

    buildFeatures {
        compose = true
    }

    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.5"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

// T846: Robolectric's ParcelFileDescriptor shadow accesses the JDK descriptor
// field. This opens only the test JVM; Android application permissions are unchanged.
tasks.withType<Test>().configureEach {
    jvmArgs("--add-opens=java.base/java.io=ALL-UNNAMED")
    maxHeapSize = "${testHeapMiB}m"
    forkEvery = testForkEvery.toLong()
    systemProperty("blent.test.heapMiB", testHeapMiB.toString())
}

// T846: explicit native/JVM integration suite. Ordinary JVM tests need no Rust
// executable; the normal Python tooling suite builds it offline and runs this task.
afterEvaluate {
    val unit = tasks.named<Test>("testDebugUnitTest")
    unit.configure { exclude("**/SecureConnectionInteropTest*") }
    tasks.register<Test>("tlsInteropTest") {
        dependsOn("compileDebugUnitTestKotlin", "compileDebugUnitTestJavaWithJavac",
            "processDebugUnitTestJavaRes", "generateDebugUnitTestConfig")
        testClassesDirs = unit.get().testClassesDirs
        classpath = unit.get().classpath
        systemProperties(unit.get().systemProperties)
        filter { includeTestsMatching("com.blent.connection.SecureConnectionInteropTest") }
        val fixture = providers.gradleProperty("blentTlsFixture").orElse("")
        inputs.property("nativeFixture", fixture)
        outputs.upToDateWhen { false }
        doFirst {
            require(fixture.get().isNotEmpty() && file(fixture.get()).canExecute()) {
                "tlsInteropTest requires -PblentTlsFixture=/absolute/path/to/secure_connection_fixture"
            }
            systemProperty("blent.tls.fixture", file(fixture.get()).absolutePath)
        }
    }
}

// Per-variant generated Kotlin avoids conflating debug and release software.
listOf("debug", "release", "profile", "optimized").forEach { variant ->
    val title = variant.replaceFirstChar { it.uppercaseChar() }
    val profileSource = layout.buildDirectory.dir("generated/source/profile/$variant")
    val generateProfileIdentity = tasks.register("generate${title}ProfileIdentity") {
        inputs.files(identityInputs + accessoryIdentityInputs)
        inputs.property("variant", variant)
        outputs.dir(profileSource)
        doLast {
            val output = profileSource.get().file("com/blent/ProfileBuild.kt").asFile
            output.parentFile.mkdirs()
            output.writeText("package com.blent\ninternal object ProfileBuild { const val SOURCE_ID = \"$sourceId:$variant\" }\n")
        }
    }
    android.sourceSets.getByName(variant).java.srcDir(profileSource)
    tasks.matching { it.name == "pre${title}Build" }.configureEach { dependsOn(generateProfileIdentity) }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    testImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
    testImplementation("org.robolectric:robolectric:4.17")
    // 2024.02.02 ships material3 1.2.1 built against compose 1.6.x — the
    // 2024.01.00 BOM paired material3 1.1.2 with animation-core 1.6.0, which
    // crashes with NoSuchMethodError in CircularProgressIndicator.
    implementation(platform("androidx.compose:compose-bom:2024.02.02"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.activity:activity-compose:1.8.2")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.2")
}
