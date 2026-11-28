# Complete system encoding CPU acceptance — 2026-11-28

Paired Runtime `b627fdcd522a8de99ad556032ad3c0d5aa1b137c` closes every `0xD5xxxxxx` word under the existing fixed-state
CPU profile: 1,060 admitted words and 16,776,156 original unchanged-state faults.
All 128 HINT values now retain the existing disabled-extension/NOP profile,
correcting 98 formerly rejected values. Unknown/privileged system accesses use
native cached fault descriptors without entering the interpreter. No new host
control state, hot instruction check or guest-text rewrite is introduced.

The private opt-in probe exercises 128 HINTs, 64 timed waits and 10 rejected
accesses across two budgets and three repetitions (1,212 programs). Dirty GPR/FP
prefixes, whole CPU state, exact retirement/faults, unchanged code, zero fallback
retirement and zero interpreter decodes are required. Linux native and Apple
core JIT 10/default 1 checks, strict lint, feature isolation and notices pass.
JNI schema 1, 40-byte layout and flags remain unchanged.

Runtime additionally checks all 16,777,216 system words, dirty-bank/callback/
budget/reservation programs and cold/warm/invalidation paths. All 2,406 original
system code-generation programs remain byte-for-byte identical. Complete evidence
is in `runtime/docs/validation/jit-system-encoding-20261128.md`.

Windows remains the Android SDK/NDK/JDK/APK/device host. CPU acceptance does not
establish actual Android app-UID/ART or large-page-device acceptance. Stateful
hardware extensions and generic unowned instruction spaces remain active work.
