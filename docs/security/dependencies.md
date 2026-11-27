# Dependency notices

Rust records and original license texts come from the pinned shared Runtime.
The Gradle 9.4.1 wrapper is from https://github.com/gradle/gradle/tree/v9.4.1
and retains its Apache 2.0 license at `third-party/gradle-LICENSE.txt`.
The wrapper JAR SHA-256 is checked during bootstrap; its distribution ZIP hash
is pinned in `gradle-wrapper.properties`. Android build tools are external tools,
not bundled SDK binaries. APK runtime dependencies are the platform APIs and
our own JNI/Rust libraries; no Android UI/terminal library is bundled yet.

<!-- BEGIN GENERATED RUST DEPENDENCIES -->

## Rust dependencies

Generated from the committed lockfiles. This is the complete locked closure,
including optional, build, test and other-target packages; it is not a claim
that every entry is linked into each product. Project-owned packages are excluded.

| Package | Version | Declared license | Selected license | Scope |
| --- | --- | --- | --- | --- |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | MIT | Main |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | MIT | Main |
| errno | 0.3.14 | MIT OR Apache-2.0 | MIT | Main |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | MIT | Main |
| libc | 0.2.190 | MIT OR Apache-2.0 | MIT | Main |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | Main |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | MIT | Main |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | MIT | Main |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | MIT | Main |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | MIT | Main |
| redox_syscall | 0.5.18 | MIT | MIT | Main |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | Main |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | MIT | Main |
| smallvec | 1.16.3 | MIT OR Apache-2.0 | MIT | Main |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | Main |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | MIT | Main |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | MIT | Main |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | MIT | Main |

Auxiliary lockfiles: none.

<!-- END GENERATED RUST DEPENDENCIES -->
