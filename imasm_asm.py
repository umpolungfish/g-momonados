"""imasm_asm.py — a minimal hand-rolled x86-64 instruction encoder and ELF64
writer. No gcc, no as, no ld: every instruction is assembled as raw bytes
here, and the executable file is written directly.

Only encodes the small instruction set imasm_emit.py's VM codegen needs.
"""
import struct


class Asm:
    def __init__(self):
        self.code = bytearray()
        self.labels = {}         # name -> offset into self.code
        self.fixups = []         # (offset_of_rel32_field, target_label, next_instr_offset)

    def here(self):
        return len(self.code)

    def label(self, name):
        assert name not in self.labels, f"duplicate label {name}"
        self.labels[name] = self.here()

    def emit(self, b: bytes):
        self.code += b

    # ---- data movement ----
    def mov_r64_imm64(self, reg, imm):
        # REX.W + B8+r id  (movabs)
        rex = 0x48 | (1 if reg >= 8 else 0)
        self.emit(bytes([rex, 0xB8 + (reg & 7)]))
        self.emit(struct.pack('<Q', imm & 0xFFFFFFFFFFFFFFFF))

    def mov_r32_imm32(self, reg, imm):
        if reg >= 8:
            self.emit(bytes([0x41]))
        self.emit(bytes([0xB8 + (reg & 7)]))
        self.emit(struct.pack('<I', imm & 0xFFFFFFFF))

    def _modrm_disp(self, reg_field, base_reg, disp):
        # ModRM with disp32, base register (no SIB unless base is RSP/R12)
        mod = 0x80  # disp32
        rm = base_reg & 7
        modrm = mod | ((reg_field & 7) << 3) | rm
        out = bytes([modrm])
        if rm == 4:  # RSP/R12 needs SIB
            out += bytes([0x24])
        out += struct.pack('<i', disp)
        return out

    def _modrm_sib_disp(self, reg_field, base_reg, index_reg, scale, disp):
        # ModRM=mod(disp32)+reg+RM(100=SIB follows), SIB=scale+index+base, disp32
        modrm = 0x80 | ((reg_field & 7) << 3) | 0x4
        ss = {1: 0, 2: 1, 4: 2, 8: 3}[scale]
        sib = (ss << 6) | ((index_reg & 7) << 3) | (base_reg & 7)
        return bytes([modrm, sib]) + struct.pack('<i', disp)

    def _rex_bi(self, w, reg, index, base):
        rex = (0x08 if w else 0) | (0x4 if reg >= 8 else 0) | (0x2 if index >= 8 else 0) | (0x1 if base >= 8 else 0)
        return 0x40 | rex

    def movzx_r32_m8_bi(self, dst_reg, base_reg, index_reg, disp, scale=1):
        # movzx r32, byte [base + index*scale + disp]
        rex = self._rex_bi(0, dst_reg, index_reg, base_reg)
        self.emit(bytes([rex, 0x0F, 0xB6]))
        self.emit(self._modrm_sib_disp(dst_reg, base_reg, index_reg, scale, disp))

    def mov_m8_r8_bi(self, base_reg, index_reg, disp, src_reg, scale=1):
        # mov byte [base + index*scale + disp], r8
        rex = self._rex_bi(0, src_reg, index_reg, base_reg)
        self.emit(bytes([rex, 0x88]))
        self.emit(self._modrm_sib_disp(src_reg, base_reg, index_reg, scale, disp))

    def mov_m8_imm8_bi(self, base_reg, index_reg, disp, imm, scale=1):
        rex = self._rex_bi(0, 0, index_reg, base_reg)
        self.emit(bytes([rex, 0xC6]))
        self.emit(self._modrm_sib_disp(0, base_reg, index_reg, scale, disp))
        self.emit(bytes([imm & 0xFF]))

    def mov_r32_m32_bi(self, dst_reg, base_reg, index_reg, disp, scale=1):
        rex = self._rex_bi(0, dst_reg, index_reg, base_reg)
        self.emit(bytes([rex, 0x8B]))
        self.emit(self._modrm_sib_disp(dst_reg, base_reg, index_reg, scale, disp))

    def mov_m32_r32_bi(self, base_reg, index_reg, disp, src_reg, scale=1):
        rex = self._rex_bi(0, src_reg, index_reg, base_reg)
        self.emit(bytes([rex, 0x89]))
        self.emit(self._modrm_sib_disp(src_reg, base_reg, index_reg, scale, disp))

    def lea_r64_bi(self, dst_reg, base_reg, index_reg, disp, scale=1):
        rex = self._rex_bi(1, dst_reg, index_reg, base_reg)
        self.emit(bytes([rex, 0x8D]))
        self.emit(self._modrm_sib_disp(dst_reg, base_reg, index_reg, scale, disp))

    def mov_r8_m8(self, dst_reg, base_reg, disp):
        # movzx r32, byte [base+disp]  -> 0F B6 /r
        rex = 0x40 | (0x4 if dst_reg >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x0F, 0xB6]))
        self.emit(self._modrm_disp(dst_reg, base_reg, disp))

    def mov_m8_r8(self, base_reg, disp, src_reg):
        # mov byte [base+disp], r8  -> 88 /r  (REX needed for sil/dil/bpl/spl low-byte forms)
        rex = 0x40 | (0x4 if src_reg >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x88]))
        self.emit(self._modrm_disp(src_reg, base_reg, disp))

    def mov_m8_imm8(self, base_reg, disp, imm):
        # mov byte [base+disp], imm8 -> C6 /0 ib
        rex = 0x40 | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0xC6]))
        self.emit(self._modrm_disp(0, base_reg, disp))
        self.emit(bytes([imm & 0xFF]))

    def mov_r32_m32(self, dst_reg, base_reg, disp):
        rex = 0x40 | (0x4 if dst_reg >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x8B]))
        self.emit(self._modrm_disp(dst_reg, base_reg, disp))

    def mov_m32_r32(self, base_reg, disp, src_reg):
        rex = 0x40 | (0x4 if src_reg >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x89]))
        self.emit(self._modrm_disp(src_reg, base_reg, disp))

    def mov_r64_m64(self, dst_reg, base_reg, disp):
        rex = 0x48 | (0x4 if dst_reg >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x8B]))
        self.emit(self._modrm_disp(dst_reg, base_reg, disp))

    def mov_m64_r64(self, base_reg, disp, src_reg):
        rex = 0x48 | (0x4 if src_reg >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x89]))
        self.emit(self._modrm_disp(src_reg, base_reg, disp))

    def mov_r32_r32(self, dst_reg, src_reg):
        rex = 0x40 | (0x4 if src_reg >= 8 else 0) | (0x1 if dst_reg >= 8 else 0)
        self.emit(bytes([rex, 0x89]))
        self.emit(bytes([0xC0 | ((src_reg & 7) << 3) | (dst_reg & 7)]))

    def mov_r64_r64(self, dst_reg, src_reg):
        rex = 0x48 | (0x4 if src_reg >= 8 else 0) | (0x1 if dst_reg >= 8 else 0)
        self.emit(bytes([rex, 0x89]))
        self.emit(bytes([0xC0 | ((src_reg & 7) << 3) | (dst_reg & 7)]))

    # ---- arithmetic / logic (32-bit regs) ----
    def alu_r32_imm32(self, op, reg, imm):
        # op in {'add':0,'or':1,'and':4,'sub':5,'xor':6,'cmp':7}
        opc = {'add': 0, 'or': 1, 'and': 4, 'sub': 5, 'xor': 6, 'cmp': 7}[op]
        rex = 0x40 | (0x1 if reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x81]))
        self.emit(bytes([0xC0 | (opc << 3) | (reg & 7)]))
        self.emit(struct.pack('<i', imm))

    def and_r32_r32(self, dst, src):
        rex = 0x40 | (0x4 if src >= 8 else 0) | (0x1 if dst >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x21, 0xC0 | ((src & 7) << 3) | (dst & 7)]))

    def or_r32_r32(self, dst, src):
        rex = 0x40 | (0x4 if src >= 8 else 0) | (0x1 if dst >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x09, 0xC0 | ((src & 7) << 3) | (dst & 7)]))

    def add_r32_r32(self, dst, src):
        rex = 0x40 | (0x4 if src >= 8 else 0) | (0x1 if dst >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x01, 0xC0 | ((src & 7) << 3) | (dst & 7)]))

    def sub_r32_r32(self, dst, src):
        rex = 0x40 | (0x4 if src >= 8 else 0) | (0x1 if dst >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x29, 0xC0 | ((src & 7) << 3) | (dst & 7)]))

    def cmp_r32_r32(self, a, b):
        rex = 0x40 | (0x4 if b >= 8 else 0) | (0x1 if a >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x39, 0xC0 | ((b & 7) << 3) | (a & 7)]))

    def cmp_r32_imm32(self, reg, imm):
        self.alu_r32_imm32('cmp', reg, imm)

    def cmp_m8_imm8(self, base_reg, disp, imm):
        rex = 0x40 | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x80]))
        self.emit(self._modrm_disp(7, base_reg, disp))
        self.emit(bytes([imm & 0xFF]))

    def shl_r32_imm8(self, reg, imm):
        rex = 0x40 | (0x1 if reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0xC1, 0xE0 | (reg & 7), imm & 0xFF]))

    def shr_r32_imm8(self, reg, imm):
        rex = 0x40 | (0x1 if reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0xC1, 0xE8 | (reg & 7), imm & 0xFF]))

    def inc_r32(self, reg):
        rex = 0x40 | (0x1 if reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0xFF, 0xC0 | (reg & 7)]))

    def dec_r32(self, reg):
        rex = 0x40 | (0x1 if reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0xFF, 0xC8 | (reg & 7)]))

    def inc_m32(self, base_reg, disp):
        rex = 0x40 | (0x1 if base_reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0xFF]))
        self.emit(self._modrm_disp(0, base_reg, disp))

    def inc_m64(self, base_reg, disp):
        rex = 0x48 | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0xFF]))
        self.emit(self._modrm_disp(0, base_reg, disp))

    def neg_r32(self, reg):
        rex = 0x40 | (0x1 if reg >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0xF7, 0xD8 | (reg & 7)]))

    def test_r32_r32(self, a, b):
        rex = 0x40 | (0x4 if b >= 8 else 0) | (0x1 if a >= 8 else 0)
        if rex != 0x40:
            self.emit(bytes([rex]))
        self.emit(bytes([0x85, 0xC0 | ((b & 7) << 3) | (a & 7)]))

    # ---- control flow (always rel32, patched) ----
    def _jcc(self, tttn, label):
        self.emit(bytes([0x0F, 0x80 | tttn]))
        pos = self.here()
        self.emit(b'\x00\x00\x00\x00')
        self.fixups.append((pos, label, self.here()))

    def je(self, label): self._jcc(0x4, label)
    def jne(self, label): self._jcc(0x5, label)
    def jl(self, label): self._jcc(0xC, label)
    def jge(self, label): self._jcc(0xD, label)
    def jg(self, label): self._jcc(0xF, label)
    def jle(self, label): self._jcc(0xE, label)
    def jb(self, label): self._jcc(0x2, label)   # unsigned <
    def jae(self, label): self._jcc(0x3, label)  # unsigned >=
    def ja(self, label): self._jcc(0x7, label)   # unsigned >

    def jmp(self, label):
        self.emit(bytes([0xE9]))
        pos = self.here()
        self.emit(b'\x00\x00\x00\x00')
        self.fixups.append((pos, label, self.here()))

    def syscall(self):
        self.emit(bytes([0x0F, 0x05]))

    def push_r64(self, reg):
        if reg >= 8:
            self.emit(bytes([0x41]))
        self.emit(bytes([0x50 + (reg & 7)]))

    def pop_r64(self, reg):
        if reg >= 8:
            self.emit(bytes([0x41]))
        self.emit(bytes([0x58 + (reg & 7)]))

    def lea_r64(self, dst, base_reg, disp):
        rex = 0x48 | (0x4 if dst >= 8 else 0) | (0x1 if base_reg >= 8 else 0)
        self.emit(bytes([rex, 0x8D]))
        self.emit(self._modrm_disp(dst, base_reg, disp))

    def finish(self):
        for pos, label, next_off in self.fixups:
            target = self.labels[label]
            rel = target - next_off
            self.code[pos:pos + 4] = struct.pack('<i', rel)
        return bytes(self.code)


