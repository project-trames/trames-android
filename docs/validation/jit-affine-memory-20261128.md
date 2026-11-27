# Affine JIT memory acceptance — 2026-11-28

Paired Runtime: `1fa3f082b0760fae0a836a4b466cf5c15047febb`.

The native memory entry now keeps an admitted connected guest range and fixed
host bias in registers. Direct hits avoid indexed page-cache lookup. Cold
register-definition proofs reuse unchanged addresses, with verified same-block
invariant loops carrying the proof through budget/epoch guards. Helpers clear
the proof and refresh register borrows; holes, changed bases/indices/SP, wider
operands and cross-page accesses retain correct validation. Guest bytes/pointer
values and Android/JNI schema 1 / 40-byte flag ABI remain unchanged.

Flag 6 additionally requires exactly two direct accesses and a positive count
of compiled address-reuse sites in the existing 7-instruction probe. This uses
runtime-detected complete host page groups, not a fixed Android page size.
Linux ARM64 interpreter/JIT core probes and Clippy passed; Mac core, feature and
notices checks passed. Runtime release passed 7 native/capability plus 38
regression tests (45 total), including explicit host ABI canaries, changed-address
proofs, holes and a real host write fault on a reused post-index store. iOS
interpreter compilation/feature isolation and Apple scoped checks passed. Mac
does not execute native JIT. Shared-memory implementation tests are reused from
the preceding batch because only memory-contract documentation changed.

In the same balanced four-lane cohort on the existing nested ARM64 Linux/KVM VM,
the 1M hot loop changed from 1.489 ms to 1.214 ms (18.49% less time); sequential
512 KiB changed from 100.216 to 92.465 microseconds (7.73% less). Integer and
short-SVC medians did not regress in that cohort. This is CPU fixture evidence,
not general Android application or bare-metal performance. All 1,120 timed and
224 warm-up samples, checksums, source/binary hashes and six negative audit cases
are preserved in the Runtime record.

[Implementation, safety and complete measurements](https://github.com/project-trames/trames-runtime/blob/1fa3f082b0760fae0a836a4b466cf5c15047febb/docs/validation/jit-affine-memory-20261128.md).

Windows packaging CI is triggered by this paired implementation commit. Real
non-debuggable Android app-UID/ART/16 KiB execution still requires device tests.
No SDK/NDK/JDK/emulator was installed on Mac; task-owned temporary sources and
build/measurement outputs are cleaned after the evidence is frozen.
