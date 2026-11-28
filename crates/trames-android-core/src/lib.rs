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
        structure_memory_probe()?;
        movement_crypto_probe()?;
        system_category_probe()?;
        arithmetic_category_probe()?;
        exclusive_category_probe()?;
        special_memory_category_probe()?;
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

#[cfg(feature = "jit")]
fn structure_memory_probe() -> Result<(), String> {
    for (load, store, width) in [
        (0x4c40203fu32, 0x4c00203fu32, 64), // LD1/ST1, four Q registers
        (0x4c40083f, 0x4c00083f, 64),       // LD4/ST4, interleaved 4S
        (0x0d60a03f, 0x0d20a03f, 16),       // LD4/ST4, lane S[0]
        (0x4d60e83f, 0x0d20a03f, 16),       // LD4R 4S / ST4 lane
    ] {
        let words = [
            load, 0x4ea387ff, 0x4ea38400, 0x4ea38421, 0x4ea38442, store, 0xf1000442, 0x54ffff21,
            0xd4000001,
        ];
        let mut m = Memory::default();
        m.map(0x1000, PAGE_SIZE, Permissions::RW)
            .map_err(|e| e.to_string())?;
        let len = (PAGE_SIZE as usize).max(trames_memory::host_mapping_granule());
        m.map(0x10000, len as u64, Permissions::RW)
            .map_err(|e| e.to_string())?;
        m.write(0x10000, &vec![0; len]).map_err(|e| e.to_string())?;
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
        state.vectors[3] = 0x00000001000000010000000100000001;
        c.restore_architectural_state(&state);
        let mut e = <Cpu as Backend>::Engine::default();
        let out = c.run(&mut e, &mut m, 512);
        let mut bytes = [0; 64];
        m.read(0x10000, &mut bytes[..width])
            .map_err(|e| e.to_string())?;
        let state = c.architectural_state();
        let stats = e.memory_stats();
        if !matches!(out, SliceOutcome::Syscall { instructions: 257 })
            || c.pc() != 0x1024
            || c.x(2) != 0
            || !bytes[..width]
                .as_chunks::<4>()
                .0
                .iter()
                .all(|v| *v == 32u32.to_le_bytes())
            || [31, 0, 1, 2]
                .into_iter()
                .any(|r| state.vectors[r] != 32 * 0x00000001000000010000000100000001)
            || e.stats.fallback_instructions != 0
            || stats.instructions != 64
            || stats.staged != 0
            || stats.hardware < 62
            || stats.direct < 62
            || stats.fused_entries != 1
            || e.compilation_stats().address_reuses == 0
        {
            return Err(format!("native structure/lane/replicate probe: {out:?}"));
        }
        for (i, w) in words.iter().enumerate() {
            if m.fetch_u32(0x1000 + 4 * i as u64)
                .map_err(|e| e.to_string())?
                != *w
            {
                return Err("structure probe changed guest code".into());
            }
        }
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn movement_crypto_probe() -> Result<(), String> {
    let caps = trames_jit::host_features().ok_or("native profile missing")?;
    let mut words: Vec<u32> = vec![
        0x4e040cc0, 0x4e181cdf, 0x6e1c241e, 0x4e183fe4, 0x4f0226a1, 0x4f001421, 0x6f004441,
        0x6e011800, 0x4e200820, 0x4e023802, 0x4e0263a0, 0x4e0273bf, 0x0e1c3c24, 0x4e0c2c25,
    ];
    for (word, cap) in [
        (0x4e284820, 3),
        (0x4e285820, 3),
        (0x4e286820, 3),
        (0x4e287820, 3),
        (0x5e280820, 5),
        (0x5e281820, 5),
        (0x5e282820, 6),
        (0x5e020020, 5),
        (0x5e021020, 5),
        (0x5e022020, 5),
        (0x5e023020, 5),
        (0x5e024020, 6),
        (0x5e025020, 6),
        (0x5e026020, 6),
        (0x0e22e020, 1),
        (0x4e22e020, 1),
        (0x0ee2e020, 4),
        (0x4ee2e020, 4),
        (0xcec08020, 21),
        (0xcec08420, 19),
        (0xce628020, 21),
        (0xce628420, 21),
        (0xce628820, 21),
        (0xce628c20, 17),
        (0xce62c020, 18),
        (0xce62c420, 18),
        (0xce62c820, 19),
        (0xce020c20, 17),
        (0xce220c20, 17),
        (0xce420c20, 18),
        (0xce428020, 18),
        (0xce429420, 18),
        (0xce42a820, 18),
        (0xce42bc20, 18),
        (0xce823420, 17),
    ] {
        if caps.hwcap & (1 << cap) != 0 {
            words.push(word);
        }
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
    let mut rm = m.clone();
    let mut c = Cpu::new(0x1000, 0);
    let mut state = c.architectural_state();
    for (i, v) in state.vectors.iter_mut().enumerate() {
        *v = (u128::MAX / 33) * (i as u128 + 1);
    }
    state.registers[6] = 0x839af23d903e6811;
    c.restore_architectural_state(&state);
    let mut reference = trames_interpreter::Cpu::new(0x1000, 0);
    reference.restore_architectural_state(&state);
    let mut e = <Cpu as Backend>::Engine::default();
    let out = c.run(&mut e, &mut m, 128);
    let expected = reference.run_slice(&mut rm, 128);
    if !matches!(out,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || !matches!(expected,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || c.architectural_state() != reference.architectural_state()
        || e.stats.fallback_instructions != 0
        || e.stats.native_instructions != words.len() as u64 - 1
        || e.link_stats().entries != 1
    {
        return Err(format!("native movement/crypto category probe: {out:?}"));
    }
    for (i, w) in words.iter().enumerate() {
        if m.fetch_u32(0x1000 + 4 * i as u64)
            .map_err(|e| e.to_string())?
            != *w
        {
            return Err("movement/crypto probe changed guest code".into());
        }
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn system_category_probe() -> Result<(), String> {
    let words: [u32; 27] = [
        0xd51bd046, 0xd53bd047, 0xd51bd0a7, 0xd53bd0a8, 0xd51b4403, 0x1e361ab4, 0xd53b4424,
        0xd51b4425, 0xd53b4429, 0xd50b7421, 0xd50b7b21, 0xd50b7a21, 0xd50b7e21, 0xd50b7521,
        0xd5033bbf, 0xd5033b9f, 0xd5033fdf, 0xd503201f, 0xd503203f, 0xd503207f, 0xd503245f,
        0xd50322df, 0xd53b002a, 0xd53b00eb, 0xd53bd06c, 0xd53be00d, 0xd4000001,
    ];
    let mut m = Memory::default();
    m.map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    m.map(0x2000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    m.write(0x2000, &[0xa5; 192]).map_err(|e| e.to_string())?;
    for (i, w) in words.iter().enumerate() {
        m.write_array(0x1000 + 4 * i as u64, w.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    m.protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    let mut rm = m.clone();
    let mut c = Cpu::new(0x1000, 0);
    let mut state = c.architectural_state();
    state.registers[1] = 0x207f;
    state.registers[3] = 0;
    state.registers[5] = 0;
    state.registers[6] = 0x839af23d903e6811;
    state.vectors[21] = 1f32.to_bits() as u128;
    state.vectors[22] = 0;
    c.restore_architectural_state(&state);
    let mut r = trames_interpreter::Cpu::new(0x1000, 0);
    r.restore_architectural_state(&state);
    let mut e = <Cpu as Backend>::Engine::default();
    let out = c.run(&mut e, &mut m, 128);
    let reference = r.run_slice(&mut rm, 128);
    if !matches!(out,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || !matches!(reference,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || c.architectural_state() != r.architectural_state()
        || m.read_array::<192>(0x2000).map_err(|e| e.to_string())?
            != rm.read_array::<192>(0x2000).map_err(|e| e.to_string())?
        || c.x(4) != 2
        || c.x(9) != 0
        || c.architectural_state().fpsr != 0
        || c.x(13) != <Cpu as Backend>::counter_frequency()
        || e.stats.fallback_instructions != 0
    {
        return Err(format!("native system category probe: {out:?}"));
    }
    for (i, w) in words.iter().enumerate() {
        if m.fetch_u32(0x1000 + 4 * i as u64)
            .map_err(|e| e.to_string())?
            != *w
        {
            return Err("system category probe changed guest code".into());
        }
    }
    for word in [0xd53be020u32, 0xd53be040, 0xd53be0a0, 0xd53be0c0] {
        let mut clock = Memory::default();
        clock
            .map(0x1000, PAGE_SIZE, Permissions::RW)
            .map_err(|e| e.to_string())?;
        clock
            .write_array(0x1000, 0xd5033fdfu32.to_le_bytes())
            .map_err(|e| e.to_string())?;
        clock
            .write_array(0x1004, word.to_le_bytes())
            .map_err(|e| e.to_string())?;
        clock
            .write_array(0x1008, 0xd4000001u32.to_le_bytes())
            .map_err(|e| e.to_string())?;
        clock
            .protect(0x1000, PAGE_SIZE, Permissions::RX)
            .map_err(|e| e.to_string())?;
        c.set_pc(0x1000);
        let before = trames_interpreter::counter::read();
        let out = c.run(&mut e, &mut clock, 16);
        let after = trames_interpreter::counter::read();
        if !matches!(out, SliceOutcome::Syscall { instructions: 3 })
            || !(before..=after).contains(&c.x(0))
        {
            return Err(format!("native counter domain probe: {out:?}"));
        }
    }
    let mut boundary = Memory::default();
    boundary
        .map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    boundary
        .write_array(0x1000, 0xd44a8a40u32.to_le_bytes())
        .map_err(|e| e.to_string())?;
    boundary
        .protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    c.set_pc(0x1000);
    c.set_x(16, 0x10006);
    c.set_x(17, 7);
    c.set_vulkan_calls(true);
    if !matches!(
        c.run(&mut e, &mut boundary, 1),
        SliceOutcome::NativeCall {
            instructions: 0,
            module: 0x10006,
            function: 7,
            ..
        }
    ) {
        return Err("native call category probe failed".into());
    }
    c.set_vulkan_calls(false);
    if !matches!(c.run(&mut e, &mut boundary, 1), SliceOutcome::Fault(_))
        || c.fault_retired() != 0
        || e.stats.fallback_instructions != 0
    {
        return Err("native trap/category opt-in probe failed".into());
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn arithmetic_category_probe() -> Result<(), String> {
    let caps = trames_jit::host_features().ok_or("native arithmetic profile missing")?;
    let mut words: Vec<u32> = (0..32)
        .map(|r| 0x4ea01c00 | (r << 16) | (r << 5) | r)
        .collect();
    for (word, h, h2) in [
        (0x0f0ba420, 2, 0),
        (0x0e214800, 2, 0),
        (0x0e22c020, 2, 0),
        (0x4f0c9c20, 2, 0),
        (0x6f720820, 2, 0),
        (0x6f724820, 2, 0),
        (0x4f72b820, 2, 0),
        (0x5f7f0420, 2, 0),
        (0x4ea1d820, 2, 0),
        (0x4e20fc20, 2, 0),
        (0x6ea1d820, 2, 0),
        (0x4ea0fc20, 2, 0),
        (0x4e31b820, 2, 0),
        (0x6e214820, 2, 0),
        (0x0e216820, 2, 0),
        (0x4e217820, 2, 0),
        (0x6f72d820, 1 << 12, 0),
        (0x4e829420, 1 << 20, 0),
        (0x4e82a420, 0, 1 << 13),
        (0x6e42fc20, 0, 1 << 14),
        (0x4e22ec20, 1 << 23, 0),
        (0x6e82cc20, 1 << 14, 0),
        (0x4e421420, 1 << 10, 0),
        (0x9ee60027, 1 << 9, 0),
        (0x1e7e0027, 1 << 13, 0),
        (0x4e21e820, 0, 1 << 8),
    ] {
        if caps.hwcap & h == h && caps.hwcap2 & h2 == h2 {
            words.push(word);
        }
    }
    words.extend_from_slice(&[0x3dc0034c, 0x3d80074c, 0xd53b443c, 0xd4000001]);
    let mut m = Memory::default();
    m.map(0x1000, PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    m.map(0x10000, 4 * PAGE_SIZE, Permissions::RW)
        .map_err(|e| e.to_string())?;
    for (i, w) in words.iter().enumerate() {
        m.write_array(0x1000 + 4 * i as u64, w.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    m.protect(0x1000, PAGE_SIZE, Permissions::RX)
        .map_err(|e| e.to_string())?;
    let mut rm = m.clone();
    let mut c = Cpu::new(0x1000, 0);
    let mut r = trames_interpreter::Cpu::new(0x1000, 0);
    let mut state = c.architectural_state();
    state.registers[26] = 0x10ff8;
    for (i, v) in state.vectors.iter_mut().enumerate() {
        *v = (u128::MAX / 33) * (i as u128 + 1);
    }
    c.restore_architectural_state(&state);
    r.restore_architectural_state(&state);
    let mut e = <Cpu as Backend>::Engine::default();
    let out = c.run(&mut e, &mut m, 256);
    let reference = r.run_slice(&mut rm, 256);
    if !matches!(out,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || !matches!(reference,SliceOutcome::Syscall{instructions} if instructions==words.len() as u64)
        || c.architectural_state() != r.architectural_state()
        || m.read_array::<32>(0x10ff8).map_err(|e| e.to_string())?
            != rm.read_array::<32>(0x10ff8).map_err(|e| e.to_string())?
        || e.stats.fallback_instructions != 0
        || e.stats.native_instructions != words.len() as u64 - 1
    {
        return Err(format!(
            "native complete arithmetic category probe: {out:?}"
        ));
    }
    for (i, w) in words.iter().enumerate() {
        if m.fetch_u32(0x1000 + 4 * i as u64)
            .map_err(|e| e.to_string())?
            != *w
        {
            return Err("arithmetic probe changed guest text".into());
        }
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn exclusive_category_probe() -> Result<(), String> {
    // All single widths and both pair widths. Each successful retry loop must
    // retain original guest bytes and return through the shared syscall exit.
    for (lr, sc, width) in [
        (0x085ffc20u32, 0x0803fc20, 1),
        (0x485ffc20, 0x4803fc20, 2),
        (0x885ffc20, 0x8803fc20, 4),
        (0xc85ffc20, 0xc803fc20, 8),
        (0x887f8820, 0x88238820, 8),
        (0xc87f8820, 0xc8238820, 16),
    ] {
        let words = [lr, 0x91000400, sc, 0xb5ffffa3, 0xd5033f5f, 0xd4000001];
        let mut m = Memory::default();
        m.map(0x1000, PAGE_SIZE, Permissions::RW)
            .map_err(|e| e.to_string())?;
        m.map(0x10000, 4 * PAGE_SIZE, Permissions::RW)
            .map_err(|e| e.to_string())?;
        m.write(0x10000, &[0; 32]).map_err(|e| e.to_string())?;
        for (i, word) in words.iter().enumerate() {
            m.write_array(0x1000 + 4 * i as u64, word.to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
        m.protect(0x1000, PAGE_SIZE, Permissions::RX)
            .map_err(|e| e.to_string())?;
        let mut c = Cpu::new(0x1000, 0);
        c.set_x(1, 0x10000);
        c.set_x(4, 0x55);
        let mut e = <Cpu as Backend>::Engine::default();
        let out = c.run(&mut e, &mut m, 4096);
        let retired = match out {
            SliceOutcome::Syscall { instructions } => instructions,
            _ => return Err(format!("exclusive category: {out:?}")),
        };
        if c.x(3) != 0
            || c.x(0) != 1
            || retired < 6
            || (retired - 6) % 4 != 0
            || m.read_array::<1>(0x10000).map_err(|e| e.to_string())? != [1]
            || m.read_array::<16>(0x10010).map_err(|e| e.to_string())? != [0; 16]
            || e.stats.fallback_instructions != 0
            || e.stats.native_instructions != retired - 1
        {
            return Err(format!("native exclusive width {width}: {out:?}"));
        }
        for (i, word) in words.iter().enumerate() {
            if m.fetch_u32(0x1000 + 4 * i as u64)
                .map_err(|e| e.to_string())?
                != *word
            {
                return Err("exclusive probe changed guest text".into());
            }
        }
        // Budgeted LR, native CLREX and a fresh engine share CPU reservation state.
        c.set_pc(0x1000);
        if !matches!(
            c.run(&mut e, &mut m, 1),
            SliceOutcome::Yield { instructions: 1 }
        ) {
            return Err("exclusive probe LR slice".into());
        }
        c.set_pc(0x1010);
        c.run(&mut e, &mut m, 1);
        c.set_pc(0x1008);
        let mut fresh = <Cpu as Backend>::Engine::default();
        c.run(&mut fresh, &mut m, 1);
        if c.x(3) != 1 || fresh.stats.fallback_instructions != 0 {
            return Err("exclusive probe CLREX handoff".into());
        }
    }
    Ok(())
}

#[cfg(feature = "jit")]
fn special_memory_category_probe() -> Result<(), String> {
    let caps = trames_jit::host_features().ok_or("native special-memory profile missing")?;
    for base in [0x19000400u32, 0x1d000400, 0x19c00400] {
        let set = base == 0x19c00400;
        for options in 0..if set { 4 } else { 16 } {
            let words: [u32; 4] = std::array::from_fn(|stage| {
                if stage == 3 {
                    0xd4000001
                } else {
                    base | options << 12
                        | (stage as u32) << if set { 14 } else { 22 }
                        | (17 << 16)
                        | (29 << 5)
                        | 18
                }
            });
            for budget in [1, 64] {
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
                memory
                    .map(0x10000, 0x20000, Permissions::RW)
                    .map_err(|e| e.to_string())?;
                memory
                    .write(0x10000, &[0x83; 16384])
                    .map_err(|e| e.to_string())?;
                memory
                    .write(0x20000, &[0; 16384])
                    .map_err(|e| e.to_string())?;
                let mut cpu = Cpu::new(0x1000, 0);
                cpu.set_x(18, 0x20003);
                cpu.set_x(17, if set { 0x91 } else { 0x10005 });
                cpu.set_x(29, 8197);
                let mut engine = <Cpu as Backend>::Engine::default();
                if caps.hwcap2 & (1 << 43) == 0 {
                    let out = cpu.run(&mut engine, &mut memory, budget);
                    if !matches!(
                        out,
                        SliceOutcome::Fault(trames_engine::CpuFault::Instruction {
                            pc: 0x1000,
                            ..
                        })
                    ) || cpu.fault_retired() != 0
                        || engine.stats.fallback_instructions != 0
                    {
                        return Err(format!(
                            "unavailable MOPS entered an emulation path: {out:?}"
                        ));
                    }
                    continue;
                }
                let mut retired = 0;
                let mut exited = false;
                for _ in 0..16 {
                    match cpu.run(&mut engine, &mut memory, budget) {
                        SliceOutcome::Yield { instructions } => retired += instructions,
                        SliceOutcome::Syscall { instructions } => {
                            retired += instructions;
                            exited = true;
                            break;
                        }
                        other => return Err(format!("native MOPS: {other:?}")),
                    }
                }
                let mut data = [0; 8197];
                memory.read(0x20003, &mut data).map_err(|e| e.to_string())?;
                if !exited
                    || retired != 4
                    || cpu.pc() != 0x1010
                    || cpu.x(29) != 0
                    || data != [if set { 0x91 } else { 0x83 }; 8197]
                    || memory.read_array::<3>(0x20000).map_err(|e| e.to_string())? != [0; 3]
                    || memory.read_array::<4>(0x22008).map_err(|e| e.to_string())? != [0; 4]
                    || engine.stats.fallback_instructions != 0
                {
                    return Err("MOPS progress, bytes, guard or retirement mismatch".into());
                }
                for (i, word) in words.iter().enumerate() {
                    if memory
                        .fetch_u32(0x1000 + i as u64 * 4)
                        .map_err(|e| e.to_string())?
                        != *word
                    {
                        return Err("MOPS modified guest text".into());
                    }
                }
            }
        }
    }
    for base in [0x19208000u32, 0x19201000, 0x19203000] {
        for order in 0..4 {
            let word = base | order << 22 | (30 << 16) | (17 << 5) | 18;
            let mut memory = Memory::default();
            memory
                .map(0x1000, PAGE_SIZE, Permissions::RW)
                .map_err(|e| e.to_string())?;
            memory
                .write_array(0x1000, word.to_le_bytes())
                .map_err(|e| e.to_string())?;
            memory
                .write_array(0x1004, 0xd4000001u32.to_le_bytes())
                .map_err(|e| e.to_string())?;
            memory
                .protect(0x1000, PAGE_SIZE, Permissions::RX)
                .map_err(|e| e.to_string())?;
            memory
                .map(0x10000, 4 * PAGE_SIZE, Permissions::RW)
                .map_err(|e| e.to_string())?;
            let old = 0x0123456789abcdef_fedcba9876543210u128;
            let input = 0xff00aa5500ff55aa_55aaff00aa5500ffu128;
            memory
                .write_array(0x10000, old.to_le_bytes())
                .map_err(|e| e.to_string())?;
            let mut cpu = Cpu::new(0x1000, 0);
            cpu.set_x(17, 0x10000);
            cpu.set_x(18, input as u64);
            cpu.set_x(30, (input >> 64) as u64);
            let mut engine = <Cpu as Backend>::Engine::default();
            let out = cpu.run(&mut engine, &mut memory, 64);
            if caps.hwcap2 & (1 << 47) == 0 {
                if !matches!(
                    out,
                    SliceOutcome::Fault(trames_engine::CpuFault::Instruction { pc: 0x1000, .. })
                ) || engine.stats.fallback_instructions != 0
                {
                    return Err(format!(
                        "unavailable LSE128 entered an emulation path: {out:?}"
                    ));
                }
            } else {
                let expected = match base {
                    0x19201000 => old & !input,
                    0x19203000 => old | input,
                    _ => input,
                };
                if !matches!(out, SliceOutcome::Syscall { instructions: 2 })
                    || cpu.x(18) != old as u64
                    || cpu.x(30) != (old >> 64) as u64
                    || memory
                        .read_array::<16>(0x10000)
                        .map_err(|e| e.to_string())?
                        != expected.to_le_bytes()
                    || engine.stats.fallback_instructions != 0
                {
                    return Err(format!("native LSE128: {out:?}"));
                }
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
    #[cfg(feature = "jit")]
    #[test]
    fn special_memory_probe_uses_only_host_admitted_native_stages() {
        if trames_jit::NATIVE_COMPILED {
            super::special_memory_category_probe().unwrap();
        }
    }
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