# register numbers (x86-64)
RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI = range(8)
R8, R9, R10, R11, R12, R13, R14, R15 = range(8, 16)


def build_elf64_exec(code: bytes, data: bytes, entry_off_in_code: int,
                      code_vaddr=0x400000, data_vaddr=0x600000):
    """Two PT_LOAD segments: code (R+X) at code_vaddr, data (RW) at data_vaddr.
    File layout: [ELF header][2 program headers][code][pad][data].

    The loader requires p_offset == p_vaddr (mod p_align) for each PT_LOAD.
    Both vaddrs are page-aligned, so the code segment's p_offset is 0 (the
    header rides along inside its own R+E mapping, the classic minimal-ELF
    trick) and the data segment is padded out to the next page boundary in
    the file so its offset is page-aligned too.
    """
    PAGE = 0x1000
    ehsize = 64
    phentsize = 56
    phnum = 2
    code_off = ehsize + phentsize * phnum  # code bytes start right after headers, offset 0 segment
    entry = code_vaddr + code_off + entry_off_in_code

    prefix_len = code_off + len(code)
    data_off = (prefix_len + PAGE - 1) // PAGE * PAGE
    pad = b'\x00' * (data_off - prefix_len)

    eh = struct.pack(
        '<4sBBBBB7xHHIQQQIHHHHHH',
        b'\x7fELF', 2, 1, 1, 0, 0,
        2,          # e_type ET_EXEC
        0x3e,       # e_machine EM_X86_64
        1,          # e_version
        entry,      # e_entry
        ehsize,     # e_phoff
        0,          # e_shoff
        0,          # e_flags
        ehsize,     # e_ehsize
        phentsize,  # e_phentsize
        phnum,      # e_phnum
        0, 0, 0,    # e_shentsize, e_shnum, e_shstrndx
    )

    ph_code = struct.pack(
        '<IIQQQQQQ',
        1,               # PT_LOAD
        5,               # R+X
        0, code_vaddr, code_vaddr,
        prefix_len, prefix_len,
        PAGE,
    )
    ph_data = struct.pack(
        '<IIQQQQQQ',
        1,               # PT_LOAD
        6,               # R+W
        data_off, data_vaddr, data_vaddr,
        len(data), len(data),
        PAGE,
    )

    return eh + ph_code + ph_data + code + pad + data, code_vaddr + code_off, data_vaddr
