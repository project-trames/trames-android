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

The current translation strategy spills integer operands to a register frame.
It demonstrates correctness and a reusable backend boundary; it is not a
near-native performance claim. Register allocation, translated memory access,
conditional control flow and block linking are later measured optimizations.

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
