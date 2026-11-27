# Native scalar memory acceptance — 2026-11-27

Runtime pin: `d764fc7b920aa73e31b7a91c88512b36c349c5eb`.

The native backend now lowers scalar B/H/W/X LDR/STR, signed loads, immediate,
unscaled, pre/post-index, extended register offsets and literal loads. Checked
Memory/View callbacks retain the original permissions/COW/dirty/code/exclusive
model. Cross-page/provider staging is explicit; no host address or lease is cached.
No new emulated extension state or guest RX rewrite is introduced.

Actual ARM64 Linux passes 33 JIT tests and the 281-case memory unit suite (three
original ignores); macOS passes 28 non-native JIT cases and 278 memory cases
(three original ignores), with scoped Clippy and an iOS interpreter compile check.
Tests include write-only mappings/backings, cross-page faults, precise failed
writeback, COW, code writes, exclusive invalidation and non-atomic lost updates.

The core probe adds bit 6: four retired guest instructions complete a native
load/add/store/SVC round trip, two direct memory operations and zero fallback.
Instrumentation requires that bit on a real app UID. JNI stays schema 1 / 40 bytes;
original initial-probe counters remain unchanged. The probe executes on actual
ARM64 Linux, and default/core-JIT builds plus compile-time isolation pass on Mac.
Windows CI builds the JIT core and non-debuggable validation APKs successfully;
real Android app-UID and 4/16 KiB device acceptance remain pending. No Android
tools were installed on the Mac.

Five alternating old/new process pairs give a **1.66x** scalar-memory-loop gain
over the old JIT, with zero memory interpreter fallback. The new fragment path
still takes about five times the cached interpreter's tight-loop time; block
fusion remains necessary. SVC-only median is 2.5% slower and continuation 0.8%
slower; all samples, earlier pairs and limitations are retained. CPU-only timings
exclude real Linux syscall handling, JNI, lifecycle and startup.

[Detailed scope, exact sources, all raw samples and validation](https://github.com/project-trames/trames-runtime/blob/d764fc7b920aa73e31b7a91c88512b36c349c5eb/docs/validation/jit-memory-20261127.md).

CI receipts preserve the tested commits rather than rerunning for this record:

- Android `a9e32e064562938376836b6ec2675a22fe5b4f31`: all three jobs pass —
  Windows packaging, ARM64 Linux native CPU and macOS CPU
  ([run 46017052454](https://github.com/project-trames/trames-android/actions/runs/46017052454)).
- Runtime `d764fc7b920aa73e31b7a91c88512b36c349c5eb`: the dedicated ARM64
  software-JIT job passes Clippy, all JIT tests and CPU-only microbenchmarks
  ([job 140566308481](https://github.com/project-trames/trames-runtime/actions/runs/46016924642/job/140566308481)).
- The same Runtime run also passes the complete Linux workspace checks, musl
  DNS ABI, release build and reclaim-footprint check. macOS workspace Clippy
  passes; its full workspace test/release job is still running at this record's
  publication. The local scoped macOS tests and iOS interpreter check above
  already pass; a running broad job is not counted as a completed pass.
