# Native memory fusion acceptance — 2026-11-27

Runtime pin: `89c5abd57ce5cc8341f17fcea35a339b0f991595` (comment-only child of the
measured/tested implementation `5ca460c401745c4a631bbeacbb829e687598231f`).

Mixed scalar memory/integer blocks now stay within the native entry, including
same-block backedges through budget/epoch guards. Each LDR/STR retains its own
existing Memory/View borrow; no guest code rewrite, cached host data pointer,
algorithm lock, coordinator or extra extension state is introduced. Typed helper
boundaries refresh CPU register borrows and publish exact dirty-prefix faults
without replay or inaccurate retirement.

Android's flag 6 probe now requires load/add/store/SVC to finish in **one fused
entry**, with two direct native memory operations, zero staging and zero fallback.
JNI remains schema 1 / 40 bytes. This strengthened probe passes on real ARM64 Linux;
macOS core tests and the default/optional-JIT feature graph also pass. No Android
tools were installed on Mac. Windows packaging is checked by the paired CI run;
actual Android app-UID/ART and 4/16 KiB device execution remain pending.

ARM64 Linux/4096-byte pages passes **36 JIT tests** and strict all-target Clippy.
macOS passes 31 differential cases and core JIT tests. Added cases cover every
budget cut in mixed loops, carry flags across helpers, Memory/View state, precise
faults before/after successful stores, repeated faults without replay and concurrent
code changes interrupting an active fused loop. Shared Memory/interpreter sources
and iPad dependency graphs are unchanged, so their preceding passing validation
is reused. The previous Runtime run ultimately passed its Mac and Linux jobs.

Five alternating old/new process pairs use the unchanged CPU fixture and frozen
release binaries. Scalar-memory-loop median improves from **4.462 to 1.854 ms
(2.41x)**; the cached interpreter remains faster at **0.931 ms (about 1.99x)**.
This is whole-loop overhead, not hardware LDR/STR latency: the previous JIT paid
main-dispatch/identity/register boundaries around tiny native fragments, while the
interpreter already had decoded tail dispatch and fixed-width memory access.
Fusion removes much of that cost; per-access helper/fragment work still remains.

SVC-only latency is 9.1% slower, continuation 10.5% slower and round-trip 6.3%
slower in the same final cohort; these regressions remain explicit follow-ups.
The CPU-only measurements exclude actual syscall handlers, JNI and Android
lifecycle. No host/KVM or application-level ranking is inferred.

[Exact Runtime scope, safety, all samples, fixture and source/binary fingerprints](https://github.com/project-trames/trames-runtime/blob/5ca460c401745c4a631bbeacbb829e687598231f/docs/validation/jit-memory-fusion-20261127.md).

CI receipts preserve the tested source identities; the final helper-contract
comment and this paired record do not rerun code validation:

- Runtime `5ca460c`: [ARM64 software-JIT job](https://github.com/project-trames/trames-runtime/actions/runs/46025036145/job/140594120305)
  passes Clippy, all 36 JIT tests and the CPU fixture. Broader Linux/Mac workspace
  jobs are still running when this record is published.
- Android `59e0766`: [paired run 46025174090](https://github.com/project-trames/trames-android/actions/runs/46025174090)
  passes ARM64 native CPU acceptance and Windows packaging. The Mac runner is
  still queued at publication; the queued job is not counted as a pass.
- Local Mac CPU/core checks and the actual ARM64 core probe above already pass.
  Task-owned source/build caches were removed after freezing the evidence.
