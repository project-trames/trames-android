# Direct JIT syscall exit validation — 2026-11-27

Application source: `779484e0044ded429fe9e4b9e41d41598bf913a5`.
Runtime pin: `f10a1597574d3b97b5d72b6c7d1d9d4a18ae5c8b`.

[Android CI 45989737168](https://github.com/project-trames/trames-android/actions/runs/45989737168)
passed Windows APK packaging, macOS software CPU checks and native ARM64 Linux
CPU checks. Both default interpreter and explicit JIT configurations passed.
The JIT probe requires two native guest ALU instructions, one direct SVC exit,
zero interpreted instructions, exact retirement/PC and unchanged guest bytes.
The JNI report stays 40 bytes; flag bit 3 records the direct exit result.

Runtime's dedicated ARM64 `software-jit` job in
[run 45989427541](https://github.com/project-trames/trames-runtime/actions/runs/45989427541)
passed generated-code inspection, ten differential tests, Clippy and the CPU
boundary benchmark. Benchmark JSON is retained as `jit-cpu-boundary`.

[Local before/after measurements and detailed cases](https://github.com/project-trames/trames-runtime/blob/f10a1597574d3b97b5d72b6c7d1d9d4a18ae5c8b/docs/validation/jit-syscall-registers-20261127.md)
measure CPU-only exits and simulated returns, not full Linux syscall latency.
Android real-device app-UID execution and full shared Session integration remain
pending. iPad does not depend on this optional JIT crate and was not repinned.
