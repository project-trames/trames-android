// SPDX-License-Identifier: GPL-3.0-or-later
//! Android owns application lifecycle and JNI; CPU and Linux semantics belong
//! to shared Runtime. There is no JNI crossing per guest instruction/syscall.
use trames_engine::{Cpu as Backend, SliceOutcome};
use trames_execution::Registers;
use trames_memory::{Memory, Permissions, PAGE_SIZE};

#[cfg(feature = "jit")]
type Cpu = trames_jit::Cpu;
#[cfg(not(feature = "jit"))]
type Cpu = trames_interpreter::Cpu;

#[repr(C)]
#[derive(Default, Debug)]
pub struct Probe {
    pub schema: u32,
    pub flags: u32,
    pub instructions: u64,
    pub host_page_size: u64,
    pub guest_page_size: u64,
    pub native_instructions: u64,
}
const _: () = assert!(std::mem::size_of::<Probe>() == 40);
const _: () = assert!(std::mem::align_of::<Probe>() == 8);

/// A real translated guest CPU probe, not full distro/session acceptance.
pub fn probe() -> Result<Probe, String> {
    let mut memory = Memory::default();
    memory
        .map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    // mov x0,#40; add x0,x0,#2; svc #0; b start. Guest bytes stay intact.
    let words = [0xd2800500u32, 0x91000800, 0xd4000001, 0x17fffffd];
    for (i, word) in words.iter().enumerate() {
        memory
            .write_array(0x1000 + i as u64 * 4, word.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    memory
        .protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    let mut cpu = Cpu::new(0x1000, 0x2000);
    let mut engine = <Cpu as Backend>::Engine::default();
    let outcome = cpu.run(&mut engine, &mut memory, 64);
    if !matches!(outcome, SliceOutcome::Syscall { instructions: 3 })
        || cpu.x(0) != 42
        || cpu.pc() != 0x100c
    {
        return Err(format!("CPU probe: {outcome:?}"));
    }
    #[cfg(feature = "jit")]
    let (compiled, native, direct) = (
        trames_jit::NATIVE_COMPILED,
        engine.stats.native_instructions,
        engine.stats.direct_syscall_exits,
    );
    #[cfg(not(feature = "jit"))]
    let (compiled, native, direct) = (false, 0u64, 0u64);
    if compiled && native != 2 {
        return Err("JIT selected without native guest execution".into());
    }
    #[cfg(feature = "jit")]
    if compiled && (direct != 1 || engine.stats.fallback_instructions != 0) {
        return Err("SVC exit entered the interpreter".into());
    }
    #[cfg(feature = "jit")]
    let linked = if compiled {
        // Resume from the post-SVC guest PC, committing a simulated return
        // between entries. This warms the fixed branch and continuation slot.
        for _ in 0..3 {
            cpu.set_x(0, 1234);
            if !matches!(
                cpu.run(&mut engine, &mut memory, 64),
                SliceOutcome::Syscall { instructions: 4 }
            ) || cpu.pc() != 0x100c
                || cpu.x(0) != 42
            {
                return Err("linked syscall continuation failed".into());
            }
        }
        let links = engine.link_stats();
        if links.linked_blocks == 0
            || links.syscall_resumes == 0
            || engine.stats.fallback_instructions != 0
        {
            return Err("JIT did not execute native links and cached syscall resumption".into());
        }
        true
    } else {
        false
    };
    #[cfg(not(feature = "jit"))]
    let linked = false;
    #[cfg(feature = "jit")]
    let branches = if compiled {
        branch_probe()?;
        true
    } else {
        false
    };
    #[cfg(not(feature = "jit"))]
    let branches = false;
    #[cfg(feature = "jit")]
    let memory_native = if compiled {
        memory_probe()?;
        true
    } else {
        false
    };
    #[cfg(not(feature = "jit"))]
    let memory_native = false;
    for (i, word) in words.iter().enumerate() {
        if memory
            .fetch_u32(0x1000 + i as u64 * 4)
            .map_err(|e| e.to_string())?
            != *word
        {
            return Err("guest code changed".into());
        }
    }
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page <= 0 {
        return Err("host page size unavailable".into());
    }
    Ok(Probe {
        schema: 1,
        flags: 1
            | ((compiled as u32) << 1)
            | (((native > 0) as u32) << 2)
            | (((direct > 0) as u32) << 3)
            | ((linked as u32) << 4)
            | ((branches as u32) << 5)
            | ((memory_native as u32) << 6),
        instructions: 3,
        host_page_size: page as u64,
        guest_page_size: PAGE_SIZE,
        native_instructions: native,
    })
}

#[cfg(feature = "jit")]
fn branch_probe() -> Result<(), String> {
    let words = [
        0xd2800221u32,
        0x91000400,
        0xf1000421,
        0x54ffffc1,
        0xd4000001,
    ];
    let mut memory = Memory::default();
    memory
        .map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    for (i, word) in words.iter().enumerate() {
        memory
            .write_array(0x1000 + i as u64 * 4, word.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    memory
        .protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    let mut cpu = Cpu::new(0x1000, 0);
    let mut engine = <Cpu as Backend>::Engine::default();
    let outcome = cpu.run(&mut engine, &mut memory, 64);
    if !matches!(outcome, SliceOutcome::Syscall { instructions: 53 })
        || cpu.x(0) != 17
        || cpu.x(1) != 0
        || cpu.pc() != 0x1014
        || cpu.architectural_state().nzcv != 0x60000000
        || engine.stats.fallback_instructions != 0
        || engine.link_stats().linked_blocks == 0
    {
        return Err(format!("native conditional branch probe: {outcome:?}"));
    }
    for (i, word) in words.iter().enumerate() {
        if memory
            .fetch_u32(0x1000 + i as u64 * 4)
            .map_err(|e| e.to_string())?
            != *word
        {
            return Err("conditional branch guest bytes changed".into());
        }
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn memory_probe() -> Result<(), String> {
    let words = [
        0xf9400020u32,
        0x91000400,
        0xf9000020,
        0xf9400020,
        0x91000400,
        0xf9000020,
        0xd4000001,
    ];
    let data_len = (PAGE_SIZE as usize).max(trames_memory::host_mapping_granule());
    let mut memory = Memory::default();
    memory
        .map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    memory
        .map(0x10000, data_len as u64, Permissions::RW)
        .map_err(|e| e.to_string())?;
    for (i, word) in words.iter().enumerate() {
        memory
            .write_array(0x1000 + i as u64 * 4, word.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    memory
        .protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    // Materialize a complete host protection group before admitting an alias;
    // the probe runs on both 4 KiB and 16 KiB Android configurations.
    memory
        .write(0x10000, &vec![0; data_len])
        .map_err(|e| e.to_string())?;
    memory
        .write_array(0x10000, 40u64.to_le_bytes())
        .map_err(|e| e.to_string())?;
    let mut cpu = Cpu::new(0x1000, 0);
    cpu.set_x(1, 0x10000);
    let mut engine = <Cpu as Backend>::Engine::default();
    let outcome = cpu.run(&mut engine, &mut memory, 64);
    if !matches!(outcome, SliceOutcome::Syscall { instructions: 7 })
        || cpu.x(0) != 42
        || cpu.pc() != 0x101c
        || memory.read_u64(0x10000).map_err(|e| e.to_string())? != 42
        || engine.stats.fallback_instructions != 0
        || engine.memory_stats().instructions != 4
        || engine.memory_stats().hardware != 2
        || engine.memory_stats().staged != 0
        || engine.memory_stats().fused_entries != 1
    {
        return Err(format!("native scalar memory probe: {outcome:?}"));
    }
    for (i, word) in words.iter().enumerate() {
        if memory
            .fetch_u32(0x1000 + i as u64 * 4)
            .map_err(|e| e.to_string())?
            != *word
        {
            return Err("native memory probe changed guest code".into());
        }
    }
    Ok(())
}

/// # Safety
/// `out` must point to a live, aligned, writable Probe owned by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn trames_android_probe(out: *mut Probe) -> i32 {
    if out.is_null() {
        return -1;
    }
    match probe() {
        Ok(report) => {
            unsafe {
                out.write(report);
            }
            0
        }
        Err(_) => -2,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn guest_cpu_probe_keeps_original_instructions_and_exact_exit() {
        let report = super::probe().unwrap();
        assert_eq!(
            (report.schema, report.instructions, report.guest_page_size),
            (1, 3, 4096)
        );
        assert_ne!(report.flags & 1, 0);
    }
}
