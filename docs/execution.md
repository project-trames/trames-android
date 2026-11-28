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
special-memory and non-hint PAC/MTE compatibility operations retain the shared
interpreter path where admitted. Fixed-width SIMD/FP, stateless crypto, common
scalar/vector memory and LSE, system controls and exclusive reservations now
execute natively. Unadvertised or absent host extensions are not supplied by
software emulation. Each worker owns a bounded RW-to-RX cache;
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

Full host-ISA native coverage remains in progress. Special memory (MOPS, LOR
and explicitly owned LSE128/LS64 profiles) and non-hint PAC/MTE compatibility
operations remain. Stateful extensions need real context-save ownership and
remain unadvertised. CPU-only benchmarks exclude Linux syscall handlers,
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

The native CPU probe also requires FP/NEON arithmetic and integer conversion,
real V31, preserved canonical vector values, zero fallback and one native entry.
Scalar FP16 arithmetic is included when admitted by the host. Register allocation
runs in the cold compiler; existing host FP control/status is switched once per
entry. Vector memory fusion now extends this path; remaining SIMD families are
still pending.
[FP/NEON implementation and measurements](https://github.com/project-trames/trames-runtime/blob/05c4fc1d1065d1d8fdde3e1fd52a9ce90a31b89d/docs/validation/jit-fp-neon-20261128.md).


The vector-memory probe executes Q LDR/STR and Q LDP/STP loops with native NEON
ADD, including V31. Flag bit 6 now also requires complete canonical lane values,
exact retirement, zero fallback/staging, at least 62/64 direct hardware accesses,
address-proof reuse and one fused entry per loop. The mixed compiler handles
B/H/S/D/Q scalar transfers and S/D/Q pairs with the existing memory contract.
Hardware hits stay inside the original FP scope; checked callbacks publish live
registers and restore host FP control/status before entering Rust, then reload
guest banks/status on return. JNI schema and feature flags remain unchanged.
[Vector-memory implementation and measurements](https://github.com/project-trames/trames-runtime/blob/367358df0a0eb25394a4af3146d4ce0d7950690e/docs/validation/jit-vector-memory-20261128.md).


The structure probe adds four-register LD1/ST1, interleaved LD4/ST4, LD4/ST4
single lanes and LD4R broadcast. Each loop wraps V31 to V0, performs native NEON
arithmetic, and verifies all output bytes and vectors, exact 257-instruction
retirement, zero fallback/staging, at least 62/64 hardware/direct accesses,
address-proof reuse and one fused entry. The compiler also covers LD/ST2/3,
one-to-four-register lists, all B/H/S/D lane indices, LD1R..LD4R, and both immediate
and X-register post-index writeback. Contiguous host-vector groups are arranged
in the cold compiler. Native transfer/permute/broadcast behavior remains hardware
owned. JNI schema/flags, iPad isolation and Windows-only Android tooling stay
unchanged. [Runtime validation](https://github.com/project-trames/trames-runtime/blob/f66050968f3433d89698e87bf7d3d477f28e38b7/docs/validation/jit-structures-20261128.md).


The category probe now also requires the complete baseline SIMD copy/immediate/
permute category and every stateless crypto family admitted by the process host.
It compares the complete result to an independent interpreter, exact retirement,
zero fallback, one native entry and unchanged guest text. TBL/TBX use cold packed
register groups, including V31 wrap; insertion and destructive crypto instructions
retain the old destination. No per-instruction JNI bridge or new CPU state is
introduced. Missing optional ISA is rejected in cold compilation, including
untranslated BF16/FHM/FCMA/DotProd/I8MM/RDM/FP16/JSCVT/FRINT forms. LS64 waits for
an owned HWCAP3/device-memory contract. See the
[category report](https://github.com/project-trames/trames-runtime/blob/76c662beea2aaa93f074506a2b7fd8972f0d7c3a/docs/validation/jit-move-crypto-20261128.md) and
[remaining category plan](https://github.com/project-trames/trames-runtime/blob/76c662beea2aaa93f074506a2b7fd8972f0d7c3a/docs/roadmap/native-jit-coverage.md).


The system category probe now also requires guest TLS/TLS2, FPCR/FPSR updates
and clearing, the guest's exact 64-byte DC ZVA block, translated cache maintenance,
barriers/hints, all four counter aliases, and cached native-call/trap exits. It
checks complete reference state/data, exact retirement, zero interpreter entry
and unchanged guest code. Current call opt-ins are checked at the reached typed
boundary. There is no per-instruction JNI transition or new extension state.

On ARM64 Linux/Android the software backends now share actual CNTVCT/CNTFRQ;
ordinary JIT counter reads are direct hardware MRS and synchronized aliases
retain ordering. This replaces the artificial 1-GHz Linux software timer and
allows coherent calibration of Runtime's existing optional Linux vDSO. Apple/
iPad keep their portable counter and interpreter-only dependency graph. The VM
acceptance uses a 24-MHz counter; frequency is not hard-coded in production.
[System category validation](https://github.com/project-trames/trames-runtime/blob/1ef4b02bdc53059abd15aadbba3f2dc67dea32ab/docs/validation/jit-system-20261128.md)
records Linux native and Apple isolation results separately from Windows Android
builds and outstanding app-UID/ART/16-KiB-device execution.
At the system-category pin, the [coverage plan](https://github.com/project-trames/trames-runtime/blob/1ef4b02bdc53059abd15aadbba3f2dc67dea32ab/docs/roadmap/native-jit-coverage.md)
listed arithmetic and exclusives before the subsequent category completions
below. That pin did not claim full native instruction coverage.


The fixed-width arithmetic category is now native: scalar SIMD, widening/
narrowing and saturation, shifts/inserts, indexed multiply/add, reductions,
reciprocal families, precision/scaled conversions, FP16/BF16/DotProd/I8MM/RDM/
FCMA/FHM, JSCVT and FRINT32/64. All feature/encoding and register-allocation
choices stay in the cold compiler. The existing guest vector bank and FP state
are reused, with 24 caller-saved host vector registers and real low-register
allocation for restricted indexed operands; no software ISA or new extension
state is introduced. Host D8–D15, TLS, guest code and the native scalar ABI are
preserved.

The Android native arithmetic probe fills all 32 guest vectors, executes baseline
and host-admitted extension families, then crosses checked memory callbacks. It
requires complete interpreter-reference state/data, exact retirement and zero
fallback, without changing JNI schema/flags. Runtime's independent encoding
matrix has 68,214 admitted encodings, zero invalid admissions and no valid
unadmitted forms in its declared fixed-width classes. The broader suite executes
10,640 distinct forms and 7,129 aliased programs under pressure and FP controls.
[Arithmetic validation](https://github.com/project-trames/trames-runtime/blob/767b92efb81251861576cefdf676a8bb4f15a4e3/docs/validation/jit-arithmetic-20261128.md)
separates Linux-native, Apple/iPad isolation, Windows Android builds and outstanding
app-UID/ART/16-KiB-device acceptance.

The [remaining plan](https://github.com/project-trames/trames-runtime/blob/08bc623868ab03599c5bd4be9b2b88c0874b71be/docs/roadmap/native-jit-coverage.md)
now covers special memory, non-hint compatibility paths and stateful ABI work.
SVE/SME and other unowned extensions are not software-emulated or advertised.

The exclusive-memory category is now native at every single/pair width:
LDXR/LDAXR/LDXP/LDAXP, STXR/STLXR/STXP/STLXP and every CLREX option. Closed
private windows keep the hardware monitor across native GPR lowering with no
helper/frame traffic between LR and SC. Shared/open/sliced sequences use native
fragments through the original Runtime reservation/ABA domain. The existing guest
alignment constraint is admitted once at the window loan boundary; capability,
encoding and allocation decisions remain cold. No additional extension state,
global lock, JNI crossing or guest-text rewrite is introduced.

The Android native probe now checks all single/pair widths, guest retries, exact
retirement, native CLREX and a fresh-engine sliced handoff with zero fallback and
unchanged code. JNI schema/flags stay unchanged. Runtime's
[exclusive category validation](https://github.com/project-trames/trames-runtime/blob/08bc623868ab03599c5bd4be9b2b88c0874b71be/docs/validation/jit-exclusive-20261128.md)
records 114 Linux Debug/Release JIT tests, independent LLVM encodings, exact real
fault recovery and matched CPU-only measurements. Private single/pair retry loops
are about 3.79×/3.85× faster than the previous JIT's interpreter-exclusive path;
guarded shared loops are about 1.22× faster. Android app-UID/ART and 16-KiB device
acceptance remain distinct from these Linux/Apple CPU probes.

The paired Runtime CI follow-up fixes a DNS reclaim test lifetime race and
restores the memory helper's `native-code` cfg. Linux GNU, the CI musl GNU-C
frontend and Apple each pass 213 network library tests; the iOS interpreter
check passes. CPU/JIT implementation and Android probes are unchanged, so their
passing acceptance is reused. See the [scoped validation](https://github.com/project-trames/trames-runtime/blob/f595ae2a78e81a316fb842dd093c8c5c1eee005a/docs/validation/jit-exclusive-ci-followup-20261128.md).
