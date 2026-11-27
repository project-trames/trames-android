#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Check build-time backend selection; no runtime policy checks are added."""
import subprocess

def tree(*features):
    return subprocess.check_output(["cargo", "tree", "--locked", "-p", "trames-android-core", "--edges", "normal", "--prefix", "none", "--format", "{p} features={f}", *features], text=True)
plain = tree("--no-default-features")
assert "trames-interpreter " in plain
assert "trames-jit " not in plain, "Default interpreter build must not compile JIT"
assert all("native-code" not in line for line in plain.splitlines()
           if line.startswith("trames-memory ")), "Interpreter must not compile dynamic native fixups"
selected = tree("--features", "jit")
assert "trames-jit " in selected
assert any("native-code" in line for line in selected.splitlines()
           if line.startswith("trames-memory ")), "JIT must enable protected native operands"
print("Compile-time CPU selection verified: default interpreter, explicit optional JIT")
