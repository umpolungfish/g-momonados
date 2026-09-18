//! A machine that runs an IMASM module. Dispatch is on the glyph and nothing
//! else; what an instruction *was* in x86 survives only as payload the glyph
//! reads. Ported from imasm_vm.py. ⊞ engages the ALU, ⋈ links slots, ⊡ commits
//! to memory, ⊤ makes a truth and ⊥ consumes one, ∈ splits and ∋ fuses, > calls
//! and ⊣ terminates, < transfers, ⊙ transfers through data.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;

fn mask(size: u8) -> u128 {
    match size { 1 => 0xFF, 2 => 0xFFFF, 4 => 0xFFFF_FFFF, 8 => u64::MAX as u128, 16 => u128::MAX, _ => u64::MAX as u128 }
}
fn sign(v: u128, size: u8) -> i128 {
    let m = mask(size); let v = v & m;
    if v > m >> 1 { v as i128 - (m as i128 + 1) } else { v as i128 }
}

/// (base register name, size in bytes, byte offset within base)
fn reg_info(name: &str) -> (&'static str, u8, u8) {
    const R64: [&str;16]=["rax","rcx","rdx","rbx","rsp","rbp","rsi","rdi","r8","r9","r10","r11","r12","r13","r14","r15"];
    // 64-bit
    for &r in R64.iter() { if name == r { return (r, 8, 0); } }
    // xmm
    if let Some(rest) = name.strip_prefix("xmm") { if rest.parse::<u8>().is_ok() {
        // return the canonical static string
        const X: [&str;16]=["xmm0","xmm1","xmm2","xmm3","xmm4","xmm5","xmm6","xmm7","xmm8","xmm9","xmm10","xmm11","xmm12","xmm13","xmm14","xmm15"];
        return (X[rest.parse::<usize>().unwrap()], 16, 0);
    }}
    if name == "rip" { return ("rip", 8, 0); }
    // The thread-local segment bases, set by arch_prctl and read through every
    // TLS access. Full-width pseudo-registers, so the ea() base term "fs" or
    // "gs" resolves to the base the guest installed.
    if name == "fs" { return ("fs", 8, 0); }
    if name == "gs" { return ("gs", 8, 0); }
    // e** (32-bit) and r**d
    let map32: [(&str,&str);16]=[("eax","rax"),("ecx","rcx"),("edx","rdx"),("ebx","rbx"),("esp","rsp"),("ebp","rbp"),("esi","rsi"),("edi","rdi"),
        ("r8d","r8"),("r9d","r9"),("r10d","r10"),("r11d","r11"),("r12d","r12"),("r13d","r13"),("r14d","r14"),("r15d","r15")];
    for (n,b) in map32 { if name==n { return (b,4,0); } }
    let map16: [(&str,&str);16]=[("ax","rax"),("cx","rcx"),("dx","rdx"),("bx","rbx"),("sp","rsp"),("bp","rbp"),("si","rsi"),("di","rdi"),
        ("r8w","r8"),("r9w","r9"),("r10w","r10"),("r11w","r11"),("r12w","r12"),("r13w","r13"),("r14w","r14"),("r15w","r15")];
    for (n,b) in map16 { if name==n { return (b,2,0); } }
    // 8-bit low
    let map8: [(&str,&str);16]=[("al","rax"),("cl","rcx"),("dl","rdx"),("bl","rbx"),("spl","rsp"),("bpl","rbp"),("sil","rsi"),("dil","rdi"),
        ("r8b","r8"),("r9b","r9"),("r10b","r10"),("r11b","r11"),("r12b","r12"),("r13b","r13"),("r14b","r14"),("r15b","r15")];
    for (n,b) in map8 { if name==n { return (b,1,0); } }
    // 8-bit high
    for (n,b) in [("ah","rax"),("ch","rcx"),("dh","rdx"),("bh","rbx")] { if name==n { return (b,1,1); } }
    ("rax", 8, 0)
}

pub enum Stop { Halt(String), SysExit(i32) }

/// The bridge for syscalls the no_std VM cannot fulfill on its own: real files.
/// A bare-metal kernel build supplies no `Host`, so `open`/`read`/`write`/
/// `close` on any fd — including 0/1/2 — answer -EBADF/-ENOSYS exactly as
/// before. A hosted std binary implements this trait once with real
/// `std::fs`/stdio and the guest's file and console I/O becomes real: the
/// guest reads and writes actual files under whatever permission the host
/// process already has, and fd 0/1/2 are real stdin/stdout/stderr.
pub trait Host {
    fn open(&mut self, path: &str, flags: i32, mode: i32) -> i32;
    fn read(&mut self, fd: i32, buf: &mut [u8]) -> i64;
    fn write(&mut self, fd: i32, buf: &[u8]) -> i64;
    fn close(&mut self, fd: i32) -> i32;
}

pub struct Machine {
    code: BTreeMap<u64, Vec<(char, Vec<String>)>>,
    addrs: Vec<u64>,
    next_of: BTreeMap<u64, u64>,
    pub entry: u64,
    reg: BTreeMap<String, u128>,
    mem: BTreeMap<u64, u8>,
    flags: (u128, u128, u8),
    kind: String,
    pub steps: u64,
    pub bits: u8,
    /// Parsed from the module's own `; sym NAME 0xADDR` lines, so a saved
    /// `.imasm` file resolves a symbol name without a second read of the
    /// original binary.
    pub symbols: BTreeMap<String, u64>,
    /// Anonymous-mmap and brk cursors: fresh regions far from any loaded code
    /// or data, since `mem` is a sparse map and reads of never-written bytes
    /// already come back zero — a freshly "mapped" page needs no zeroing.
    mmap_next: u64,
    brk_cur: u64,
    /// IRELATIVE relocations (slot, resolver) carried by the module. Applied
    /// once before a process runs: each resolver is called and its pointer
    /// stored in the slot, which is how a static binary's CPU-selected memcpy,
    /// strlen and kin get wired before main.
    irelative: Vec<(u64, u64)>,
    /// RELATIVE relocations (slot, value), stored at load before the process
    /// runs — the self-relocation a static-pie binary would do at entry.
    relative: Vec<(u64, u64)>,
    /// Real file and console I/O, when a hosted caller supplies one.
    host: Option<Box<dyn Host>>,
    /// Optional trace: addresses to log on entry (allocator functions), and the
    /// log itself. Armed by trace_allocs(); read after a run.
    watch: BTreeMap<u64, String>,
    pub syslog: Vec<String>,
    /// Optional per-instruction register trace over a PC window [lo,hi).
    pub trace_lo: u64,
    pub trace_hi: u64,
    /// Optional watch: log every store that touches this address.
    pub wmem: u64,
    cur_pc: u64,
}

impl Machine {
    pub fn new(module: &str) -> Machine {
        let mut m = Machine {
            code: BTreeMap::new(), addrs: Vec::new(), next_of: BTreeMap::new(), entry: 0,
            reg: BTreeMap::new(), mem: BTreeMap::new(), flags: (0,0,1), kind: "cmp".into(), steps: 0, bits: 64,
            symbols: BTreeMap::new(),
            mmap_next: 0x0003_0000_0000, brk_cur: 0x0002_0000_0000, irelative: Vec::new(), relative: Vec::new(), host: None,
            watch: BTreeMap::new(), syslog: Vec::new(), trace_lo: 0, trace_hi: 0, wmem: 0, cur_pc: 0,
        };
        for r in ["rax","rcx","rdx","rbx","rsp","rbp","rsi","rdi","r8","r9","r10","r11","r12","r13","r14","r15","rip","fs","gs"] {
            m.reg.insert(r.into(), 0);
        }
        for k in 0..16 { m.reg.insert(format!("xmm{}", k), 0); }
        m.reg.insert("rsp".into(), 0x7FFF_0000);
        m.parse(module);
        m
    }

