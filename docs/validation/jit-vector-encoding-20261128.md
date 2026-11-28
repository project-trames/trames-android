# FP/AdvSIMD encoding category CPU acceptance — 2026-11-28

Paired Runtime `7d74ff5809364b8e41081f507da2cf1d6e7a661b` closes both fixed-width FP/AdvSIMD primary groups, including direct
instruction-error caching and three shared decoder corrections. Independent LLVM
and actual ARM64 CPU execution reject all 1,776 corrected reserved words.

The private opt-in probe covers 30 reserved opcode/width selector families and
30 independently checked allocated neighbors. Each runs budgets 1/64 and three
cold/warm entries after dirty integer/FP prefixes, comparing complete reference
state, exact faults/retirement, unchanged text and zero interpreter fallback.
On the current host that is 360 programs. The same JNI schema 1, 40-byte layout
and flag meanings are retained. No admission probe or per-instruction JNI crossing
is added. Pure interpreter/iPad CPU graphs still exclude executable allocation.

Linux native and Apple checks pass: default core 1, explicit JIT 8 tests and
strict scoped lint; feature selection and notice checks pass. Shared Runtime's
full CPU report is `runtime/docs/validation/jit-vector-encoding-20261128.md`.
ISA-model results, exact source/artifact identities and remaining hardware scope
are recorded there. Linux native and independent model acceptance do not establish
Android app-UID/ART or large-page-device acceptance. SDK/NDK/JDK, APK and device
work remain on Windows. The generic interpreter entry outside the audited scalar,
FP/AdvSIMD and control/exception spaces remains explicit work.
