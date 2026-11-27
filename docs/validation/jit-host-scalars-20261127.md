# Host-driven JIT scalar acceptance — 2026-11-27

Runtime pin: `f3f45e56c02f020d9ac2e7792f1a1550f1ee8286`, preserving the published
`10dcf23` perf integration. This batch follows the owner correction: host ISA
capabilities drive optional instructions; no new software extension replacements
or extra persistent register model are added.

Local actual ARM64 Linux passes 26 JIT tests, strict JIT/core Clippy and the
Android CPU probe, including the new linked zero-fallback SUBS/B.NE probe.
macOS passes 22 non-native differential tests, scoped Clippy, default/core-JIT
probe builds and compile-time feature isolation. The 40-byte JNI ABI is unchanged;
instrumentation now also requires flag bit 5. No Android tools were installed
on the Mac. [Android CI 46009219067](https://github.com/project-trames/trames-android/actions/runs/46009219067)
passes all three jobs: Windows interpreter/JIT packaging, native ARM64 software
CPU and macOS software CPU. Real Android app-UID/device execution remains pending.

That CI source is `fec3826` / Runtime `02cc483`. The final Runtime pin adds only
an existing perf cgroup declaration-order/spacing repair; Android core's normal
JIT dependency graph has no trames-vfs, and its compiled source is unchanged.
The successful packaging/CPU checks are reused. Runtime's dedicated native JIT
job also passes in run 46009057483; global verification of the cgroup CI fix is
separate from that unchanged JIT source.

The controlled same-Runtime-base countdown benchmark improves about **7.9x**:
new JIT is 0.215–0.216 ms per million ADDs versus KVM's approximately 0.224 ms.
This is CPU-only nested-VM throughput with a validated CBNZ countdown, not whole
application/syscall performance. The actual VM has CRC/FlagM/FlagM2 and lacks
CSSC; missing CSSC produces precise SIGILL instead of software execution.

Full host-ISA native coverage is not complete. Checked memory/atomics, FP/NEON,
crypto/vector and remaining families still need native lowering and acceptance.
Stateful extensions wait for their real context-save path; no parallel software
extension model is planned.

[Detailed scope, all frozen samples and source/build receipts](https://github.com/project-trames/trames-runtime/blob/02cc4833df6c140f4468277a970858e0cd1f08b2/docs/validation/jit-host-scalars-20261127.md).
