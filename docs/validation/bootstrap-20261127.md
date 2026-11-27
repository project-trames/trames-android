# Android execution bootstrap — 2026-11-27

Validated application source: `483dbc4c7a976291a0794657b598ab7e2de37a18`.
Runtime pin: `964f8725d33c899dc6b62f1b499c114d03d91190`.

[GitHub Actions run 45983764045](https://github.com/project-trames/trames-android/actions/runs/45983764045)
completed successfully in all three jobs:

- Windows hosted runner: interpreter-only Android static library, explicit JIT
  Android static library, non-debuggable validation APK and instrumentation APK.
- macOS 15: default/optional CPU configurations and six differential JIT tests.
- ARM64 Linux: default/optional CPU probes and six JIT tests that assert actual
  native guest instruction execution (including a 2,048-encoding corpus).

Windows is the GitHub-hosted build machine. No personal Windows computer was
accessed. The Mac received no Android SDK, NDK, JDK or emulator installation.

The downloaded APK was independently inspected without an Android toolchain:
its package is `org.trames.android.validation`, `debuggable` is false, and its
only native library is ARM64 `libtrames_android.so`. All four ELF PT_LOAD
segments use 16 KiB alignment, none is writable and executable simultaneously,
and the uncompressed native library's ZIP data is 16 KiB aligned.

APK SHA-256: `0a81d01f4728577a9beefbb56cd5726c2a8ba62ef7b26c17d71d42af374c7f49`.
The Actions artifact `trames-android-validation` contains the app/test APK pair.

These are cross-build, package-layout and CPU-backend results. Android app-UID
execution, ART signal coexistence, 4/16 KiB real-device acceptance and full Linux
session/terminal integration remain pending. Current JIT coverage is the first
integer-block compiler with interpreter fallback, not a performance claim.

The iPad project now re-exports the same shared interpreter. Its all-features
CPU dependency graph excludes `trames-jit`; iOS/Linux CPU builds and product
boundary checks passed. Later Runtime lifecycle-test lint fixes affect no
Android production code, so this tested Android pin is retained.
