plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
}

// Keep in sync with the engine version in the root Cargo.toml.
// (The Rust binding crate labelize-android stays 0.1.0, like wasm/.)
version = "1.6.0"

val fontLicenseDir = layout.buildDirectory.dir("generated/fontLicenseAssets")
val fontLicenseAssets = tasks.register<Sync>("prepareFontLicenseAssets") {
    from(rootProject.file("../licenses/Unifont-OFL.txt")) { into("labelize/licenses") }
    into(fontLicenseDir)
}

android {
    sourceSets.getByName("main").assets.srcDir(fontLicenseDir)

    namespace = "com.goodboy008.labelize"
    compileSdk = 35

    defaultConfig {
        minSdk = 24
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = "11"
    }
}

tasks.named("preBuild") { dependsOn(fontLicenseAssets) }

afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("release") {
                groupId = "com.goodboy008"
                artifactId = "labelize-android"
                version = project.version.toString()
                from(components["release"])
            }
        }
    }
}
