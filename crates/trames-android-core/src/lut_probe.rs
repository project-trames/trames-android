// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit CPU acceptance; no per-instruction JNI or feature probe.
use super::*;
const FORMS: &[u32] = &[
    0x4e801000, 0x4e803000, 0x4e805000, 0x4e807000, 0x4ec00000, 0x4ec01000, 0x4ec02000, 0x4ec03000,
    0x4ec04000, 0x4ec05000, 0x4ec06000, 0x4ec07000, 0x4e402000, 0x4e406000, 0x4e401000, 0x4e403000,
    0x4e405000, 0x4e407000,
];
pub(super) fn run() -> Result<(), String> {
    let host = <Cpu as Backend>::features();
    let available = host.hwcap & 2 != 0 && host.hwcap2 & (1 << 49) != 0;
    for &base in FORMS {
        for (d, n, reg) in [(0, 1, 2), (31, 31, 0), (18, 18, 18)] {
            let word = (base & !0x1f03ff) | (reg << 16) | (n << 5) | d;
            for pattern in [0u8, 0x55, 0xaa, 0xff, 0x1b, 0xe4, 0x80, 0x7f] {
                for budget in [1, 64] {
                    let words = [0xb1000529u32, 0x1e622820, word, word, 0xd4000001];
                    let mut a = Memory::default();
                    a.map(0x1000, PAGE_SIZE, Permissions::RW)
                        .map_err(|e| e.to_string())?;
                    for (i, w) in words.iter().enumerate() {
                        a.write_array(0x1000 + 4 * i as u64, w.to_le_bytes())
                            .map_err(|e| e.to_string())?;
                    }
                    a.protect(0x1000, PAGE_SIZE, Permissions::RX)
                        .map_err(|e| e.to_string())?;
                    let mut b = a.clone();
                    let mut state = trames_execution::Aarch64State {
                        pc: 0x1000,
                        sp: 0x2000,
                        nzcv: 0xb0000000,
                        fpsr: 0x08000010,
                        tpidr_el0: 123,
                        tpidr2_el0: 456,
                        vectors: [u128::MAX / 3; 32],
                        ..Default::default()
                    };
                    state.registers =
                        std::array::from_fn(|i| (i as u64).wrapping_mul(0x123456789abcdef));
                    state.vectors[reg as usize] = u128::from_le_bytes([pattern; 16]);
                    let mut c = Cpu::new(0, 0);
                    let mut r = trames_interpreter::Cpu::new(0, 0);
                    c.restore_architectural_state(&state);
                    r.restore_architectural_state(&state);
                    let mut e = <Cpu as Backend>::Engine::default();
                    let mut ended = false;
                    let mut prefix = 0;
                    if !available
                        && !matches!(
                            r.run_slice(&mut b, 2),
                            SliceOutcome::Yield { instructions: 2 }
                        )
                    {
                        return Err("LUT reference prefix failed".into());
                    }
                    for _ in 0..8 {
                        let actual = c.run(&mut e, &mut a, budget);
                        if available {
                            let expected = r.run_slice(&mut b, budget);
                            if format!("{actual:?}") != format!("{expected:?}")
                                || c.architectural_state() != r.architectural_state()
                                || c.fault_retired() != r.fault_retired()
                            {
                                return Err(format!(
                                    "LUT {word:x}/{budget}: {actual:?}/{expected:?}"
                                ));
                            }
                            if !matches!(actual, SliceOutcome::Yield { .. }) {
                                if !matches!(actual, SliceOutcome::Syscall { .. }) {
                                    return Err("LUT valid instruction faulted".into());
                                }
                                ended = true;
                                break;
                            }
                        } else if let SliceOutcome::Yield { instructions } = actual {
                            prefix += instructions;
                        } else {
                            if !matches!(actual,SliceOutcome::Fault(trames_engine::CpuFault::Instruction{pc:0x1008,instruction})if instruction==word)
                                || c.architectural_state() != r.architectural_state()
                                || prefix + c.fault_retired() != 2
                            {
                                return Err(format!(
                                    "LUT absent-host contract {word:x}: {actual:?}"
                                ));
                            }
                            ended = true;
                            break;
                        }
                    }
                    if !ended || e.stats.fallback_instructions != 0 {
                        return Err("LUT progress/fallback mismatch".into());
                    }
                    for (i, w) in words.iter().enumerate() {
                        if a.fetch_u32(0x1000 + 4 * i as u64)
                            .map_err(|e| e.to_string())?
                            != *w
                        {
                            return Err("LUT backend changed text".into());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
