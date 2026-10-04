//! Decode a symbol-bounded ELF region with Vox's native decoder, including
//! instructions unreachable to its default control-flow walker.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 { return Err("usage: vox_disasm_region ELF START_HEX END_HEX".into()); }
    let raw = std::fs::read(&args[1])?;
    let (_, segments) = vox_core::vox::parse_elf(&raw);
    let image = vox_core::vox_decode::Image { segments };
    let mut address = u64::from_str_radix(args[2].trim_start_matches("0x"),16)?;
    let end = u64::from_str_radix(args[3].trim_start_matches("0x"),16)?;
    if address >= end { return Err("empty or reversed disassembly region".into()); }
    while address < end {
        let bytes = image.bytes_at(address).ok_or("address outside ELF code")?;
        let ins = vox_core::x86::decode(bytes,address)
            .ok_or_else(|| format!("Vox decoding stopped at {address:x}; no bytes skipped"))?;
        if ins.len == 0 || address + ins.len as u64 > end {
            return Err("instruction crosses symbol boundary".into());
        }
        let operands = ins.ops.iter().map(|op| op.field()).collect::<Vec<_>>().join(" ");
        println!("{:x}\t{}\t{}",address,ins.mnemonic,operands);
        address += ins.len as u64;
    }
    Ok(())
}
