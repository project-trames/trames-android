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

The first compiler translates whitelisted non-flag integer instructions, ADR/P,
B/BL and NOP. Original guest memory is RX data, never patched. Other instructions
run through the canonical interpreter, retaining deferred flags, guest TLS,
FP/SIMD and exclusive reservations. Each worker owns a bounded RW-to-RX cache;
invalidation reuses Runtime code-page identities. Native blocks are at most 64
instructions and stay within one 4 KiB guest page. Host x18/TLS are untouched.

Within each integer block, compile-time register allocation keeps values in
caller-saved host registers. Only initial reads, dirty evictions and final dirty
writebacks access the guest register frame. Host x18/TLS and callee-saved state
are preserved. Guest memory operations still use the canonical interpreter.

Guest SVC is decoded during compilation and terminates its block. After dirty
registers are published, the backend advances guest PC once and returns
`SliceOutcome::Syscall` directly; no guest SVC opcode is emitted into the host
code cache. The shared Session dispatcher reads x8/x0-x5 and owns syscall policy,
blocking/retry behavior and return values. Android currently validates this exit
with its CPU probe; full Session/terminal integration is still pending. Probe
flag bit 3 proves a direct compiled exit without interpreter fallback.

Translated guest memory accesses, conditional control flow and block linking
remain later optimizations. CPU-only exit benchmarks do not include Linux syscall
handler, filesystem/network, JNI or Android lifecycle costs.

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
