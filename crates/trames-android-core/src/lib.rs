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
        vector_probe()?;
        vector_memory_probe()?;
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
    let mut words = vec![
        0xf9400020u32,
        0x91000400,
        0xf9000020,
        0xf9400020,
        0x91000400,
        0xf9000020,
        0xa9401020, // LDP x0,x4
        0x91000400,
        0xa9001020, // STP x0,x4
        0xc8dffc20, // LDAR x0
        0x91000400,
        0xc89ffc20, // STLR x0
        0xd5033bbf, // DMB ISH
        0xd5033b9f, // DSB ISH
        0xd5033fdf, // ISB SY
        0xd4000001,
    ];
    // This is a diagnostic probe, not the CPU execution hot path. Only test
    // instructions present in the actual process-host ABI; never emulate an
    // absent extension to make the probe pass.
    let host = trames_jit::host_features().ok_or("native host profile missing")?;
    let mut expected = 44;
    let mut memory_instructions = 8;
    words.pop();
    if host.hwcap & (1 << 8) != 0 {
        words.extend_from_slice(&[
            0xd2800024, 0xf8240020, 0x91000400, 0x91000404, 0xc8a07c24, 0xa9401424, 0x91000486,
            0xaa0503e7, 0x48247c26, 0xf9400020,
        ]);
        expected += 3;
        memory_instructions += 5;
    }
    if host.hwcap & (1 << 15) != 0 {
        words.extend_from_slice(&[0xf8bfc020, 0x91000400, 0xc89ffc20]);
        expected += 1;
        memory_instructions += 2;
    }
    if host.hwcap & (1 << 26) != 0 {
        words.extend_from_slice(&[0xd9400020, 0x91000400, 0xd9000020]);
        expected += 1;
        memory_instructions += 2;
    }
    words.push(0xd4000001);
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
    if !matches!(outcome, SliceOutcome::Syscall { instructions } if instructions == words.len() as u64)
        || cpu.x(0) != expected
        || cpu.pc() != 0x1000 + 4 * words.len() as u64
        || memory.read_u64(0x10000).map_err(|e| e.to_string())? != expected
        || engine.stats.fallback_instructions != 0
        || engine.memory_stats().instructions != memory_instructions
        || engine.memory_stats().hardware != memory_instructions - 2
        || engine.memory_stats().direct != memory_instructions - 2
        || engine.compilation_stats().address_reuses == 0
        || engine.memory_stats().staged != 0
        || engine.memory_stats().fused_entries != 1
    {
        return Err(format!(
            "native scalar/pair/ordered/LSE/RCpc memory probe: {outcome:?}"
        ));
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

#[cfg(feature = "jit")]
fn vector_probe() -> Result<(), String> {
    let mut words = vec![
        0x1e6e1001u32,
        0x1e601000,
        0x1e612800,
        0x1e610800,
        0x9e780000,
        0xd2800023,
        0x9e670062,
        0x4ee28442,
        0x9e660044,
        0x1e61281f,
        0x9e6603e5,
    ];
    let half = trames_jit::host_features()
        .ok_or("native profile missing")?
        .hwcap
        & (1 << 9)
        != 0;
    if half {
        words.extend_from_slice(&[0x52878006, 0x1ee700c3, 0x1ee32863, 0x1ee60066]);
    }
    words.push(0xd4000001);
    let mut m = Memory::default();
    m.map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    for (i, w) in words.iter().enumerate() {
        m.write_array(0x1000 + 4 * i as u64, w.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    m.protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    let mut c = Cpu::new(0x1000, 0);
    let mut e = <Cpu as Backend>::Engine::default();
    let out = c.run(&mut e, &mut m, 64);
    let state = c.architectural_state();
    if !matches!(out,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || c.pc() != 0x1000 + 4 * words.len() as u64
        || c.x(0) != 3
        || c.x(4) != 2
        || c.x(5) != 4f64.to_bits()
        || half && c.x(6) != 0x4000
        || state.vectors[0] != 3f64.to_bits() as u128
        || state.vectors[2] != 2
        || state.vectors[31] != 4f64.to_bits() as u128
        || state.fpsr != 0
        || e.stats.fallback_instructions != 0
        || e.stats.native_instructions != words.len() as u64 - 1
        || e.link_stats().entries != 1
        || e.memory_stats().instructions != 0
    {
        return Err(format!("native FP/NEON probe: {out:?}"));
    }
    for (i, w) in words.iter().enumerate() {
        if m.fetch_u32(0x1000 + 4 * i as u64)
            .map_err(|e| e.to_string())?
            != *w
        {
            return Err("FP probe changed guest code".into());
        }
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn vector_memory_probe() -> Result<(), String> {
    for (load, store, width) in [
        (0x3dc00020u32, 0x3d800020u32, 16),
        (0xad407c20, 0xad007c20, 32),
    ] {
        // LDR/LDP Q, ADD v0.4s, STR/STP Q, SUBS, B.NE, SVC. Pair's V31
        // shares its number with XZR but remains an ordinary vector register.
        let words = [load, 0x4ea18400, store, 0xf1000442, 0x54ffff81, 0xd4000001];
        let mut m = Memory::default();
        m.map(0x1000, PAGE_SIZE, Permissions::RW)
            .map_err(|e| e.to_string())?;
        let data_len = (PAGE_SIZE as usize).max(trames_memory::host_mapping_granule());
        m.map(0x10000, data_len as u64, Permissions::RW)
            .map_err(|e| e.to_string())?;
        m.write(0x10000, &vec![0; data_len])
            .map_err(|e| e.to_string())?;
        for (i, w) in words.iter().enumerate() {
            m.write_array(0x1000 + 4 * i as u64, w.to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
        m.protect(0x1000, PAGE_SIZE, Permissions::RX)
            .map_err(|e| e.to_string())?;
        let mut c = Cpu::new(0x1000, 0);
        c.set_x(1, 0x10000);
        c.set_x(2, 32);
        let mut state = c.architectural_state();
        state.vectors[1] = 0x00000001000000010000000100000001;
        c.restore_architectural_state(&state);
        let mut e = <Cpu as Backend>::Engine::default();
        let out = c.run(&mut e, &mut m, 256);
        let stats = e.memory_stats();
        let mut actual = [0; 32];
        m.read(0x10000, &mut actual[..width])
            .map_err(|e| e.to_string())?;
        let expected = (32u128 * 0x00000001000000010000000100000001).to_le_bytes();
        if !matches!(out, SliceOutcome::Syscall { instructions: 161 })
            || c.pc() != 0x1018
            || c.x(2) != 0
            || actual[..16] != expected
            || actual[16..width].iter().any(|b| *b != 0)
            || e.stats.fallback_instructions != 0
            || stats.instructions != 64
            || stats.staged != 0
            || stats.hardware < 62
            || stats.direct < 62
            || stats.fused_entries != 1
            || e.compilation_stats().address_reuses == 0
        {
            return Err(format!("native vector-memory fusion probe: {out:?}"));
        }
        for (i, w) in words.iter().enumerate() {
            if m.fetch_u32(0x1000 + 4 * i as u64)
                .map_err(|e| e.to_string())?
                != *w
            {
                return Err("vector-memory probe changed guest code".into());
            }
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
