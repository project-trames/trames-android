# Native branch/call CPU acceptance — 2026-11-28

Paired Runtime `d251bc26eac982314a6872dcec1ab939c001438e` adds cold host-admitted HBC/CMPBR native instructions and closes
owned control/exception fault selectors without interpreter entry. Existing
B/BL, B.cond, CBZ/CBNZ, TBZ/TBNZ, BR/BLR/RET and PAC compatibility remain covered.

The new opt-in private CPU probe covers 244 representative encodings over all
16 NZCV combinations (3,904 programs): complete GPR/vector/FP/TLS/PC/LR state,
actual host admission, exact retirement and unchanged text. It compares the
reference interpreter only during the explicit probe; no per-instruction/JNI
bridge is introduced. Schema 1, 40-byte layout and flag meanings are unchanged.

Linux native and Apple compile-time fallback checks pass: default core 1 test,
explicit JIT 7 tests, strict scoped lint, feature-graph and notice checks. Shared
Runtime passes scoped Debug/Release, Apple/iOS and cross-target checks; its
source-bound detailed report is `runtime/docs/validation/jit-control-20261128.md`.
QEMU max passes the Android category probe with positive HBC; CMPBR is absent
there. This is ISA-model acceptance, not positive hardware speed.

CMPBR-positive execution, positive HBC hardware, Android app-UID/ART and real
large-page devices remain acceptance work. SDK/NDK/JDK/APKs stay on Windows.
The generic unrecognized-encoding interpreter entry outside audited scalar and
owned control/exception spaces remains explicit; fault-only entries are not
proved absent merely by a zero retired-fallback counter. Full stateful host
extensions remain unadvertised until their real shared ABI ownership exists.
