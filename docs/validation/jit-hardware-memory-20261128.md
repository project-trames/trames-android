# Hardware-alias JIT acceptance — 2026-11-28

Paired Runtime: `a96bacdc9f8ad3a49555108e6efa95817d87403b`.

Scalar JIT cache hits now issue inline host LDR/STR through existing protected
CPU aliases, retain register bindings and avoid the per-access Rust helper.
Entry-scoped aliases use the existing private/immutable ownership admission;
shared writable/COW/provider and software-only mixed-page cases stay checked.
Generated native fault PCs recover to local checked continuations without
replaying completed instructions or skipping Rust frames. Guest executable bytes
and the extension register model are unchanged.

Android flag 6 now checks two load/add/store rounds followed by SVC: 40 -> 42,
7 retired instructions, 4 memory operations, exactly 2 actual inline hardware
accesses, one fused entry, zero staging/fallback and unchanged original code.
Data occupies a complete runtime-detected host protection group for 4/16 KiB
configurations. JNI schema 1 / 40-byte response and flag bit 6 remain unchanged.
`check-features.py` additionally requires dynamic native-memory code to be absent
from the interpreter build and present in explicit JIT builds; no runtime feature
probe is added.

Local Linux ARM64 core tests pass both interpreter and JIT, and JIT all-target
Clippy passes. Mac core tests, feature checks and reviewed-dependency notices pass.
Runtime Linux release acceptance is 6 native/capability + 36 differential tests;
Linux/Apple shared-memory tests and iOS interpreter compile/feature isolation also
pass. Mac does not execute native JIT. Windows packaging CI is triggered by this
paired commit; real non-debuggable Android app-UID/ART/16 KiB execution still needs
a device. No Android SDK/NDK/JDK/emulator was installed on Mac.

In a new matched resident/private worker cohort on the existing nested ARM64
Linux/KVM VM, the 1M hot-page scalar loop is old JIT 27.679 ms, new JIT 1.467 ms,
interpreter 22.032 ms, nested KVM 0.854 ms. That is 18.87x improvement over old JIT
and 1.72x KVM elapsed time, with 99.9991% inline hits. This single hot location
is a CPU microbenchmark, not general Android application performance. Original
short SVC tail execution is retained and checked. Shared writable pages do not
receive the private-page speedup.

[Runtime implementation, safety contracts, all 960 timed samples and source audit](https://github.com/project-trames/trames-runtime/blob/a96bacdc9f8ad3a49555108e6efa95817d87403b/docs/validation/jit-hardware-memory-20261128.md).

CI receipts retain the tested implementation identities:

- Runtime `a96bacd`: [ARM64 native JIT job](https://github.com/project-trames/trames-runtime/actions/runs/46041721341/job/140650814040)
  passed strict Clippy, all 42 JIT tests and the CPU fixture.
- Android `feed058`: [ARM64 CPU/core job](https://github.com/project-trames/trames-android/actions/runs/46041819268/job/140651157906)
  passed compile-time feature checks, interpreter/JIT core probes and all Runtime
  JIT tests. The [Windows packaging job](https://github.com/project-trames/trames-android/actions/runs/46041819268/job/140651157664)
  is still running; Android Mac CI is queued. These are not counted as passes.
- Runtime's broader Linux/Mac workspace jobs are still running at publication.
  This receipt-only follow-up keeps the tested Runtime pin and skips another CI
  run. Task-owned source/measurement binaries and the local Mac build cache are
  removed after the frozen evidence is verified; shared dependency caches remain.
