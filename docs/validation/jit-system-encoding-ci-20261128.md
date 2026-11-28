# System encoding CI format repair — 2026-11-28

Paired Runtime `cec48f55a166cb21134889d1b082ea8a7fe108d3` sorts the stream module declaration that failed the
final merged workspace formatting check. Actual Android and Runtime workspaces
both pass `cargo fmt --all --check`. Existing CPU acceptance and JNI schema are
unchanged; normal remote CI verifies the new paired pin.
