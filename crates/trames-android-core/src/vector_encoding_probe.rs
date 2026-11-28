// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit CPU acceptance only; never used for native admission or execution.
use super::*;
const RESERVED: &[u32] = &[
    0x2e20e020, 0x5f088420, 0x5f088c20, 0x5f208420, 0x5f208c20, 0x5f402020, 0x5f402820, 0x5f406020,
    0x5f406820, 0x5f40a020, 0x5f40a820, 0x5f602020, 0x5f602820, 0x5f606020, 0x5f606820, 0x5f60a020,
    0x5f60a820, 0x6e20e020, 0x7f402020, 0x7f402820, 0x7f406020, 0x7f406820, 0x7f40a020, 0x7f40a820,
    0x7f602020, 0x7f602820, 0x7f606020, 0x7f606820, 0x7f60a020, 0x7f60a820,
];
fn neighbor(word: u32) -> u32 {
    if word & 0x0f000000 == 0x0e000000 {
        word & !(1 << 29)
    } else if word & (1 << 10) != 0 {
        word | (1 << 29)
    } else {
        word & !(1 << 28)
    }
}
pub(super) fn run() -> Result<(), String> {
    for &reserved in RESERVED {
        for word in [reserved, neighbor(reserved)] {
            let positive = word != reserved;
            let host = <Cpu as Backend>::features();
            if positive && word & 0xbfe0fc00 == 0x0ee0e000 && host.hwcap & (1 << 4) == 0 {
                continue;
            }
            for budget in [1, 64] {
                let words = [0xb1000529u32, 0x1e622820, word, 0xd4000001];
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
                    fpsr: 0x10,
                    tpidr_el0: 123,
                    tpidr2_el0: 456,
                    vectors: [u128::MAX / 3; 32],
                    ..Default::default()
                };
                state.registers =
                    std::array::from_fn(|i| (i as u64).wrapping_mul(0x123456789abcdef));
                let mut c = Cpu::new(0, 0);
                let mut r = trames_interpreter::Cpu::new(0, 0);
                let mut e = <Cpu as Backend>::Engine::default();
                for _ in 0..3 {
                    c.restore_architectural_state(&state);
                    r.restore_architectural_state(&state);
                    let mut ended = false;
                    for _ in 0..8 {
                        let x = c.run(&mut e, &mut a, budget);
                        let y = r.run_slice(&mut b, budget);
                        if format!("{x:?}") != format!("{y:?}")
                            || c.architectural_state() != r.architectural_state()
                            || c.fault_retired() != r.fault_retired()
                        {
                            return Err(format!(
                                "vector decode probe {word:x}/{budget}: {x:?}/{y:?}"
                            ));
                        }
                        if !matches!(x, SliceOutcome::Yield { .. }) {
                            if positive {
                                if !matches!(x, SliceOutcome::Syscall { .. }) {
                                    return Err("allocated vector neighbor faulted".into());
                                }
                            } else if !matches!(x,SliceOutcome::Fault(trames_engine::CpuFault::Instruction{pc:0x1008,instruction})if instruction==word)
                            {
                                return Err("reserved vector encoding executed".into());
                            }
                            ended = true;
                            break;
                        }
                    }
                    if !ended || e.stats.fallback_instructions != 0 {
                        return Err("vector decode progress/fallback failed".into());
                    }
                    for (i, w) in words.iter().enumerate() {
                        if a.fetch_u32(0x1000 + 4 * i as u64)
                            .map_err(|e| e.to_string())?
                            != *w
                        {
                            return Err("vector decode changed guest text".into());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
