plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.ranaadeeljameel.kabootarbook"
    compileSdk = 35
    defaultConfig {
        applicationId = "com.ranaadeeljameel.kabootarbook"
        minSdk = 24
        targetSdk = 35
        versionCode = 2
        versionName = "0.2.0"
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a") }
    }
    buildFeatures { compose = true }
    packaging { jniLibs { useLegacyPackaging = false } }
}

kotlin { jvmToolchain(17) }

dependencies {
    implementation(platform("androidx.compose:compose-bom:2025.02.00"))
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.navigation:navigation-compose:2.8.8")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    debugImplementation("androidx.compose.ui:ui-tooling")
}

tasks.register<Exec>("buildRustAndroid") {
    workingDir = rootProject.projectDir.resolve("../rust")
    commandLine("cargo", "ndk", "-t", "arm64-v8a", "-t", "armeabi-v7a", "-o", projectDir.resolve("src/main/jniLibs").absolutePath, "build", "--release")
    inputs.dir(rootProject.projectDir.resolve("../rust/src"))
    outputs.dir(projectDir.resolve("src/main/jniLibs"))
}
tasks.named("preBuild") { dependsOn("buildRustAndroid") }