    fn parse(&mut self, text: &str) {
        let mut addr: Option<u64> = None;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix(';') {
                let t = rest.trim();
                if let Some(e) = t.strip_prefix("entry ") {
                    if let Ok(v) = u64::from_str_radix(e.trim().trim_start_matches("0x"), 16) { self.entry = v; }
                } else if let Some(b) = t.strip_prefix("bits ") {
                    if let Ok(v) = b.trim().parse::<u8>() { self.bits = v; }
                } else if let Some(s) = t.strip_prefix("irel ") {
                    let mut it = s.split_whitespace();
                    if let (Some(a), Some(b)) = (it.next(), it.next()) {
                        if let (Ok(slot), Ok(res)) = (
                            u64::from_str_radix(a.trim_start_matches("0x"), 16),
                            u64::from_str_radix(b.trim_start_matches("0x"), 16)) {
                            self.irelative.push((slot, res));
                        }
                    }
                } else if let Some(s) = t.strip_prefix("rela ") {
                    let mut it = s.split_whitespace();
                    if let (Some(a), Some(b)) = (it.next(), it.next()) {
                        if let (Ok(slot), Ok(val)) = (
                            u64::from_str_radix(a.trim_start_matches("0x"), 16),
                            u64::from_str_radix(b.trim_start_matches("0x"), 16)) {
                            self.relative.push((slot, val));
                        }
                    }
                } else if let Some(s) = t.strip_prefix("sym ") {
                    let mut it = s.rsplitn(2, ' ');
                    if let (Some(addr_s), Some(name)) = (it.next(), it.next()) {
                        if let Ok(v) = u64::from_str_radix(addr_s.trim().trim_start_matches("0x"), 16) {
                            self.symbols.insert(name.to_string(), v);
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix('=') {
                let mut it = rest.splitn(2, '\t');
                if let (Some(at), Some(blob)) = (it.next(), it.next()) {
                    if let Ok(a) = u64::from_str_radix(at.trim().trim_start_matches("0x"), 16) {
                        let bytes = hexbytes(blob);
                        for (k, b) in bytes.iter().enumerate() { if *b != 0 { self.mem.insert(a + k as u64, *b); } }
                    }
                }
            } else if let Some(rest) = line.strip_prefix('@') {
                if let Ok(a) = u64::from_str_radix(rest.trim().trim_start_matches("0x"), 16) {
                    addr = Some(a); self.code.entry(a).or_default();
                }
            } else if !line.is_empty() {
                if let Some(a) = addr {
                    let mut parts = line.split('\t');
                    if let Some(g) = parts.next() {
                        let glyph = g.chars().next().unwrap_or('?');
                        let fields: Vec<String> = parts.map(|s| s.to_string()).collect();
                        self.code.get_mut(&a).unwrap().push((glyph, fields));
                    }
                }
            }
        }
        self.addrs = self.code.keys().copied().collect();
        for w in self.addrs.windows(2) { self.next_of.insert(w[0], w[1]); }
    }

    // ── slots ──
    fn get_reg(&self, name: &str) -> u128 {
        let (base, size, off) = reg_info(name);
        (self.reg[base] >> (off as u32 * 8)) & mask(size)
    }
    fn set_reg(&mut self, name: &str, val: u128) {
        let (base, size, off) = reg_info(name);
        let cur = self.reg.get(base).copied().unwrap_or(0);
        let nv = match size {
            8 => val & mask(8),
            4 => val & mask(4),           // 32-bit writes zero the top
            16 => val & mask(16),
            _ => { let m = mask(size) << (off as u32 * 8); (cur & !m) | ((val << (off as u32 * 8)) & m) }
        };
        self.reg.insert(base.to_string(), nv);
    }
    fn load(&self, addr: u64, size: u8) -> u128 {
        let mut v = 0u128;
        for k in 0..size as u64 { v |= (*self.mem.get(&(addr + k)).unwrap_or(&0) as u128) << (8 * k); }
        v
    }
    fn store(&mut self, addr: u64, val: u128, size: u8) {
        let v = val & mask(size);
        for k in 0..size as u64 { self.mem.insert(addr + k, ((v >> (8*k)) & 0xFF) as u8); }
        if self.wmem != 0 && addr <= self.wmem && self.wmem < addr + size as u64 && self.syslog.len() < 100_000 {
            self.syslog.push(format!("STORE [{:x}] = {:x} size {} pc {:x} step {}", addr, v, size, self.cur_pc, self.steps));
        }
    }
    fn ea(&self, field: &str) -> (u64, u8) {
        // m:base:index:scale:disp:size
        let parts: Vec<&str> = field.split(':').collect();
        let base = parts[1]; let index = parts[2]; let scale = parts[3];
        let disp = parts[4]; let size: u8 = parts[5].parse().unwrap_or(8);
        // The base slot may fold a segment pseudo-register with a real base by
        // '+', e.g. "fs" or "fs+rax"; sum every term so a TLS access lands at
        // the installed fs/gs base plus any register base.
        let mut a: i128 = 0;
        if !base.is_empty() { for t in base.split('+') { if !t.is_empty() { a += self.get_reg(t) as i128; } } }
        if !index.is_empty() { a += self.get_reg(index) as i128 * scale.parse::<i128>().unwrap_or(1); }
        a += parse_imm(disp);
        ((a as u128 & mask(8)) as u64, size)
    }
    fn read(&self, field: &str, size_hint: u8) -> (u128, u8) {
        match field.as_bytes()[0] {
            b'r' => { let n = &field[2..]; (self.get_reg(n), reg_info(n).1) }
            b'i' => ((parse_imm(&field[2..]) as u128) & mask(8), size_hint),
            _ => { let (a, s) = self.ea(field); (self.load(a, s), s) }
        }
    }
    fn write(&mut self, field: &str, val: u128) {
        if field.as_bytes()[0] == b'r' { self.set_reg(&field[2..], val); }
        else { let (a, s) = self.ea(field); self.store(a, val, s); }
    }
    fn width(&self, field: &str) -> u8 {
        match field.as_bytes()[0] {
            b'r' => reg_info(&field[2..]).1,
            b'm' => field.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(8),
            _ => 8,
        }
    }

    // ── truth ──
    fn cc(&self, name: &str) -> bool {
        // A float compare (comiss/ucomiss and kin) stored its zero and carry
        // bits directly; the parity bit a NaN would set is not modelled.
        if self.kind == "fflags" {
            let zf = self.flags.0 != 0; let cf = self.flags.1 != 0;
            return match name {
                "e"|"z" => zf, "ne"|"nz" => !zf,
                "b"|"c"|"nae" => cf, "ae"|"nb"|"nc" => !cf,
                "be"|"na" => cf || zf, "a"|"nbe" => !(cf || zf),
                "p" => false, "np" => true,
                _ => false,
            };
        }
        let (a, b, size) = self.flags;
        let (zf, sf, cf, of);
        if self.kind == "explicit" {
            zf = a & mask(size) == 0; sf = sign(a, size) < 0;
            cf = b & 1 != 0; of = b & 2 != 0;
        } else if self.kind == "add" {
            // a and b are the two addends; CF is the unsigned carry-out.
            let r = a.wrapping_add(b) & mask(size);
            zf = r == 0; sf = sign(r, size) < 0;
            cf = (a & mask(size)) + (b & mask(size)) > mask(size);
            of = (sign(a,size) >= 0) == (sign(b,size) >= 0) && (sign(r,size) >= 0) != (sign(a,size) >= 0);
        } else if self.kind == "logic" {
            let r = a & mask(size);
            zf = r == 0; sf = sign(r, size) < 0; cf = false; of = false;
        } else if self.kind == "shift" {
            // a is the result, b is the carried-out bit.
            let r = a & mask(size);
            zf = r == 0; sf = sign(r, size) < 0; cf = b != 0; of = false;
        } else if self.kind == "test" {
            let r = (a & b) & mask(size);
            zf = r == 0; sf = sign(r, size) < 0; cf = false; of = false;
        } else {
            let r = a.wrapping_sub(b) & mask(size);
            zf = r == 0; sf = sign(r, size) < 0;
            cf = (a & mask(size)) < (b & mask(size));
            of = (sign(a, size) - sign(b, size)) != sign(r, size);
        }
        match name {
            "e"|"z" => zf, "ne"|"nz" => !zf,
            "s" => sf, "ns" => !sf,
            "b"|"nae"|"c" => cf, "ae"|"nb"|"nc" => !cf,
            "be"|"na" => cf||zf, "a"|"nbe" => !(cf||zf),
            "l"|"nge" => sf!=of, "ge"|"nl" => sf==of,
            "le"|"ng" => zf||(sf!=of), "g"|"nle" => !zf&&sf==of,
            "o" => of, "no" => !of,
            "p"|"np" => {
                let result = match self.kind.as_str() {
                    "add" => a.wrapping_add(b),
                    "explicit"|"logic"|"shift" => a,
                    _ => a.wrapping_sub(b),
                };
                let even = (result as u8).count_ones() % 2 == 0;
                if name == "p" { even } else { !even }
            }
            _ => false,
        }
    }
    fn set_flags(&mut self, a: u128, b: u128, size: u8, kind: &str) { self.flags = (a, b, size); self.kind = kind.to_string(); }
    /// The current carry flag, for adc/sbb to fold in.
    fn cf(&self) -> bool { self.cc("c") }

    fn slot(&self) -> u8 { self.bits / 8 }              // stack slot width, 8 or 4
    fn push_val(&mut self, v: u128) { let s = self.slot(); self.set_reg("rsp", self.get_reg("rsp").wrapping_sub(s as u128)); let sp = self.get_reg("rsp") as u64; self.store(sp, v, s); }
    fn pop_val(&mut self) -> u128 { let s = self.slot(); let sp = self.get_reg("rsp") as u64; let v = self.load(sp, s); self.set_reg("rsp", self.get_reg("rsp").wrapping_add(s as u128)); v }

    /// Linux x86-64 syscall ABI: number in rax, args in rdi,rsi,rdx,r10,r8,r9
    /// (r10 stands in for rcx, which `syscall` itself clobbers). Only
    /// exit/exit_group need nothing from the host; read/write/open/openat/
    /// close all delegate to `self.host` — real files and real console I/O
    /// when one is set, -EBADF/-ENOSYS when it is not. mmap is anonymous-only
    /// (file-backed mapping is a further rung, refused with -ENOSYS exactly
    /// like every syscall this VM cannot yet fulfill); brk tracks a cursor
    /// with no real protection semantics, which is enough for a bump allocator.
    fn do_syscall(&mut self) -> Result<(), Stop> {
        let num = sign(self.get_reg("rax"), 8);
        let a0 = self.get_reg("rdi"); let a1 = self.get_reg("rsi"); let a2 = self.get_reg("rdx");
        let a3 = self.get_reg("r10"); let a4 = self.get_reg("r8");
        match num {
            60 | 231 => return Err(Stop::SysExit((sign(a0,8) & 0xFF) as i32)),
            0 => { // read(fd, buf, count)
                let fd = sign(a0,8) as i32; let buf = a1 as u64; let count = (a2 as usize).min(1<<20);
                let mut tmp = vec![0u8; count];
                let n = self.host.as_mut().map(|h| h.read(fd, &mut tmp)).unwrap_or(-9);
                if n > 0 { for k in 0..n as usize { self.store(buf + k as u64, tmp[k] as u128, 1); } }
                self.set_reg("rax", (n as i128 as u128) & mask(8));
            }
            1 => { // write(fd, buf, count)
                let fd = sign(a0,8) as i32; let buf = a1 as u64; let count = (a2 as usize).min(1<<20);
                let mut bytes = Vec::with_capacity(count);
                for k in 0..count as u64 { bytes.push(*self.mem.get(&(buf + k)).unwrap_or(&0)); }
                let n = self.host.as_mut().map(|h| h.write(fd, &bytes)).unwrap_or(-9);
                self.set_reg("rax", (n as i128 as u128) & mask(8));
            }
            20 => { // writev(fd, iov, iovcnt): gather the iovec array and write it
                let fd = sign(a0,8) as i32; let iov = a1 as u64; let cnt = (a2 as usize).min(1024);
                let mut bytes = Vec::new();
                for i in 0..cnt as u64 {
                    let base = self.load(iov + i*16, 8) as u64;
                    let len = (self.load(iov + i*16 + 8, 8) as usize).min(1<<20);
                    for k in 0..len as u64 { bytes.push(*self.mem.get(&(base + k)).unwrap_or(&0)); }
                }
                let total = bytes.len() as i64;
                let n = self.host.as_mut().map(|h| h.write(fd, &bytes)).map(|_| total).unwrap_or(-9);
                self.set_reg("rax", (n as i128 as u128) & mask(8));
            }
            2 | 257 => { // open(path,flags,mode) / openat(dirfd,path,flags,mode)
                let (path_ptr, flags, mode) = if num == 2 { (a0 as u64, a1 as i32, a2 as i32) } else { (a1 as u64, a2 as i32, a3 as i32) };
                let path = self.read_cstr(path_ptr);
                let fd = self.host.as_mut().map(|h| h.open(&path, flags, mode)).unwrap_or(-38);
                self.set_reg("rax", (fd as i128 as u128) & mask(8));
            }
            3 => { // close(fd)
                let fd = sign(a0,8) as i32;
                let r = self.host.as_mut().map(|h| h.close(fd)).unwrap_or(-9);
                self.set_reg("rax", (r as i128 as u128) & mask(8));
            }
            9 => { // mmap(addr,length,prot,flags,fd,offset)
                let length = a1 as u64; let fd = sign(a4, 8);
                if fd != -1 { self.set_reg("rax", (-38i128 as u128) & mask(8)); }
                else {
                    let page = 4096u64;
                    let n = (((length.max(1) + page - 1) / page) * page).max(page);
                    let addr = self.mmap_next; self.mmap_next += n;
                    self.set_reg("rax", addr as u128);
                    if !self.watch.is_empty() && self.syslog.len() < 100_000 {
                        self.syslog.push(format!("            mmap len=0x{:x} -> 0x{:x} @step {}", length, addr, self.steps));
                    }
                }
            }
            12 => { // brk(addr): 0 reads the current break, else sets it
                if a0 != 0 { self.brk_cur = a0 as u64; }
                self.set_reg("rax", self.brk_cur as u128);
            }
            158 => { // arch_prctl(code, addr): install the thread-local base
                match sign(a0, 8) {
                    0x1002 => { self.set_reg("fs", a1); self.set_reg("rax", 0); } // ARCH_SET_FS
                    0x1001 => { self.set_reg("gs", a1); self.set_reg("rax", 0); } // ARCH_SET_GS
                    0x1003 => { self.store(a1 as u64, self.get_reg("fs"), 8); self.set_reg("rax", 0); } // ARCH_GET_FS
                    0x1004 => { self.store(a1 as u64, self.get_reg("gs"), 8); self.set_reg("rax", 0); } // ARCH_GET_GS
                    _ => { self.set_reg("rax", 0); }
                }
            }
            318 => { // getrandom(buf, len, flags): the sparse map already reads
                     // zero, so the buffer is filled; report the count requested.
                self.set_reg("rax", a1);
            }
            // glibc's static init issues a run of housekeeping calls whose only
            // requirement is that they succeed: set_tid_address, set_robust_list,
            // rseq, sigaltstack, rt_sigaction/procmask, mprotect, munmap,
            // prlimit64, sched_getaffinity, uname, poll, clock_gettime and the
            // like. Their out-parameters read back zero from the sparse map,
            // which each of these tolerates. Reporting success lets init reach
            // main; a real semantics for any one of them is a later rung.
            _ => { self.set_reg("rax", 0); } // succeed by default so userland init proceeds
        }
        Ok(())
    }

    // ── ALU ──
    fn alu(&mut self, op: &str, f: &[String]) {
        match op {
            "nop"|"endbr64"|"endbr32" => return,
            "lea" => { let (a,_) = self.ea(&f[1]); self.write(&f[0], a as u128); return; }
            // Bit scan: index of the lowest (bsf) or highest (bsr) set bit, with
            // ZF flagging a zero source. String routines read these off pmovmskb.
            "bsf"|"bsr" => {
                let s = self.width(&f[0]);
                let v = self.read(&f[1], s).0 & mask(s);
                if v == 0 { self.set_flags(0, 0, s, "cmp"); }
                else {
                    let idx = if op == "bsf" { v.trailing_zeros() } else { 127 - v.leading_zeros() };
                    self.write(&f[0], idx as u128 & mask(s));
                    self.set_flags(1, 0, s, "cmp");
                }
                return;
            }
            // BMI count of trailing/leading zeros: a zero source yields the
            // operand width in bits and sets CF, unlike bsf/bsr. musl's slot
            // search depends on the width-for-empty answer.
            "tzcnt"|"lzcnt" => {
                let s = self.width(&f[0]);
                let bits = s as u32 * 8;
                let v = self.read(&f[1], s).0 & mask(s);
                let (res, cf) = if v == 0 { (bits, 1u128) }
                    else if op == "tzcnt" { (v.trailing_zeros(), 0) }
                    else { (bits - 1 - (v.leading_zeros() - (128 - bits)), 0) };
                self.write(&f[0], res as u128 & mask(s));
                // CF from a zero source, ZF from a zero result, under fflags.
                self.flags = (if res == 0 { 1 } else { 0 }, cf, 0);
                self.kind = "fflags".into();
                return;
            }
            // String move/store, forward (DF=0, the memcpy_fwd/memset case). A
            // rep does the whole run here in one VM step, so a megabyte copy
            // costs one step, not a million.
            "movs"|"rep_movs" => {
                let w = parse_imm(&f[0][2..]) as u64;
                let count = if op.starts_with("rep") { self.get_reg("rcx") as u64 } else { 1 };
                let mut si = self.get_reg("rsi") as u64; let mut di = self.get_reg("rdi") as u64;
                for _ in 0..count {
                    let v = self.load(si, w as u8); self.store(di, v, w as u8);
                    si = si.wrapping_add(w); di = di.wrapping_add(w);
                }
                self.set_reg("rsi", si as u128); self.set_reg("rdi", di as u128);
                if op.starts_with("rep") { self.set_reg("rcx", 0); }
                return;
            }
            "stos"|"rep_stos" => {
                let w = parse_imm(&f[0][2..]) as u64;
                let count = if op.starts_with("rep") { self.get_reg("rcx") as u64 } else { 1 };
                let val = self.get_reg("rax") & mask(w as u8);
                let mut di = self.get_reg("rdi") as u64;
                for _ in 0..count { self.store(di, val, w as u8); di = di.wrapping_add(w); }
                self.set_reg("rdi", di as u128);
                if op.starts_with("rep") { self.set_reg("rcx", 0); }
                return;
            }
            // Bit test: CF becomes the selected bit; the set/reset/complement
            // forms also write it back. A following jb/jae reads CF via fflags.
            "bt"|"bts"|"btr"|"btc" => {
                let sz = self.width(&f[0]);
                let bits = sz as u32 * 8;
                let idx = (self.read(&f[1], sz).0 as u32) % bits;
                let val = self.read(&f[0], sz).0;
                let cf = (val >> idx) & 1;
                self.flags = (0, cf, 0); self.kind = "fflags".into();
                if op != "bt" {
                    let nv = match op { "bts" => val | (1u128 << idx), "btr" => val & !(1u128 << idx), _ => val ^ (1u128 << idx) };
                    self.write(&f[0], nv & mask(sz));
                }
                return;
            }
            // Atomic compare-and-swap: dest is f[0], source register f[1], the
            // implicit accumulator is a/ax/eax/rax sized to the operand. Flags
            // are set as a cmp of accumulator against dest, which is exactly what
            // the following jne in a lock loop reads.
            "cmpxchg" => {
                let s = self.width(&f[0]);
                let acc_name = match s { 1 => "al", 2 => "ax", 4 => "eax", _ => "rax" };
                let dest = self.read(&f[0], s).0 & mask(s);
                let acc = self.get_reg(acc_name) & mask(s);
                self.set_flags(acc, dest, s, "cmp");
                if acc == dest { let src = self.read(&f[1], s).0; self.write(&f[0], src & mask(s)); }
                else { self.set_reg(acc_name, dest); }
                return;
            }
            // Exchange-and-add: dest gets dest+src, the source register gets the
            // old dest, flags as an add.
            "xadd" => {
                let s = self.width(&f[0]);
                let dest = self.read(&f[0], s).0 & mask(s);
                let src = self.read(&f[1], s).0 & mask(s);
                self.write(&f[0], dest.wrapping_add(src) & mask(s));
                self.write(&f[1], dest);
                self.set_flags(dest, src, s, "add");
                return;
            }
            "cdq"|"cltd" => { let v = if sign(self.get_reg("eax"),4) < 0 { mask(4) } else { 0 }; self.set_reg("edx", v); return; }
            "cqo" => { let v = if sign(self.get_reg("rax"),8) < 0 { mask(8) } else { 0 }; self.set_reg("rdx", v); return; }
            "cdqe"|"cltq" => { let v = (sign(self.get_reg("eax"),4) as u128) & mask(8); self.set_reg("rax", v); return; }
            "idiv"|"div" => {
                let size = self.width(&f[0]);
                let d = self.read(&f[0], size).0;
                let (lo_n, hi_n) = ([8u8,4,2,1].contains(&size), true); let _ = (lo_n, hi_n);
                let lo = self.get_reg(match size {8=>"rax",4=>"eax",2=>"ax",_=>"al"});
                let hi = self.get_reg(match size {8=>"rdx",4=>"edx",2=>"dx",_=>"ah"});
                let (q, r);
                if op == "idiv" {
                    let n = sign((hi << (size as u32 *8)) | lo, if size < 8 { size*2 } else { 8 });
                    let dd = sign(d, size);
                    let qq = (n.abs() / dd.abs()) * if (n<0)==(dd<0) {1} else {-1};
                    q = (qq as u128) & mask(size); r = ((n - qq*dd) as u128) & mask(size);
                } else {
                    let n = (hi << (size as u32 *8)) | lo;
                    q = (n / d) & mask(size); r = (n % d) & mask(size);
                }
                self.set_reg(match size {8=>"rax",4=>"eax",2=>"ax",_=>"al"}, q);
                self.set_reg(match size {8=>"rdx",4=>"edx",2=>"dx",_=>"ah"}, r);
                return;
            }
            _ => {}
        }
        if is_simd(op) { self.simd(op, f); return; }
        if is_float(op) { self.float_op(op, f); return; }

        // An operandless mnemonic here is an opcode the decoder does not yet
        // cover, surfaced as a desync. Skip it rather than index past the end;
        // the run continues far enough to show where the next coverage gap is.
        if f.is_empty() { return; }
        let size = self.width(&f[0]);
        let a = self.read(&f[0], size).0;
        if op == "shrd" || op == "shld" {
            let bits = size as u32 * 8;
            let count = (self.read(&f[2],1).0 & if size == 8 {63} else {31}) as u32;
            if count == 0 { return; }
            // For architecturally undefined 16-bit counts 17..31 we retain
            // deterministic concatenated-shift semantics. OF for counts >1
            // is likewise deterministic, not asserted as a hardware guarantee.
            let source = self.read(&f[1],size).0 & mask(size);
            let a = a & mask(size);
            let (result, carry) = if op == "shrd" {
                (((source << bits) | a) >> count, (((source << bits) | a) >> (count-1)) & 1)
            } else {
                ((((a << bits) | source) << count) >> bits, (((a << bits) | source) >> (2*bits-count)) & 1)
            };
            let result = result & mask(size);
            let overflow = ((a ^ result) >> (bits-1)) & 1;
            self.write(&f[0],result);
            self.set_flags(result,carry | (overflow << 1),size,"explicit");
            return;
        }
        if op == "bswap" {
            let r = if size == 8 { (a as u64).swap_bytes() as u128 }
                    else { (a as u32).swap_bytes() as u128 };
            self.write(&f[0], r);
            return;
        }
        if matches!(op, "not"|"neg"|"inc"|"dec") {
            let r = match op { "not" => !a, "neg" => (a as i128).wrapping_neg() as u128, "inc" => a.wrapping_add(1), _ => a.wrapping_sub(1) };
            self.write(&f[0], r & mask(size));
            if op != "not" {
                let cf = if op == "neg" { a & mask(size) != 0 } else { self.cf() };
                let signbit = 1u128 << (size * 8 - 1);
                let of = match op {
                    "neg" => a & mask(size) == signbit,
                    "inc" => a & mask(size) == signbit - 1,
                    _ => a & mask(size) == signbit,
                };
                self.set_flags(r & mask(size), cf as u128 | ((of as u128) << 1), size, "explicit");
            }
            return;
        }
        // One-operand imul/mul: the full 2*size product lands in edx:eax
        // (rdx:rax at 64-bit). Dropping the high half silently breaks every
        // magic-number division the compiler emits.
        if (op == "imul" || op == "mul") && f.len() == 1 {
            let (lo_r, hi_r) = match size { 8 => ("rax","rdx"), 4 => ("eax","edx"), 2 => ("ax","dx"), _ => ("al","ah") };
            let prod: u128 = if op == "imul" {
                ((sign(self.get_reg(lo_r), size)).wrapping_mul(sign(a, size))) as u128
            } else {
                (self.get_reg(lo_r) & mask(size)).wrapping_mul(a & mask(size))
            };
            self.set_reg(lo_r, prod & mask(size));
            let hi = (prod >> (size as u32 * 8)) & mask(size);
            self.set_reg(hi_r, hi);
            let overflow = if op == "mul" { hi != 0 }
                else { prod as i128 != sign(prod & mask(size), size) };
            self.set_flags(prod & mask(size), if overflow {3} else {0}, size, "explicit");
            return;
        }
        if op == "imul" && f.len() == 3 {
            let x = self.read(&f[1], size).0; let y = self.read(&f[2], size).0;
            let r = (sign(x, size) * sign(y, size)) as u128;
            self.write(&f[0], r & mask(size));
            let overflow = r as i128 != sign(r & mask(size),size);
            self.set_flags(r & mask(size), if overflow {3} else {0}, size, "explicit");
            return;
        }
        let b = self.read(&f[f.len()-1], size).0;
        // adc/sbb fold in the incoming carry; without it a bignum add/subtract
        // loses the carry between limbs.
        let cin = if op == "adc" || op == "sbb" { self.cf() as u128 } else { 0 };
        let r: u128 = match op {
            "add" => a.wrapping_add(b),
            "adc" => a.wrapping_add(b).wrapping_add(cin),
            "sub" => a.wrapping_sub(b),
            "sbb" => a.wrapping_sub(b).wrapping_sub(cin),
            "and" => a & b, "or" => a | b, "xor" => a ^ b,
            "imul" => (sign(a,size) * sign(b,size)) as u128,
            // x86 masks the shift count to 5 bits for 8/16/32-bit operands and
            // 6 bits only at 64-bit; masking everything to 63 gave a 32-bit
            // shift by 32-plus a zero where the hardware keeps the low bits.
            "shl"|"sal" => { let c = b & if size==8 {63} else {31}; a.wrapping_shl(c as u32) }
            "shr" => { let c = b & if size==8 {63} else {31}; (a & mask(size)) >> c }
            "sar" => { let c = b & if size==8 {63} else {31}; (sign(a,size) >> c) as u128 }
            _ => a,
        };
        self.write(&f[0], r & mask(size));
        // Flags that carry the carry: add/adc store their addends so CF reads
        // the real overflow; sub/sbb route through the subtract path; the
        // logical ops clear CF.
        match op {
            "imul" => {
                let overflow = r as i128 != sign(r & mask(size),size);
                self.set_flags(r & mask(size), if overflow {3} else {0},size,"explicit");
            }
            "add"|"adc"|"sub"|"sbb" => {
                let a = a & mask(size); let b = b & mask(size);
                let subtract = op == "sub" || op == "sbb";
                let cf = if subtract { a < b + cin } else { a + b + cin > mask(size) };
                let signbit = 1u128 << (size * 8 - 1);
                let of = if subtract { ((a ^ b) & (a ^ r) & signbit) != 0 }
                         else { (!(a ^ b) & (a ^ r) & signbit) != 0 };
                self.set_flags(r & mask(size), cf as u128 | ((of as u128) << 1), size, "explicit");
            }
            "and"|"or"|"xor" => self.set_flags(r & mask(size), 0, size, "logic"),
            "shl"|"sal"|"shr"|"sar" => {
                // CF is the last bit shifted out; a count of zero leaves the
                // flags untouched.
                let c = b & if size==8 {63} else {31};
                if c != 0 {
                    let bits = size as u32 * 8;
                    let cf = if op == "shl" || op == "sal" { (a >> (bits - c as u32)) & 1 }
                             else { (a >> (c - 1)) & 1 };
                    self.set_flags(r & mask(size), cf, size, "shift");
                }
            }
            _ => self.set_flags(r & mask(size), 0, size, "cmp"),
        }
    }

    fn simd(&mut self, op: &str, f: &[String]) {
        let dst = f[0].clone();
        match op {
            "movdqa"|"movdqu"|"movaps"|"movups" => { let v = self.read(&f[1], 16).0; self.write(&dst, v & mask(16)); }
            "movlps"|"movhps"|"movhlps"|"movlhps" => {
                let source = self.read(&f[1],16).0;
                let high = op == "movhps" || op == "movlhps";
                if dst.starts_with("m:") {
                    let (addr, _) = self.ea(&dst);
                    self.store(addr, if high {source >> 64} else {source}, 8);
                } else {
                    let old = self.read(&dst,16).0;
                    let source = if op == "movhlps" {source >> 64} else {source};
                    let shift = if high {64} else {0};
                    self.write(&dst, (old & !(mask(8) << shift)) | ((source & mask(8)) << shift));
                }
            }
            "movd"|"movq" => {
                let w = if op == "movd" { 4 } else { 8 };
                let src_is_x = f[1].starts_with("r:xmm");
                let v = self.read(&f[1], w).0;
                if dst.starts_with("r:xmm") { self.write(&dst, v & mask(w)); }
                else { self.write(&dst, if src_is_x { v & mask(w) } else { v }); }
            }
            "psrldq" => { let a = self.read(&dst,16).0; let n = parse_imm(&f[1][2..]) as u32; self.write(&dst, (a >> (n*8)) & mask(16)); }
            "psrlq"|"psllq" => {
                let a = self.read(&dst,16).0;
                let n = if f[1].as_bytes()[0]==b'i' { parse_imm(&f[1][2..]) as u32 } else { self.read(&f[1],8).0 as u32 };
                let lanes = [ (a & mask(8)), (a >> 64) & mask(8) ];
                let r: [u128;2] = if op=="psrlq" { [lanes[0]>>n, lanes[1]>>n] } else { [(lanes[0]<<n)&mask(8), (lanes[1]<<n)&mask(8)] };
                self.write(&dst, r[0] | (r[1] << 64));
            }
            "unpcklpd"|"unpckhpd"|"unpcklps"|"unpckhps" => {
                let a = self.read(&dst,16).0; let b = self.read(&f[1],16).0;
                let w = if op.ends_with("pd") {8} else {4};
                let start = if op.starts_with("unpckh") {8 / w} else {0};
                let mut result = 0;
                for k in 0..8/w {
                    let shift = (start + k) * w * 8;
                    result |= ((a >> shift) & mask(w)) << (2*k*w*8);
                    result |= ((b >> shift) & mask(w)) << ((2*k+1)*w*8);
                }
                self.write(&dst, result);
            }
            "shufpd"|"shufps" => {
                let a = self.read(&dst,16).0; let b = self.read(&f[1],16).0;
                let sel = parse_imm(&f[2][2..]) as u32;
                let result = if op == "shufpd" {
                    ((a >> ((sel & 1)*64)) & mask(8)) |
                    (((b >> (((sel >> 1) & 1)*64)) & mask(8)) << 64)
                } else {
                    let mut r = 0;
                    for k in 0..4 { let source = if k < 2 {a} else {b};
                        r |= ((source >> (((sel >> (k*2)) & 3)*32)) & mask(4)) << (k*32);
                    } r
                };
                self.write(&dst, result);
            }
            "pinsrw" => {
                let a = self.read(&dst,16).0;
                let b = self.read(&f[1],2).0 & mask(2);
                let shift = (parse_imm(&f[2][2..]) as u32 & 7) * 16;
                self.write(&dst, (a & !(mask(2) << shift)) | (b << shift));
            }
            "pshufd" => {
                let a = self.read(&f[1],16).0; let sel = parse_imm(&f[2][2..]) as u128;
                let l = [a&mask(4),(a>>32)&mask(4),(a>>64)&mask(4),(a>>96)&mask(4)];
                let mut o=0u128; for k in 0..4 { let s=((sel>>(2*k))&3) as usize; o |= l[s] << (32*k); }
                self.write(&dst, o);
            }
            "punpcklqdq" => { let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0; self.write(&dst, (a&mask(8))|((b&mask(8))<<64)); }
            "punpckhqdq" => { let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0; self.write(&dst, (a>>64)|(b & !mask(8))); }
            "punpckldq" => {
                let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0;
                let x=[a&mask(4),(a>>32)&mask(4)]; let y=[b&mask(4),(b>>32)&mask(4)];
                self.write(&dst, x[0] | (y[0]<<32) | (x[1]<<64) | (y[1]<<96));
            }
            "pmuludq" => {
                let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0;
                let x0=a&mask(4); let x2=(a>>64)&mask(4); let y0=b&mask(4); let y2=(b>>64)&mask(4);
                self.write(&dst, (x0*y0) | ((x2*y2)<<64));
            }
            "pxor"|"pand"|"pandn"|"por"|"xorps"|"andps"|"orps" => { let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0;
                self.write(&dst, match op {"pxor"|"xorps"=>a^b,"pand"|"andps"=>a&b,"pandn"=>(!a)&b,_=>a|b}); }
            // Byte/word/dword equality: each lane becomes all-ones on a match,
            // zero otherwise. The SSE2 string routines lean on this and pmovmskb.
            "pcmpeqb"|"pcmpeqw"|"pcmpeqd" => {
                let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0;
                let w:u8 = match op.chars().last().unwrap(){'b'=>1,'w'=>2,_=>4};
                let n=16/w; let mut o=0u128;
                for k in 0..n { let sh=(k*w) as u32*8; if ((a>>sh)&mask(w))==((b>>sh)&mask(w)) { o |= mask(w) << sh; } }
                self.write(&dst,o);
            }
            // Signed 32-bit greater-than comparison, as defined by PCMPGTD.
            "pcmpgtd" => {
                let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0; let mut o=0u128;
                for k in 0..4u32 {
                    let sh=k*32;
                    let x=((a>>sh)&0xffff_ffff) as u32 as i32;
                    let y=((b>>sh)&0xffff_ffff) as u32 as i32;
                    if x > y { o |= 0xffff_ffffu128 << sh; }
                }
                self.write(&dst,o);
            }
            // Per-byte unsigned min/max, used by strcmp/memcmp fast paths.
            "pminub"|"pmaxub" => {
                let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0; let mut o=0u128;
                for k in 0..16u32 { let sh=k*8; let p=(a>>sh)&0xff; let q=(b>>sh)&0xff;
                    let v=if op=="pminub"{p.min(q)}else{p.max(q)}; o |= v<<sh; }
                self.write(&dst,o);
            }
            // Gather the top bit of each of the 16 bytes into a GPR — the mask a
            // string routine tests to find the first differing or zero byte.
            "pmovmskb" => {
                let a=self.read(&f[1],16).0; let mut m=0u128;
                for k in 0..16u32 { if (a>>(k*8+7))&1==1 { m |= 1u128<<k; } }
                self.write(&dst, m);
            }
            _ => { // lane-wise padd/psub/pmull
                let a=self.read(&dst,16).0; let b=self.read(&f[1],16).0;
                let w: u8 = match op.chars().last().unwrap() {'b'=>1,'w'=>2,'d'=>4,'q'=>8,_=>4};
                let n = 16 / w; let mut o=0u128;
                for k in 0..n {
                    let sh = (k*w) as u32 * 8;
                    let p = (a >> sh) & mask(w); let q = (b >> sh) & mask(w);
                    let v = if op.starts_with("padd") { p.wrapping_add(q) } else if op.starts_with("psub") { p.wrapping_sub(q) } else { p.wrapping_mul(q) };
                    o |= (v & mask(w)) << sh;
                }
                self.write(&dst, o);
            }
        }
    }

    /// Read the low w bytes of an operand as a float, widened to f64 so one
    /// path serves both single and double.
    fn fread(&self, field: &str, w: u8) -> f64 {
        let bits = self.read(field, 16).0 & mask(w);
        if w == 4 { f32::from_bits(bits as u32) as f64 } else { f64::from_bits(bits as u64) }
    }
    /// Write a float into the low w bytes, preserving the rest of an xmm
    /// register (scalar ops leave the upper lanes) and storing plainly to
    /// memory.
    fn fwrite(&mut self, field: &str, val: f64, w: u8) {
        let bits: u128 = if w == 4 { (val as f32).to_bits() as u128 } else { val.to_bits() as u128 };
        if field.starts_with("r:xmm") { let cur = self.read(field, 16).0; self.write(field, (cur & !mask(w)) | bits); }
        else { self.write(field, bits); }
    }

    /// IEEE floating point: the scalar and packed single/double ops, plus the
    /// int/float conversions and the ordered compares that set the flags a
    /// following branch reads. Everything is computed as real f32/f64.
    fn float_op(&mut self, op: &str, f: &[String]) {
        let dst = f[0].clone();
        // ── moves ──
        if op == "movss" || op == "movsd" {
            let w = if op == "movss" { 4 } else { 8 };
            let v = self.read(&f[1], 16).0 & mask(w);
            if dst.starts_with("r:xmm") {
                if f[1].as_bytes()[0] == b'm' { self.write(&dst, v); }        // load zero-extends
                else { let cur = self.read(&dst, 16).0; self.write(&dst, (cur & !mask(w)) | v); } // reg-reg merges low lane
            } else {
                // store: exactly the scalar width to memory. The operand field
                // carries the 16-byte xmm size, so writing through it would
                // clobber the twelve bytes past the float.
                let (a, _) = self.ea(&dst);
                self.store(a, v, w);
            }
            return;
        }
        if op == "movupd" { let v = self.read(&f[1], 16).0; self.write(&dst, v & mask(16)); return; }
        // ── float compare with imm8 predicate → per-lane all-ones/zero mask ──
        if op == "cmpss" || op == "cmpsd" || op == "cmpps" || op == "cmppd" {
            let scalar = op.ends_with("ss") || op.ends_with("sd");
            let w: u8 = if op.ends_with("ss") || op.ends_with("ps") { 4 } else { 8 };
            let imm = (parse_imm(&f[2][2..]) as u8) & 7;
            let a = self.read(&dst, 16).0; let b = self.read(&f[1], 16).0;
            let lanes = if scalar { 1 } else { (16 / w) as usize };
            let mut o = a;
            for k in 0..lanes {
                let sh = (k as u32) * (w as u32) * 8;
                let x = if w == 4 { f32::from_bits(((a>>sh)&mask(4)) as u32) as f64 } else { f64::from_bits(((a>>sh)&mask(8)) as u64) };
                let y = if w == 4 { f32::from_bits(((b>>sh)&mask(4)) as u32) as f64 } else { f64::from_bits(((b>>sh)&mask(8)) as u64) };
                let r = match imm { 0=>x==y, 1=>x<y, 2=>x<=y, 3=>x.is_nan()||y.is_nan(),
                                    4=>!(x==y), 5=>!(x<y), 6=>!(x<=y), _=>!(x.is_nan()||y.is_nan()) };
                let m = if r { mask(w) } else { 0 };
                o = (o & !(mask(w) << sh)) | (m << sh);
            }
            self.write(&dst, o & mask(16));
            return;
        }
        // ── ordered/unordered compare → integer-style flags ──
        if op.starts_with("comi") || op.starts_with("ucomi") {
            let w = if op.ends_with("sd") { 8 } else { 4 };
            let a = self.fread(&dst, w); let b = self.fread(&f[1], w);
            let (zf, cf) = if a.is_nan() || b.is_nan() { (true, true) }
                           else if a < b { (false, true) } else if a > b { (false, false) } else { (true, false) };
            self.flags = (if zf {1} else {0}, if cf {1} else {0}, 0);
            self.kind = "fflags".into();
            return;
        }
        // ── int → float ──
        if op.starts_with("cvtsi2") {
            let sw = self.width(&f[1]); let iv = sign(self.read(&f[1], sw).0, sw);
            let w = if op.ends_with("sd") { 8 } else { 4 };
            self.fwrite(&dst, iv as f64, w);
            return;
        }
        // ── float → int (2C truncates, 2D rounds; both taken as truncation) ──
        if matches!(op, "cvttss2si"|"cvtss2si"|"cvttsd2si"|"cvtsd2si") {
            let w = if op.contains("sd") { 8 } else { 4 };
            let val = self.fread(&f[1], w);
            let dw = self.width(&f[0]);
            self.write(&f[0], (val as i64 as u128) & mask(dw));
            return;
        }
        // ── precision convert ──
        if op == "cvtss2sd" { let v = self.fread(&f[1], 4); self.fwrite(&dst, v, 8); return; }
        if op == "cvtsd2ss" { let v = self.fread(&f[1], 8); self.fwrite(&dst, v, 4); return; }
        // ── arithmetic: scalar (ss/sd) and packed (ps/pd) ──
        let w: u8 = if op.ends_with("ss") || op.ends_with("ps") { 4 } else { 8 };
        let scalar = op.ends_with("ss") || op.ends_with("sd");
        let kind = &op[..3];
        let apply = |x: f64, y: f64| match kind {
            "add" => x + y, "sub" => x - y, "mul" => x * y, "div" => x / y,
            "min" => x.min(y), "max" => x.max(y), "sqr" => fsqrt(y), _ => x,
        };
        if scalar {
            let a = self.fread(&dst, w); let b = self.fread(&f[1], w);
            self.fwrite(&dst, apply(a, b), w);
        } else {
            let a = self.read(&dst, 16).0; let b = self.read(&f[1], 16).0;
            let n = 16 / w; let mut o = 0u128;
            for k in 0..n {
                let sh = (k * w) as u32 * 8;
                let x = if w == 4 { f32::from_bits(((a>>sh)&mask(4)) as u32) as f64 } else { f64::from_bits(((a>>sh)&mask(8)) as u64) };
                let y = if w == 4 { f32::from_bits(((b>>sh)&mask(4)) as u32) as f64 } else { f64::from_bits(((b>>sh)&mask(8)) as u64) };
                let r = apply(x, y);
                let rb: u128 = if w == 4 { (r as f32).to_bits() as u128 } else { r.to_bits() as u128 };
                o |= (rb & mask(w)) << sh;
            }
            self.write(&dst, o);
        }
    }

    fn step(&mut self, addr: u64) -> Result<Option<u64>, Stop> {
        let next = self.next_of.get(&addr).copied().unwrap_or(0);
        self.reg.insert("rip".into(), next as u128);
        let insns = self.code.get(&addr).cloned().unwrap_or_default();
        for (glyph, f) in insns {
            match glyph {
                '∋' => continue,
                '⊣' => {
                    if f.get(0).map(|s| s.as_str()) == Some("leave") {
                        let s = self.slot();
                        let rbp = self.get_reg("rbp"); self.set_reg("rsp", rbp);
                        let v = self.load(self.get_reg("rsp") as u64, s); self.set_reg("rbp", v);
                        self.set_reg("rsp", self.get_reg("rsp").wrapping_add(s as u128)); continue;
                    }
                    let ret = self.pop_val();
                    // ret imm16: stdcall callee-cleanup of stack args.
                    if let Some(im) = f.get(1) { if im.as_bytes().get(0) == Some(&b'i') {
                        let n = parse_imm(&im[2..]) as u128;
                        self.set_reg("rsp", self.get_reg("rsp").wrapping_add(n));
                    }}
                    return Ok(Some(ret as u64));
                }
                '⊤' => { let size=self.width(&f[1]); let a=self.read(&f[1],size).0; let b=self.read(&f[2],size).0; self.set_flags(a,b,size,&f[0]); }
                '∈' => { if self.cc(&f[0]) { return Ok(Some(parse_imm(&f[1][2..]) as u64)); } }
                '≺' => { return Ok(Some(parse_imm(&f[1][2..]) as u64)); }
                '⊙' => {
                    if f.get(0).map(|s|s.as_str()) == Some("syscall") { self.do_syscall()?; continue; }
                    if f.get(0).map(|s|s.as_str()) == Some("external") { return Err(Stop::Halt(format!("external {}", f.get(1).cloned().unwrap_or_default()))); }
                    let tgt = self.read(&f[1], 8).0;
                    if f.get(0).map(|s|s.as_str()) == Some("call") { self.push_val(next as u128); }
                    return Ok(Some(tgt as u64));
                }
                '≻' => {
                    self.push_val(next as u128);
                    return Ok(Some(parse_imm(&f[1][2..]) as u64));
                }
                '⋈' | '⊡' => {
                    let op = f[0].as_str();
                    match op {
                        "push" => { let v=self.read(&f[1],self.slot()).0; self.push_val(v); }
                        "pop" => { let v=self.pop_val(); self.write(&f[1], v); }
                        "leave" => { let s=self.slot(); let rbp=self.get_reg("rbp"); self.set_reg("rsp",rbp); let v=self.load(self.get_reg("rsp") as u64,s); self.set_reg("rbp",v); self.set_reg("rsp", self.get_reg("rsp").wrapping_add(s as u128)); }
                        "xchg" => { let x=self.read(&f[1],8).0; let y=self.read(&f[2],8).0; self.write(&f[1],y); self.write(&f[2],x); }
                        "mov"|"movabs" => { let w=self.width(&f[1]); let v=self.read(&f[2],w).0; self.write(&f[1], v & mask(w)); }
                        "movzx" => { let v=self.read(&f[2],8).0; self.write(&f[1], v & mask(self.width(&f[2]))); }
                        "movsx"|"movsxd" => { let sw=self.width(&f[2]); let v=self.read(&f[2],8).0; self.write(&f[1], (sign(v,sw) as u128) & mask(self.width(&f[1]))); }
                        _ => self.alu(op, &f[1..]),
                    }
                }
                '⊥' => {
                    if f.get(1).map(|s|s.as_str()) == Some("set") { let v = if self.cc(&f[0]) {1} else {0}; self.write(&f[2], v); }
                    else if self.cc(&f[0]) { let w=self.width(&f[2]); let v=self.read(&f[3],w).0; self.write(&f[2], v); }
                }
                '⊞' => self.alu(&f[0], &f[1..]),
                _ => {}
            }
        }
        Ok(self.next_of.get(&addr).copied())
    }

    /// Resolve a symbol name to its address from the module's own embedded
    /// table (`; sym NAME 0xADDR`) — no second read of the original binary.
    pub fn resolve(&self, name: &str) -> Option<u64> { self.symbols.get(name).copied() }

    /// Wire real file/console I/O into `open`/`read`/`write`/`close`. With no
    /// host set, those syscalls answer -EBADF/-ENOSYS, same as before.
    pub fn set_host(&mut self, h: Box<dyn Host>) { self.host = Some(h); }

    fn read_cstr(&self, addr: u64) -> String {
        let mut bytes = Vec::new();
        let mut a = addr;
        while bytes.len() < 4096 {
            let b = *self.mem.get(&a).unwrap_or(&0);
            if b == 0 { break; }
            bytes.push(b); a += 1;
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Step from `pc` until it reaches `sentinel` (Ok) or the machine stops
    /// (Err). `sentinel: None` means "no such address exists" — the only ways
    /// out are `Stop::SysExit`/`Stop::Halt`, which is what a whole process
    /// looks like: it never `ret`s to a return address, it exits.
    /// Arm the allocator trace: log the entry to each of these functions, with
    /// the argument registers, into syslog. Call before run_process.
    pub fn trace_allocs(&mut self) {
        for name in ["malloc","free","realloc","calloc","aligned_alloc",
                     "__libc_malloc","__libc_free","__libc_realloc","__libc_calloc"] {
            if let Some(&a) = self.symbols.get(name) { self.watch.insert(a, name.to_string()); }
        }
    }

    /// Read a register by name and a byte from guest memory, for host-side
    /// diagnostics after a run.
    pub fn reg(&self, name: &str) -> u128 { self.get_reg(name) }
    pub fn peek(&self, addr: u64, len: u64) -> Vec<u8> { (0..len).map(|k| *self.mem.get(&(addr+k)).unwrap_or(&0)).collect() }

    fn run_loop(&mut self, mut pc: u64, sentinel: Option<u64>, limit: u64) -> Result<u64, Stop> {
        self.steps = 0;
        let mut trace: Vec<String> = Vec::new();
        loop {
            if Some(pc) == sentinel { return Ok(pc); }
            self.cur_pc = pc;
            if !self.watch.is_empty() {
                if let Some(name) = self.watch.get(&pc) {
                    if self.syslog.len() < 100_000 {
                        let (di, si, dx) = (self.get_reg("rdi"), self.get_reg("rsi"), self.get_reg("rdx"));
                        self.syslog.push(format!("{:>16} rdi=0x{:x} rsi=0x{:x} rdx=0x{:x} @step {}", name, di, si, dx, self.steps));
                    }
                }
            }
            if self.trace_hi > self.trace_lo && pc >= self.trace_lo && pc < self.trace_hi && self.syslog.len() < 100_000 {
                let insn_txt = self.code.get(&pc).map(|v| v.iter().map(|(g,f)| format!("{} {}", g, f.join(" "))).collect::<Vec<_>>().join(";")).unwrap_or_default();
                self.syslog.push(format!("{:x}: ax={:x} bx={:x} cx={:x} dx={:x} si={:x} di={:x} bp={:x} r8={:x}  | {}",
                    pc, self.get_reg("rax"), self.get_reg("rbx"), self.get_reg("rcx"), self.get_reg("rdx"),
                    self.get_reg("rsi"), self.get_reg("rdi"), self.get_reg("rbp"), self.get_reg("r8"), insn_txt));
            }
            if !self.code.contains_key(&pc) {
                return Err(Stop::Halt(format!("no instruction at 0x{:x} after {} steps\n  previous 12:\n{}", pc, self.steps, trace.join("\n"))));
            }
            if trace.len() >= 80 { trace.remove(0); }
            let insn_txt = self.code.get(&pc).map(|v| v.iter().map(|(g,f)| format!("{} {}", g, f.join(" "))).collect::<Vec<_>>().join(" ; ")).unwrap_or_default();
            trace.push(format!("  {:04x}: {}", pc, insn_txt));
            match self.step(pc)? {
                Some(n) => pc = n,
                None => return Err(Stop::Halt(format!("ran off the end after {} steps\n  previous 12:\n{}", self.steps, trace.join("\n")))),
            }
            self.steps += 1;
            if self.steps > limit { return Err(Stop::Halt(format!("step budget {} reached, still running at 0x{:x}\n  previous 12:\n{}", limit, pc, trace.join("\n")))); }
        }
    }

    /// Run one function to its ⊣. 64-bit takes integer args in registers (System
    /// V); 32-bit takes them on the stack (cdecl). Returns eax.
    pub fn call(&mut self, addr: u64, args: &[i64], limit: u64) -> Result<i64, Stop> {
        let sentinel: u64 = 0xDEAD_0000;
        if self.bits == 32 {
            // cdecl: args pushed right-to-left, then the return address on top.
            for v in args.iter().rev() { self.push_val((*v as u32) as u128); }
            self.push_val(sentinel as u128);
        } else {
            for (name, v) in ["rdi","rsi","rdx","rcx","r8","r9"].iter().zip(args) {
                self.set_reg(name, (*v as u128) & mask(8));
            }
            self.push_val(sentinel as u128);
        }
        self.run_loop(addr, Some(sentinel), limit)?;
        Ok(sign(self.get_reg("eax"), 4) as i64)
    }

    /// Run the whole file as a real process from its own entry point: a real
    /// argv/envp/auxv stack underneath it, real syscalls in front of it, no
    /// assumption that it ever returns — it ends by calling exit, same as any
    /// process does. `call()` next to this is a narrower, older contract for
    /// naming one function and getting one value back; this is "run it."
    pub fn run_process(&mut self, argv: &[String], envp: &[String], limit: u64) -> Result<(), Stop> {
        // Wire the IRELATIVE slots first: run each resolver and store the full
        // 64-bit pointer it returns in rax. Done before the argv stack is laid
        // down, on a scratch stack, so a later PLT jump through the slot lands on
        // the real implementation instead of a zero.
        // RELATIVE first: they populate .init_array pointers and GOT slots the
        // resolvers below may themselves read.
        let rela = core::mem::take(&mut self.relative);
        for (slot, value) in &rela { self.store(*slot, *value as u128, 8); }
        self.relative = rela;

        let relocs = core::mem::take(&mut self.irelative);
        let sentinel = 0x7fff_dead_0000u64;
        for (slot, resolver) in &relocs {
            self.set_reg("rsp", 0x7FFF_0000u128);
            self.push_val(sentinel as u128);
            if self.run_loop(*resolver, Some(sentinel), 5_000_000).is_ok() {
                let p = self.get_reg("rax");
                self.store(*slot, p, 8);
            }
        }
        self.irelative = relocs;

        let mut sp = self.get_reg("rsp") as u64;
        let mut argv_ptrs = Vec::new();
        for s in argv {
            let bytes = s.as_bytes();
            sp -= bytes.len() as u64 + 1;
            for (k, b) in bytes.iter().enumerate() { self.mem.insert(sp + k as u64, *b); }
            self.mem.insert(sp + bytes.len() as u64, 0);
            argv_ptrs.push(sp);
        }
        let mut envp_ptrs = Vec::new();
        for s in envp {
            let bytes = s.as_bytes();
            sp -= bytes.len() as u64 + 1;
            for (k, b) in bytes.iter().enumerate() { self.mem.insert(sp + k as u64, *b); }
            self.mem.insert(sp + bytes.len() as u64, 0);
            envp_ptrs.push(sp);
        }
        // argc, argv[], NULL, envp[], NULL, auxv pairs, AT_NULL — the layout
        // the psABI guarantees at process entry, %rsp 16-byte aligned.
        let mut words: Vec<u64> = Vec::new();
        words.push(argv.len() as u64);
        words.extend(&argv_ptrs); words.push(0);
        words.extend(&envp_ptrs); words.push(0);
        words.push(6); words.push(4096);   // AT_PAGESZ
        words.push(0); words.push(0);      // AT_NULL
        let table_addr = (sp - words.len() as u64 * 8) & !0xF;
        for (k, w) in words.iter().enumerate() {
            let a = table_addr + k as u64 * 8;
            for b in 0..8u64 { self.mem.insert(a + b, ((w >> (8 * b)) & 0xFF) as u8); }
        }
        self.set_reg("rsp", table_addr as u128);
        match self.run_loop(self.entry, None, limit) {
            Err(e) => Err(e),
            Ok(_) => Ok(()), // unreachable: sentinel is None, so run_loop only ever returns via Err
        }
    }
}

fn is_simd(op: &str) -> bool {
    matches!(op, "movdqa"|"movdqu"|"movaps"|"movups"|"movd"|"movq"|"movlps"|"movhps"|"movhlps"|"movlhps"|"pxor"|"pand"|"pandn"|"por"
        |"paddd"|"paddq"|"paddw"|"paddb"|"psubd"|"psubq"|"psubw"|"psubb"|"pmulld"|"pmuludq"
        |"psrlq"|"psllq"|"psrldq"|"pshufd"|"pinsrw"|"punpckldq"|"punpcklqdq"|"punpckhqdq"
        |"pcmpeqb"|"pcmpeqw"|"pcmpeqd"|"pcmpgtd"|"pminub"|"pmaxub"|"pmovmskb"
        |"xorps"|"andps"|"orps"|"unpcklpd"|"unpckhpd"|"unpcklps"|"unpckhps"|"shufpd"|"shufps")
}

#[cfg(test)]
mod membrane_simd_tests {
    use super::*;

    #[test]
    fn double_shifts_decode_and_execute() {
        for (bytes, mnemonic, operands) in [
            (vec![0x48,0x0f,0xad,0xd7], "shrd", vec!["r:rdi","r:rdx","r:cl"]),
            (vec![0x48,0x0f,0xac,0xd7,4], "shrd", vec!["r:rdi","r:rdx","i:0x4"]),
            (vec![0x0f,0xa5,0xd0], "shld", vec!["r:eax","r:edx","r:cl"]),
            (vec![0x66,0x0f,0xa4,0x10,1], "shld", vec!["m:rax::1:0x0:2","r:dx","i:0x1"]),
        ] {
            let ins = crate::x86::decode(&bytes,0x1000).unwrap();
            assert_eq!(ins.len, bytes.len());
            assert_eq!(ins.mnemonic, mnemonic);
            assert_eq!(ins.ops.iter().map(|op| op.field()).collect::<Vec<_>>(), operands);
        }
        for (size, dst, src) in [(2,"ax","dx"),(4,"eax","edx"),(8,"rax","rdx")] {
            let bits = size as u32 * 8;
            for op in ["shld","shrd"] {
                for count in 0..=255u32 {
                    let c = count & if size == 8 {63} else {31};
                    if c > bits { continue; } // No hardware-defined expected result.
                    let mut m = Machine::new("");
                    let a = 0x8123456789abcdefu128 & mask(size);
                    let b = 0xfedcba9876543210u128 & mask(size);
                    m.set_reg(dst,a); m.set_reg(src,b); m.set_reg("cl",count as u128);
                    m.set_flags(0,3,size,"explicit");
                    let mut expected=a; let mut source=b; let mut carry=1;
                    for _ in 0..c {
                        if op == "shrd" {
                            carry=expected&1;
                            expected=(expected>>1)|((source&1)<<(bits-1)); source>>=1;
                        } else {
                            carry=(expected>>(bits-1))&1;
                            expected=((expected<<1)|(source>>(bits-1)))&mask(size);
                            source=(source<<1)&mask(size);
                        }
                    }
                    m.alu(op,&[format!("r:{dst}"),format!("r:{src}"),"r:cl".into()]);
                    assert_eq!(m.reg(dst),expected,"{op} size={size} count={count}");
                    assert_eq!(m.cf(),carry!=0);
                    if c==0 { assert!(m.cc("o")); assert!(m.cc("e")); }
                    else {
                        assert_eq!(m.cc("e"),expected==0);
                        assert_eq!(m.cc("p"),(expected as u8).count_ones()%2==0);
                        if c==1 { assert_eq!(m.cc("o"),((a^expected)>>(bits-1))!=0); }
                    }
                }
            }
        }
    }

    #[test]
    fn multiplication_flags_and_full_product() {
        for (size,lo,hi,src) in [(1,"al","ah","bl"),(2,"ax","dx","bx"),
            (4,"eax","edx","ebx"),(8,"rax","rdx","rbx")] {
            for (a,b) in [(0,7),(7,9),(mask(size),2),(1u128<<(size*8-1),2),(mask(size),mask(size))] {
                for op in ["mul","imul"] {
                    let mut m=Machine::new("");
                    m.set_reg(lo,a); m.set_reg(src,b);
                    let product=if op=="mul" {a*b} else {(sign(a,size)*sign(b,size)) as u128};
                    m.alu(op,&[format!("r:{src}")]);
                    assert_eq!(m.reg(lo),product&mask(size));
                    assert_eq!(m.reg(hi),(product>>(size*8))&mask(size));
                    let overflow=if op=="mul" {product>mask(size)} else {product as i128!=sign(product&mask(size),size)};
                    assert_eq!(m.cf(),overflow); assert_eq!(m.cc("o"),overflow);
                }
            }
        }
        for operands in [vec!["r:eax","r:ebx"],vec!["r:eax","r:ebx","i:0x2"]] {
            let mut m=Machine::new(""); m.set_reg("eax",2); m.set_reg("ebx",0x7fffffff);
            m.alu("imul",&operands.iter().map(|s|s.to_string()).collect::<Vec<_>>());
            assert_eq!(m.reg("eax"),0xfffffffe); assert!(m.cf()); assert!(m.cc("o"));
        }
    }

    #[test]
    fn carry_survives_increment_and_full_width_borrow() {
        let mut machine = Machine::new("");
        machine.set_flags(0, 1, 8, "sub");
        machine.write("r:rcx", 4);
        machine.alu("dec", &["r:rcx".into()]);
        assert!(machine.cf());
        machine.write("r:rax", 0);
        machine.alu("sbb", &["r:rax".into(), "i:-0x1".into()]);
        assert_eq!(machine.reg("rax"), 0);
        assert!(machine.cf());
        machine.alu("adc", &["r:rax".into(), "i:-0x1".into()]);
        assert_eq!(machine.reg("rax"), 0);
        assert!(machine.cf());
    }

    #[test]
    fn byte_swap_preserves_flags() {
        let mut machine = Machine::new("");
        machine.set_flags(0, 1, 8, "sub");
        machine.write("r:rax", 0x0123456789abcdef);
        machine.alu("bswap", &["r:rax".into()]);
        assert_eq!(machine.reg("rax"), 0xefcdab8967452301);
        assert!(machine.cf());
    }

    #[test]
    fn packed_moves_preserve_adjacent_memory_and_lanes() {
        let mut machine = Machine::new("");
        machine.write("r:xmm0", (22u128 << 64) | 11);
        machine.store(0x1008, 99, 8);
        machine.simd("movq", &["m:::1:0x1000:8".into(), "r:xmm0".into()]);
        assert_eq!(machine.load(0x1000, 8), 11);
        assert_eq!(machine.load(0x1008, 8), 99);
        machine.simd("movhps", &["r:xmm0".into(), "m:::1:0x1008:8".into()]);
        assert_eq!(machine.reg("xmm0"), (99u128 << 64) | 11);
        machine.write("r:xmm1", (44u128 << 64) | 33);
        machine.simd("unpckhpd", &["r:xmm0".into(), "r:xmm1".into()]);
        assert_eq!(machine.reg("xmm0"), (44u128 << 64) | 99);
        machine.simd("shufpd", &["r:xmm0".into(), "r:xmm1".into(), "i:0x1".into()]);
        assert_eq!(machine.reg("xmm0"), (33u128 << 64) | 44);
        machine.simd("punpckhqdq", &["r:xmm0".into(), "r:xmm1".into()]);
        assert_eq!(machine.reg("xmm0"), (44u128 << 64) | 33);
    }

    #[test]
    fn pinsrw_preserves_other_lanes_and_masks_the_selector() {
        for selector in 0..16 {
            let mut machine = Machine::new("");
            let original = 0x00112233445566778899aabbccddeeffu128;
            machine.write("r:xmm0", original);
            machine.write("r:eax", 0xdead1234);
            let fields = ["r:xmm0".into(), "r:eax".into(), format!("i:0x{:x}", selector)];
            machine.simd("pinsrw", &fields);
            let shift = (selector & 7) * 16;
            assert_eq!(machine.reg("xmm0"),
                (original & !(0xffffu128 << shift)) | (0x1234u128 << shift));
        }
        assert!(is_simd("pinsrw"));
    }
}

/// Square root without std: a couple of Newton steps off a bit-halving seed.
/// Enough for the scalar sqrtss/sqrtsd the code emits; not a rounding-correct
/// libm.
fn fsqrt(x: f64) -> f64 {
    if x < 0.0 { return f64::NAN; }
    if x == 0.0 || x.is_nan() || x.is_infinite() { return x; }
    let mut g = f64::from_bits((x.to_bits() >> 1) + (1u64 << 61));
    for _ in 0..6 { g = 0.5 * (g + x / g); }
    g
}

fn is_float(op: &str) -> bool {
    matches!(op,
        "movss"|"movsd"|"movupd"|
        "addss"|"addsd"|"addps"|"addpd"|
        "subss"|"subsd"|"subps"|"subpd"|
        "mulss"|"mulsd"|"mulps"|"mulpd"|
        "divss"|"divsd"|"divps"|"divpd"|
        "minss"|"minsd"|"minps"|"minpd"|
        "maxss"|"maxsd"|"maxps"|"maxpd"|
        "sqrtss"|"sqrtsd"|"sqrtps"|"sqrtpd"|
        "comiss"|"comisd"|"ucomiss"|"ucomisd"|
        "cvtsi2ss"|"cvtsi2sd"|"cvtss2si"|"cvtsd2si"|"cvttss2si"|"cvttsd2si"|
        "cvtss2sd"|"cvtsd2ss"|
        "cmpss"|"cmpsd"|"cmpps"|"cmppd")
}

fn hexbytes(s: &str) -> Vec<u8> {
    let mut out = Vec::new(); let mut hi: Option<u8> = None;
    for ch in s.chars() {
        let v = match ch { '0'..='9'=>ch as u8-b'0','a'..='f'=>ch as u8-b'a'+10,'A'..='F'=>ch as u8-b'A'+10,_=>continue };
        match hi { None=>hi=Some(v), Some(h)=>{ out.push((h<<4)|v); hi=None; } }
    }
    out
}

/// Parse an immediate spelled "0x1a" or "-0x1a".
fn parse_imm(s: &str) -> i128 {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('-') {
        -(i128::from_str_radix(rest.trim_start_matches("0x"), 16).unwrap_or(0))
    } else {
        i128::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(0)
    }
}
