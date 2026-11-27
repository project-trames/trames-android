#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Check build-time backend selection; no runtime policy checks are added."""
import subprocess

def tree(*features):
    return subprocess.check_output(["cargo", "tree", "--locked", "-p", "trames-android-core", "--edges", "normal", "--prefix", "none", *features], text=True)
plain = tree("--no-default-features")
assert "trames-interpreter " in plain
assert "trames-jit " not in plain, "Default interpreter build must not compile JIT"
selected = tree("--features", "jit")
assert "trames-jit " in selected
print("Compile-time CPU selection verified: default interpreter, explicit optional JIT")
