// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit CPU acceptance; never part of instruction admission/execution.
use super::*;
pub(super) fn run() -> Result<(), String> {
    let words = (0..128)
        .map(|i| (true, 0xd503201f | (i << 5)))
        .chain((0..64).map(|i| (true, 0xd5031000 | i)))
        .chain(
            [
                0xd5000000, 0xd5034fdf, 0xd53b4500, 0xd51b4500, 0xd5381000, 0xd5181000, 0xd5080000,
                0xd5280000, 0xd5480000, 0xd5680000,
            ]
            .map(|w| (false, w)),
        );
    for (positive, word) in words {
        for budget in [1, 64] {
            let code = [0xb1000529u32, 0x1e622820, word, 0xd4000001];
            let mut a = Memory::default();
            a.map(0x1000, PAGE_SIZE, Permissions::RW)
                .map_err(|e| e.to_string())?;
            for (i, w) in code.iter().enumerate() {
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
            state.registers = std::array::from_fn(|r| (r as u64).wrapping_mul(0x123456789abcdef));
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
                        return Err(format!("system encoding {word:x}/{budget}: {x:?}/{y:?}"));
                    }
                    if !matches!(x, SliceOutcome::Yield { .. }) {
                        if positive {
                            if !matches!(x, SliceOutcome::Syscall { .. }) {
                                return Err("system hint faulted".into());
                            }
                        } else if !matches!(x,SliceOutcome::Fault(trames_engine::CpuFault::Instruction{pc:0x1008,instruction})if instruction==word)
                        {
                            return Err("unowned system instruction executed".into());
                        }
                        ended = true;
                        break;
                    }
                }
                if !ended
                    || e.stats.fallback_instructions != 0
                    || c.decode_stats().instructions != 0
                {
                    return Err("system encoding entered interpreter or lost progress".into());
                }
                for (i, w) in code.iter().enumerate() {
                    if a.fetch_u32(0x1000 + 4 * i as u64)
                        .map_err(|e| e.to_string())?
                        != *w
                    {
                        return Err("system encoding changed guest text".into());
                    }
                }
            }
        }
    }
    Ok(())
}
