plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.xima.music.player"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

// Tauri's generated Android project pins the Kotlin Gradle plugin to 1.9.25.
// Anything that drags in a 2.x stdlib makes the 1.9 compiler unable to read its
// metadata, and then even `trim`/`takeIf`/`emptyList` fail to resolve. Keep the
// whole Kotlin line on the compiler's version.
configurations.configureEach {
    resolutionStrategy {
        force("org.jetbrains.kotlin:kotlin-stdlib:1.9.25")
        force("org.jetbrains.kotlin:kotlin-stdlib-jdk7:1.9.25")
        force("org.jetbrains.kotlin:kotlin-stdlib-jdk8:1.9.25")
        force("org.jetbrains.kotlin:kotlin-stdlib-common:1.9.25")
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.16.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    // 1.7.3 is the last line built against Kotlin 1.9; newer coroutines require a 2.x stdlib.
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")

    // Media3 — the audio engine. See docs/CONTRACTS.md §7.
    implementation("androidx.media3:media3-exoplayer:1.11.0")
    implementation("androidx.media3:media3-session:1.11.0")
    implementation("androidx.media3:media3-common:1.11.0")

    // Storage Access Framework helpers for user-picked folders.
    implementation("androidx.documentfile:documentfile:1.1.0")

    // Provided by the Tauri Android scaffolding (`src-tauri/gen/android`).
    implementation(project(":tauri-android"))

    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.6.1")
}
