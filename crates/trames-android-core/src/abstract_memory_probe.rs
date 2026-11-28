// SPDX-License-Identifier: GPL-3.0-or-later
//! Probe only: native instructions through the original abstract CPU contract.
use super::*;
use std::cell::RefCell;
use std::sync::atomic::Ordering;
use trames_execution::Aarch64State;
use trames_memory::{Access, CpuMemory, MemoryError};

struct Abstract {
    memory: Memory,
    fences: RefCell<Vec<Ordering>>,
}
impl CpuMemory for Abstract {
    fn mapped_at_or_after(&self, a: u64) -> bool {
        self.memory.mapped_at_or_after(a)
    }
    fn read(&self, a: u64, out: &mut [u8]) -> Result<(), MemoryError> {
        self.memory.read(a, out)
    }
    fn write(&mut self, a: u64, bytes: &[u8]) -> Result<(), MemoryError> {
        self.memory.write(a, bytes)
    }
    fn check(&self, a: u64, n: usize, access: Access) -> Result<(), MemoryError> {
        self.memory.check(a, n, access)
    }
    fn check_translation(&self, a: u64) -> Result<(), MemoryError> {
        self.memory.check_translation(a)
    }
    fn fetch_u32(&self, a: u64) -> Result<u32, MemoryError> {
        self.memory.fetch_u32(a)
    }
    fn instruction_identity(&self) -> (u64, u64, u64) {
        self.memory.instruction_identity()
    }
    fn instruction_page_identity(&self, a: u64) -> Result<(u64, u64, u64), MemoryError> {
        self.memory.instruction_page_identity(a)
    }
    fn atomic_update<T>(
        &mut self,
        a: u64,
        n: usize,
        f: impl FnOnce(&mut [u8]) -> (T, bool),
    ) -> Result<T, MemoryError> {
        CpuMemory::atomic_update(&mut self.memory, a, n, f)
    }
    fn exclusive_read(&mut self, a: u64, out: &mut [u8]) -> Result<u128, MemoryError> {
        CpuMemory::exclusive_read(&mut self.memory, a, out)
    }
    fn exclusive_write(
        &mut self,
        a: u64,
        bytes: &[u8],
        token: Option<u128>,
    ) -> Result<bool, MemoryError> {
        CpuMemory::exclusive_write(&mut self.memory, a, bytes, token)
    }
    fn fence(&self, order: Ordering) {
        self.fences.borrow_mut().push(order);
        CpuMemory::fence(&self.memory, order);
    }
}

pub(super) fn run() -> Result<(), String> {
    let host = <Cpu as Backend>::features();
    for (word, required) in [
        (0xc8dffc20, 0),  // LDAR
        (0xc89ffc20, 0),  // STLR
        (0xc8e4fc20, 8),  // CASAL
        (0xf8e40020, 8),  // LDADDAL
        (0xf8bfc020, 15), // LDAPR
        (0x99800020, 26), // LDAPURSW
        (0x99000020, 26), // STLUR W
    ] {
        if required != 0 && host.hwcap & (1 << required) == 0 {
            continue;
        }
        for budget in [1, 3, 64] {
            let words: [u32; 5] = [0xb1000529, 0x1e622820, word, word, 0xd4000001];
            let make = || -> Result<Abstract, String> {
                let mut memory = Memory::default();
                for a in [0x1000, 0x10000] {
                    memory
                        .map(a, PAGE_SIZE, Permissions::RW)
                        .map_err(|e| e.to_string())?;
                }
                for (i, word) in words.iter().enumerate() {
                    memory
                        .write_array(0x1000 + 4 * i as u64, word.to_le_bytes())
                        .map_err(|e| e.to_string())?;
                }
                memory
                    .protect(0x1000, PAGE_SIZE, Permissions::RX)
                    .map_err(|e| e.to_string())?;
                memory
                    .write(0x10000, &[0xa5; 16])
                    .map_err(|e| e.to_string())?;
                Ok(Abstract {
                    memory,
                    fences: RefCell::new(Vec::new()),
                })
            };
            let mut a = make()?;
            let mut b = make()?;
            let mut state = Aarch64State {
                pc: 0x1000,
                sp: 0x2000,
                nzcv: 0xb0000000,
                fpsr: 0x10,
                ..Default::default()
            };
            state.registers[0] = 0xfedc_ba98_7654_3210;
            state.registers[1] = 0x10000;
            state.registers[4] = 0xa5a5_a5a5_a5a5_a5a5;
            state.registers[9] = u64::MAX;
            state.vectors[1] = 1.5f64.to_bits() as u128;
            state.vectors[2] = 2.25f64.to_bits() as u128;
            let mut actual = Cpu::new(0, 0);
            let mut reference = trames_interpreter::Cpu::new(0, 0);
            actual.restore_architectural_state(&state);
            reference.restore_architectural_state(&state);
            let mut engine = <Cpu as Backend>::Engine::default();
            let mut finished = false;
            for _ in 0..8 {
                let exit = actual.run_slice(&mut engine, &mut a, budget);
                let expected = reference.run_slice(&mut b, budget);
                if format!("{exit:?}") != format!("{expected:?}")
                    || actual.architectural_state() != reference.architectural_state()
                    || actual.fault_retired() != reference.fault_retired()
                    || *a.fences.borrow() != *b.fences.borrow()
                {
                    return Err(format!(
                        "abstract memory probe {word:x}/{budget}: {exit:?}/{expected:?}"
                    ));
                }
                if !matches!(exit, SliceOutcome::Yield { .. }) {
                    finished = true;
                    break;
                }
            }
            if !finished
                || engine.stats.fallback_instructions != 0
                || a.memory
                    .read_array::<4096>(0x10000)
                    .map_err(|e| e.to_string())?
                    != b.memory
                        .read_array::<4096>(0x10000)
                        .map_err(|e| e.to_string())?
            {
                return Err("abstract memory canonical state/fallback probe failed".into());
            }
            for (i, word) in words.iter().enumerate() {
                if a.memory
                    .fetch_u32(0x1000 + 4 * i as u64)
                    .map_err(|e| e.to_string())?
                    != *word
                {
                    return Err("abstract memory probe changed guest executable bytes".into());
                }
            }
        }
    }
    Ok(())
}
