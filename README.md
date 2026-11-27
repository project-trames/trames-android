# Trames Android

Non-root Android app using the shared [Trames Runtime](https://github.com/project-trames/trames-runtime).
The selected execution route is a software JIT with the shared interpreter as
its semantic fallback. Original guest executable bytes are never rewritten.

Android SDK/NDK/JDK and device/emulator work belong on **Windows**. The Mac keeps
source and runs shared Rust checks; no Android toolchain or new VM is required.
See [Windows setup](docs/windows.md) and [execution design](docs/execution.md).

Current milestone: shared interpreter, opt-in ARM64 native integer blocks,
JNI CPU probe, minimal application shell, and Windows APK build pipeline.
The application currently checks the CPU backend. It does not yet launch a
Linux distribution, terminal, systemd, Docker, graphics or multimedia sessions.

```powershell
git clone --recurse-submodules https://github.com/project-trames/trames-android.git
cd trames-android
./scripts/build.ps1
```

Pure Rust host checks (no Android SDK):

```sh
cargo test --locked -p trames-android-core
cargo test --locked -p trames-android-core --features jit
```

GPL-3.0-or-later. Runtime and dependencies retain their own notices.
