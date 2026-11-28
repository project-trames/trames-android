// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use trames_execution::Aarch64State;

fn memory(words: &[u32]) -> Result<Memory, String> {
    let mut m = Memory::default();
    m.map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    for (i, w) in words.iter().enumerate() {
        m.write_array(0x1000 + 4 * i as u64, w.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    m.protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    Ok(m)
}
fn state() -> Aarch64State {
    let mut state = Aarch64State {
        pc: 0x1000,
        sp: 0x2000,
        nzcv: 0xb0000000,
        fpsr: 0x10,
        tpidr_el0: 123,
        tpidr2_el0: 456,
        vectors: [u128::MAX / 3; 32],
        ..Default::default()
    };
    state.registers = std::array::from_fn(|i| u64::MAX.wrapping_sub(i as u64 * 0x123456789));
    state.registers[9] = u64::MAX;
    state.vectors[1] = 1.5f64.to_bits() as u128;
    state.vectors[2] = 2.25f64.to_bits() as u128;
    state
}
fn available(word: u32) -> bool {
    let host = <Cpu as Backend>::features();
    if word & 0x7fe0e000 == 0x1ac04000 {
        host.hwcap & (1 << 7) != 0
    } else if word & 0x7fe0f000 == 0x1ac06000
        || word & 0x7ff00000 == 0x11c00000
        || matches!(word & 0x7ffffc00, 0x5ac01800 | 0x5ac01c00 | 0x5ac02000)
    {
        host.hwcap2 & (1 << 34) != 0
    } else if matches!(word, 0xd500403f | 0xd500405f) {
        host.hwcap2 & (1 << 7) != 0
    } else if word == 0xd500401f
        || word & 0xffe07c10 == 0xba000400
        || word & 0xffffbc1f == 0x3a00080d
    {
        host.hwcap & (1 << 27) != 0
    } else {
        true
    }
}
pub(super) fn run() -> Result<(), String> {
    let mut matrix = vec![
        0x0b200020, 0x11000020, 0x0b000020, 0x10000000, 0x1ac00820, 0x13000020, 0x1a000020,
        0x3a400020, 0x1ac04020, 0x13800020, 0x3a00082d, 0xd500401f, 0x12000020, 0x0a000020,
        0x11c00020, 0x12800000, 0x1b000020, 0xd53b4200, 0x1a800020, 0x5ac00020, 0xd51b4200,
        0xd500403f, 0xd500405f, 0xba00042f, 0x3a00482d, 0x5ac01820, 0xdac01c20, 0xdac02020,
    ];
    for base in [0x9b400020, 0x9bc00020] {
        for ignored in 0..32 {
            matrix.push(base | (ignored << 10));
        }
    }
    for word in matrix.into_iter().filter(|&word| available(word)) {
        for budget in [1, 3, 64] {
            let words = [0xb1000529, 0x1e622820, word, word, 0xd4000001];
            let mut a = memory(&words)?;
            let mut b = memory(&words)?;
            let mut c = Cpu::new(0, 0);
            let mut r = trames_interpreter::Cpu::new(0, 0);
            let state = state();
            c.restore_architectural_state(&state);
            r.restore_architectural_state(&state);
            let mut e = <Cpu as Backend>::Engine::default();
            let mut finished = false;
            for _ in 0..8 {
                let actual = c.run(&mut e, &mut a, budget);
                let expected = r.run_slice(&mut b, budget);
                if format!("{actual:?}") != format!("{expected:?}")
                    || c.architectural_state() != r.architectural_state()
                    || c.fault_retired() != r.fault_retired()
                {
                    return Err(format!(
                        "scalar probe {word:x}/{budget}: {actual:?}/{expected:?}"
                    ));
                }
                if !matches!(actual, SliceOutcome::Yield { .. }) {
                    finished = true;
                    break;
                }
            }
            if !finished || e.stats.fallback_instructions != 0 {
                return Err("scalar probe progress/fallback failed".into());
            }
            for (i, w) in words.iter().enumerate() {
                if a.fetch_u32(0x1000 + 4 * i as u64)
                    .map_err(|e| e.to_string())?
                    != *w
                {
                    return Err("scalar backend changed guest code".into());
                }
            }
        }
    }
    for word in [0x32800020, 0x0bc00020, 0x53008020, 0x93810020, 0x1b400020] {
        let words = [0xb1000529, 0x1e622820, word];
        let mut a = memory(&words)?;
        let mut b = memory(&words)?;
        let mut c = Cpu::new(0, 0);
        let mut r = trames_interpreter::Cpu::new(0, 0);
        let state = state();
        c.restore_architectural_state(&state);
        r.restore_architectural_state(&state);
        let mut e = <Cpu as Backend>::Engine::default();
        for _ in 0..3 {
            let actual = c.run(&mut e, &mut a, 64);
            let expected = r.run_slice(&mut b, 64);
            if !matches!(
                actual,
                SliceOutcome::Fault(trames_engine::CpuFault::Instruction { .. })
            ) || format!("{actual:?}") != format!("{expected:?}")
                || c.architectural_state() != r.architectural_state()
                || c.fault_retired() != r.fault_retired()
                || e.stats.fallback_instructions != 0
            {
                return Err(format!(
                    "scalar fault probe {word:x}: {actual:?}/{expected:?}"
                ));
            }
        }
    }
    Ok(())
}
