param([switch]$NativeOnly, [switch]$InterpreterOnly)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$ndkVersion = "28.2.13676358"
$sdk = $env:ANDROID_HOME
if (-not $sdk) { $sdk = $env:ANDROID_SDK_ROOT }
if (-not $sdk) { throw "Set ANDROID_HOME to the Windows Android SDK directory" }
$ndk = Join-Path $sdk "ndk/$ndkVersion"
$bin = Join-Path $ndk "toolchains/llvm/prebuilt/windows-x86_64/bin"
$clang = Join-Path $bin "aarch64-linux-android30-clang.cmd"
if (-not (Test-Path $clang)) { throw "Install NDK $ndkVersion on Windows first (docs/windows.md)" }
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $clang
$env:CC_aarch64_linux_android = $clang
$env:AR_aarch64_linux_android = Join-Path $bin "llvm-ar.exe"
$env:CARGO_INCREMENTAL = "0"
$env:CARGO_TARGET_DIR = Join-Path $root "target"
rustup show
if ($LASTEXITCODE -ne 0) { throw "Rust toolchain installation failed" }
rustup target add aarch64-linux-android
if ($LASTEXITCODE -ne 0) { throw "Android Rust target installation failed" }
$arguments = @("build", "--locked", "--release", "--target", "aarch64-linux-android", "-p", "trames-android-core")
if (-not $InterpreterOnly) { $arguments += @("--features", "jit") }
cargo @arguments
if ($LASTEXITCODE -ne 0) { throw "Rust Android build failed" }
if (-not $NativeOnly) {
    if ($InterpreterOnly) { throw "Interpreter-only mode validates the native core; use -NativeOnly. APK acceptance requires JIT." }
    ./gradlew.bat --no-daemon :app:assembleValidation :app:assembleValidationAndroidTest
    if ($LASTEXITCODE -ne 0) { throw "Android APK build failed" }
}
