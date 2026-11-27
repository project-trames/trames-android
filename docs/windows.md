# Windows development

Use Git for Windows, Rustup, Python 3, JDK 17 and Android Studio/command-line
tools. Android tools are deliberately not provisioned on the Mac.

Set `ANDROID_HOME` to the Windows SDK path. Install these pinned packages with
the SDK manager (Android Studio can also install them):

```powershell
sdkmanager.bat "platforms;android-36" "build-tools;36.0.0" "ndk;28.2.13676358" "cmake;3.22.1" "platform-tools"
git submodule update --init --recursive
./scripts/build.ps1
```

The repository's pinned Rust nightly is installed by Rustup. `build.ps1` selects
the Windows NDK clang linker, builds a separate Rust Android static library,
then packages it through the checked-in Gradle wrapper. The wrapper ZIP has a
pinned SHA-256. JIT is explicitly enabled by this script; Cargo's default build
is interpreter-only. `-NativeOnly -InterpreterOnly` checks that alternate build.

Initial packaging is arm64-v8a; guest AArch64 state is independent of the host
ABI. CMake has a distinct x86_64 mapping for future emulator packaging. There
is no x86 JIT backend yet. Native code uses actual host page sizes; guest pages
remain 4096 bytes. Native shared libraries request 16 KiB ELF segment alignment.
Build compatibility does not prove execution on a 16 KiB device.

For app-UID CPU acceptance on a connected ARM64 Android device:

```powershell
adb install -r app/build/outputs/apk/validation/app-validation.apk
adb install -r app/build/outputs/apk/androidTest/validation/app-validation-androidTest.apk
adb shell am instrument -w org.trames.android.validation.test/org.trames.android.CpuAcceptance
```

The validation app is non-debuggable and uses a separate application ID with
local debug signing. It is not the signed release product. Instrumentation
rejects shell/root UIDs and requires two actual native guest instructions,
a direct compiled syscall exit without interpreter fallback, native block links,
cached post-syscall resumption, correct accounting
and unchanged guest bytes. Keep device model,
Android build, app UID, host page size and instrumentation report as evidence.
Repeat on both 4 KiB and 16 KiB Android hosts before claiming device acceptance.

Tool versions follow the [AGP 9.2 compatibility table](https://developer.android.com/build/releases/agp-9-2-0-release-notes)
and [Android page-size guidance](https://developer.android.com/guide/practices/page-sizes).
