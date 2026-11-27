plugins { id("com.android.application") }
android {
    namespace = "org.trames.android"
    compileSdk = 37
    ndkVersion = "28.2.13676358"
    defaultConfig {
        applicationId = "org.trames.android"
        minSdk = 30
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "org.trames.android.CpuAcceptance"
        ndk { abiFilters += "arm64-v8a" }
        externalNativeBuild { cmake { arguments += "-DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON" } }
    }
    buildTypes {
        release { isMinifyEnabled = false }
        create("validation") {
            initWith(getByName("release"))
            applicationIdSuffix = ".validation"
            isDebuggable = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    testBuildType = "validation"
    externalNativeBuild { cmake { path = file("src/main/cpp/CMakeLists.txt"); version = "3.22.1" } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
}
