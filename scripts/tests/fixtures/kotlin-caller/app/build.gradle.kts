plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "probe"
    compileSdk = 34
    defaultConfig { applicationId = "probe.internalabi"; minSdk = 27; targetSdk = 34 }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
}
dependencies { testImplementation("junit:junit:4.13.2") }
