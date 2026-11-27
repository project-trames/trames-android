# Execution ownership and next milestones

The interpreter and pure computation helpers live in Runtime as independent
crates. Android has no dependency on the iPad repository. A separate interpreter
repository is unnecessary: CPU, memory contract and Linux runtime can evolve
under a single Runtime pin. The iPad compatibility crates keep their public API.

`trames-android-core` has no default JIT feature. Explicit `--features jit` adds
the optional `trames-jit` dependency and its `native-aarch64` feature. Native
emission is further gated to AArch64 Linux/Android at compile time. iPad depends
only on `trames-interpreter`: neither the JIT crate nor executable cache is in
its compilation graph. This isolation adds no ordinary runtime capability test.

The compiler translates baseline integer arithmetic/logic, SP/NZCV forms,
carry/conditional operations, multiply/divide, bitfields, shifts and fixed,
conditional and register branches. CRC/FlagM/FlagM2/CSSC emit actual host opcodes
when process-host HWCAP provides them. Native capability advertisement is now
host-driven; there are no new software extension replacements or extra emulated
extension state. Original guest memory is RX data, never patched. Remaining
instruction families retain the legacy interpreter path, including FP/SIMD,
pairs and exclusive reservations. Each worker owns a bounded RW-to-RX cache;
invalidation reuses Runtime code-page identities. Native blocks are at most 64
instructions and stay within one 4 KiB guest page. Host x18/TLS are untouched.

Within each integer block, compile-time register allocation keeps values in
caller-saved host registers. Only initial reads, dirty evictions and final dirty
writebacks access the guest register frame. Host x18/TLS and callee-saved state
are preserved. Scalar LDR/STR use exact-width native fragments inside the existing
checked Memory/View borrow, now fused with surrounding integer instructions and
same-block loops when the embedding exposes instruction epochs. Helper boundaries
publish dirty registers and precise fault progress, then refresh register borrows.
Each guest access keeps its independent data borrow; no host data pointer is cached.
Ordinary native links never jump into the larger helper-frame ABI. Probe flag bit 6
requires a real load/add/store/SVC round trip, two direct native accesses, zero
fallback and one fused entry. JNI remains schema 1 / 40 bytes.
`Engine::memory_stats()` reports native accesses, staging and fused entries/blocks.

Guest SVC is decoded during compilation and terminates its block. After dirty
registers are published, the backend advances guest PC once and returns
`SliceOutcome::Syscall` directly; no guest SVC opcode is emitted into the host
code cache. The shared Session dispatcher reads x8/x0-x5 and owns syscall policy,
blocking/retry behavior and return values. Android currently validates this exit
with its CPU probe; full Session/terminal integration is still pending. Probe
flag bit 3 proves a direct compiled exit without interpreter fallback.

Same-page fixed-target blocks now use stable worker-private RW jump slots.
Generated instructions stay RX and original guest bytes stay unchanged. Budget
checks and existing live code epochs guard native chains; generation handles
revoke incoming edges before eviction. Terminal SVC blocks keep a lightweight
entry, and stable ordinary-block-to-SVC pairs can tail-jump without a full chain
frame. Every SVC still returns to Runtime before further guest execution.

Post-SVC re-entry caches the continuation address when code identity, guest PC
and budget still match. Signal/restart redirection and unknown memory embeddings
fall back to lookup. Probe flag bit 4 requires both real native links and cached
syscall resumption.

Conditional paths publish their actual guest PC and follow a native link only
when it matches the resolved target. Probe flag bit 5 requires a zero-fallback,
linked SUBS/B.NE countdown with exact retirement and NZCV. The JNI ABI remains
40 bytes. The backend reuses existing task SP/NZCV state and never changes host
SP or TLS; capability gates run during translation, not per instruction.

Full host-ISA native coverage remains in progress. Remaining memory forms, atomics and
FP/NEON/vector families are the next groups; stateful extensions need real
context-save ownership. CPU-only benchmarks exclude Linux syscall handlers,
filesystem/network, JNI and Android lifecycle costs.

Next milestones:

1. Run the non-debuggable APK on a normal device app UID (4/16 KiB hosts),
   validate executable-memory policy and coexistence with ART signal handling.
2. Connect the shared session/Linux runtime through Android app-private storage,
   terminal I/O, lifecycle and network adapters. Guest ELF remains data.
3. Expand JIT blocks using the same memory and fault model, differential tests,
   exact stepping, self-modification and syscall/native-call boundary checks.
4. Add Vulkan/ANativeWindow/AHardwareBuffer, audio and MediaCodec adapters after
   CPU/session acceptance. Keep product capabilities and performance measurable.

pKVM/root/KVM devices and a separate guest kernel are not required for this path.

CPU-only measurements and source-bound validation for this optimization are
recorded in [Runtime's JIT report](https://github.com/project-trames/trames-runtime/blob/f10a1597574d3b97b5d72b6c7d1d9d4a18ae5c8b/docs/validation/jit-syscall-registers-20261127.md).

Same-page linking design and measured CPU-only before/after results are recorded
in [Runtime's linking report](https://github.com/project-trames/trames-runtime/blob/eefc3b2d9efa46899d68248cae1249a2b2add349/docs/validation/jit-links-20261127.md).

Host-driven scalar coverage, reached-PC SIGILL and frozen same-VM performance
results are recorded in [Runtime's scalar report](https://github.com/project-trames/trames-runtime/blob/02cc4833df6c140f4468277a970858e0cd1f08b2/docs/validation/jit-host-scalars-20261127.md).

The scalar memory coverage and full before/after CPU-only cohort are recorded in
[Runtime's memory report](https://github.com/project-trames/trames-runtime/blob/d764fc7b920aa73e31b7a91c88512b36c349c5eb/docs/validation/jit-memory-20261127.md).

Mixed integer/memory block fusion, precise helper exits and the complete
before/after cohort are recorded in [Runtime's fusion report](https://github.com/project-trames/trames-runtime/blob/5ca460c401745c4a631bbeacbb829e687598231f/docs/validation/jit-memory-fusion-20261127.md).

The memory probe requires native integer paired transfers, LDAR/STLR and
DMB/DSB/ISB, then adds LSE CAS/CASP/LDADD and RCpc/unscaled accesses when the
process-host ABI provides those extensions. It requires zero fallback/staging,
correct canonical bytes, unchanged guest code and actual hardware/direct hits
for every warmed access. It keeps schema 1, the 40-byte JNI layout and flag bit 6.
Native hits have no new software alignment preflight, ordinary fence or global
lock; ISA admission and reserved-register allocation run in the cold compiler.
The full paired measurements and platform checks are recorded in
[Runtime's LSE/RCpc report](https://github.com/project-trames/trames-runtime/blob/a4fe0eb0f744987bd0e8b315567ce3663aaff0c1/docs/validation/jit-lse-rcpc-20261128.md).
