#!/usr/bin/env python3
"""imasm_link.py — link the whole imasm/ corpus into ONE standalone native
ELF64 executable. No gcc, no as, no ld: every instruction is chosen and
encoded by imasm_asm.Asm, and the file bytes are written directly.

Where imasm_emit.py compiles one glyph word into its own binary by baking
that word's length, program bytes and IMSCRIB snapshot fields in as
compile-time constants, this builds a single data-driven engine and hands
it a table of every word in every module. The engine walks the table,
resets its state per word, runs the exact tick loop kernel.rs / gpu_kernel.rs's
CUDA kernel both run, prints that word's result block, and moves to the next.

The output binary needs no interpreter, no vox, no cargo, no Python at
runtime. It IS the whole corpus, running on bare silicon.
"""
import sys, os, glob, struct
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from imasm_asm import (Asm, build_elf64_exec,
                       RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI,
                       R8, R9, R10, R11, R12, R13, R14, R15)

GLYPH_TO_ID = {
    '⊢': 0, '⊣': 1, '≻': 2, '≺': 3, '⋈': 4, '⊤': 5,
    '∈': 6, '∋': 7, '⊙': 8, '⊥': 9, '⊞': 10, '⊡': 11,
}

MAX_TICKS = 65536
SDEPTH = 256
FCAP = 64
WSTRIDE = 16   # bytes per word-table entry (power of two -> usable as index*1)


def precompute(ids):
    """IMSCRIB's four snapshot fields, from the INITIAL program, exactly as
    the reference computes them before its loop runs."""
    plen = len(ids)
    seen = [False] * 16
    diversity = 0
    for t in ids:
        if not seen[t]:
            seen[t] = True
            diversity += 1
    self_ref = 1 if (plen > 0 and ids[0] == ids[-1]) else 0
    frob_pos = 1 if any(t in (6, 7, 12, 13) for t in ids) else 0
    h5 = 5 in ids; h9 = 9 in ids; h10 = 10 in ids
    dial = 0
    if h5 and h9 and h10:
        dial = 1
        for i, t in enumerate(ids):
            if t == 10:
                found = False
                for off in range(1, plen):
                    if ids[(i + off) % plen] in (5, 9):
                        found = True
                        break
                if not found:
                    dial = 0
                    break
    return diversity & 3, (1 if self_ref else 2), (1 if frob_pos else 2), (1 if dial else 2)


def collect_words(imasm_dir):
    """Every word in every module, sorted by module then line. Returns a list
    of (module, lineno, ids)."""
    words = []
    for path in sorted(glob.glob(os.path.join(imasm_dir, '*.imasm'))):
        module = os.path.splitext(os.path.basename(path))[0]
        with open(path, encoding='utf-8') as f:
            lineno = 0
            for raw in f:
                lineno += 1
                line = raw.strip()
                if not line:
                    continue
                ids = []
                ok = True
                for c in line:
                    if c not in GLYPH_TO_ID:
                        ok = False
                        break
                    ids.append(GLYPH_TO_ID[c])
                if ok and ids:
                    words.append((module, lineno, ids))
    return words


def build(words, out_path):
    nwords = len(words)

    # ---- program blob + word table (built in Python, all runtime data) ----
    blob = bytearray()
    wtab = bytearray()
    for (_mod, _ln, ids) in words:
        off = len(blob)
        blob.extend(ids)
        div, sref, frob, dial = precompute(ids)
        entry = struct.pack('<IIBBBBI', off, len(ids), div, sref, frob, dial, 0)
        assert len(entry) == WSTRIDE
        wtab.extend(entry)

    # ---- data layout ----
    OFF_STACK = 0
    OFF_REG = OFF_STACK + SDEPTH
    OFF_ENGAGR = OFF_REG + 8
    OFF_MEM = OFF_ENGAGR + 1
    OFF_FRESUME = OFF_MEM + 4
    OFF_FRIGHT = OFF_FRESUME + FCAP * 4
    OFF_FSET = OFF_FRIGHT + FCAP
    OFF_TOP = OFF_FSET + FCAP
    OFF_IP = OFF_TOP + 4
    OFF_FDEPTH = OFF_IP + 4
    OFF_TICK = OFF_FDEPTH + 4
    OFF_HALTED = OFF_TICK + 8
    OFF_CPLEN = OFF_HALTED + 1 + 3      # keep u32s 4-aligned for tidiness
    OFF_CPROGOFF = OFF_CPLEN + 4
    OFF_CDIV = OFF_CPROGOFF + 4
    OFF_CSREF = OFF_CDIV + 1
    OFF_CFROB = OFF_CSREF + 1
    OFF_CDIAL = OFF_CFROB + 1
    OFF_WIDX = OFF_CDIAL + 1 + 3
    OFF_STRTAB = OFF_WIDX + 4           # "NTFB"
    OFF_DIGITS = OFF_STRTAB + 4         # 20 scratch
    OFF_OUTBUF = OFF_DIGITS + 20
    OUTBUF_CAP = 8192
    OFF_WTAB = OFF_OUTBUF + OUTBUF_CAP
    OFF_BLOB = OFF_WTAB + len(wtab)
    DATA_SIZE = OFF_BLOB + len(blob)

    data = bytearray(DATA_SIZE)
    data[OFF_STRTAB:OFF_STRTAB + 4] = b"NTFB"
    data[OFF_WTAB:OFF_WTAB + len(wtab)] = wtab
    data[OFF_BLOB:OFF_BLOB + len(blob)] = blob

    a = Asm()
    B = R12  # permanent data-segment base pointer

    def ld32(reg, off): a.mov_r32_m32(reg, B, off)
    def st32(off, reg): a.mov_m32_r32(B, off, reg)
    def ld8z(reg, off): a.mov_r8_m8(reg, B, off)
    def st8(off, reg): a.mov_m8_r8(B, off, reg)
    def sti8(off, imm): a.mov_m8_imm8(B, off, imm)
    def sti32(off, imm):
        a.mov_r32_imm32(RAX, imm)
        st32(off, RAX)

    def cmp_plen(reg, tmp):
        ld32(tmp, OFF_CPLEN)
        a.cmp_r32_r32(reg, tmp)

    def prog_read(dst, idx_reg, tmp):
        # dst = BLOB[cprogoff + idx_reg]
        ld32(tmp, OFF_CPROGOFF)
        a.add_r32_r32(tmp, idx_reg)
        a.movzx_r32_m8_bi(dst, B, tmp, OFF_BLOB)

    def prog_write0_1(tmp):
        # BLOB[cprogoff + 0] = 1   (the self-modification the reference does)
        ld32(tmp, OFF_CPROGOFF)
        a.mov_m8_imm8_bi(B, tmp, OFF_BLOB, 1)

    # ---- entry: load data base pointer ----
    a.label('_start')
    a.mov_r64_imm64(B, 0)  # patched to data_vaddr after assembly
    DATA_PTR_FIXUP = len(a.code) - 8
    sti32(OFF_WIDX, 0)

    # ================= OUTER: per-word loop =================
    a.label('WORD_TOP')
    ld32(RAX, OFF_WIDX)
    a.cmp_r32_imm32(RAX, nwords)
    a.jae('ALL_DONE')

    # load this word's table entry (stride 16 -> index = widx<<4)
    ld32(R15, OFF_WIDX)
    a.shl_r32_imm8(R15, 4)                       # r15 = widx*16
    a.mov_r32_m32_bi(RCX, B, R15, OFF_WTAB + 0)  # progoff
    st32(OFF_CPROGOFF, RCX)
    a.mov_r32_m32_bi(RCX, B, R15, OFF_WTAB + 4)  # plen
    st32(OFF_CPLEN, RCX)
    a.movzx_r32_m8_bi(RCX, B, R15, OFF_WTAB + 8)
    st8(OFF_CDIV, RCX)
    a.movzx_r32_m8_bi(RCX, B, R15, OFF_WTAB + 9)
    st8(OFF_CSREF, RCX)
    a.movzx_r32_m8_bi(RCX, B, R15, OFF_WTAB + 10)
    st8(OFF_CFROB, RCX)
    a.movzx_r32_m8_bi(RCX, B, R15, OFF_WTAB + 11)
    st8(OFF_CDIAL, RCX)

    # reset per-word state
    sti32(OFF_TOP, 0)
    sti32(OFF_IP, 0)
    sti32(OFF_FDEPTH, 0)
    a.mov_r32_imm32(RAX, 0)
    a.mov_m64_r64(B, OFF_TICK, RAX)
    sti8(OFF_HALTED, 0)
    sti8(OFF_ENGAGR, 0)
    for i in range(8):
        sti8(OFF_REG + i, 0)
    for i in range(4):
        sti8(OFF_MEM + i, 0)

    # cursor register R13 = &outbuf
    a.lea_r64(R13, B, OFF_OUTBUF)

    # ---- per-word header: "--- word N ---\n" (printed via helpers below) ----
    # (print helpers are defined further down but only *emit* here; forward
    #  labels are fine, Asm patches them at finish())

    def advance(n):
        a.lea_r64(R13, R13, n)

    def print_lit(s: bytes):
        for i, ch in enumerate(s):
            a.mov_m8_imm8(R13, i, ch)
        advance(len(s))

    def print_valbyte_from_mem_bi(base, idx, off):
        a.movzx_r32_m8_bi(RAX, base, idx, off)
        a.alu_r32_imm32('and', RAX, 3)
        a.movzx_r32_m8_bi(RAX, B, RAX, OFF_STRTAB)
        a.mov_m8_r8(R13, 0, RAX)
        advance(1)

    pd_counter = [0]

    def print_decimal_u64():
        pd_counter[0] += 1
        n = pd_counter[0]
        LOOP, EMIT, PDONE = f'PD_LOOP_{n}', f'PD_EMIT_{n}', f'PD_DONE_{n}'
        a.emit(bytes([0x48, 0x89, 0xC1]))        # mov rcx, rax
        a.mov_r32_imm32(RDX, 0)
        a.mov_r64_imm64(R8, 0)
        a.label(LOOP)
        a.emit(bytes([0x48, 0x89, 0xC8]))        # mov rax, rcx
        a.mov_r64_imm64(R9, 10)
        a.emit(bytes([0x48, 0x31, 0xD2]))        # xor rdx, rdx
        a.emit(bytes([0x49, 0xF7, 0xF1]))        # div r9
        a.emit(bytes([0x80, 0xC2, 0x30]))        # add dl, '0'
        a.lea_r64_bi(R10, B, R8, OFF_DIGITS)
        a.mov_m8_r8(R10, 0, RDX)
        a.emit(bytes([0x49, 0xFF, 0xC0]))        # inc r8
        a.emit(bytes([0x48, 0x89, 0xC1]))        # mov rcx, rax
        a.emit(bytes([0x48, 0x83, 0xF9, 0x00]))  # cmp rcx, 0
        a.jne(LOOP)
        a.emit(bytes([0x4D, 0x89, 0xC3]))        # mov r11, r8
        a.label(EMIT)
        a.emit(bytes([0x49, 0x83, 0xFB, 0x00]))  # cmp r11, 0
        a.je(PDONE)
        a.emit(bytes([0x49, 0xFF, 0xCB]))        # dec r11
        a.lea_r64_bi(R10, B, R11, OFF_DIGITS)
        a.mov_r8_m8(RAX, R10, 0)
        a.mov_m8_r8(R13, 0, RAX)
        advance(1)
        a.jmp(EMIT)
        a.label(PDONE)

    print_lit(b"--- word ")
    ld32(RAX, OFF_WIDX)
    print_decimal_u64()
    print_lit(b" ---\n")

    # ================= tick loop =================
    a.label('LOOP_TOP')
    a.mov_r64_m64(RAX, B, OFF_TICK)
    a.mov_r64_imm64(RCX, MAX_TICKS)
    a.emit(bytes([0x48, 0x39, 0xC8]))  # cmp rax, rcx
    a.jae('DONE')
    a.inc_m64(B, OFF_TICK)

    # if (ip >= plen) { ip=0; if (top>200) P[0]=1 }
    ld32(RAX, OFF_IP)
    cmp_plen(RAX, R14)
    a.jb('IPOK1')
    sti32(OFF_IP, 0)
    ld32(RAX, OFF_TOP)
    a.cmp_r32_imm32(RAX, 200)
    a.jle('IPOK1')
    prog_write0_1(R14)
    a.label('IPOK1')

    # tok = P[ip]; next_ip = ip+1 (wrap)
    ld32(RCX, OFF_IP)
    prog_read(RAX, RCX, R14)
    a.mov_r32_r32(RDX, RCX)
    a.inc_r32(RDX)
    cmp_plen(RDX, R14)
    a.jb('NEXTOK')
    a.mov_r32_imm32(RDX, 0)
    a.label('NEXTOK')
    NEXT_IP_SLOT = OFF_DIGITS
    st32(NEXT_IP_SLOT, RDX)

    for tok in range(0, 12):
        a.cmp_r32_imm32(RAX, tok)
        a.je(f'TOK_{tok}')
    a.jmp('DISPATCH_END')

    # -- tok 0: VINIT: push N (0) --
    a.label('TOK_0')
    ld32(RCX, OFF_TOP)
    a.cmp_r32_imm32(RCX, SDEPTH)
    a.jae('DISPATCH_END')
    a.mov_m8_imm8_bi(B, RCX, OFF_STACK, 0)
    a.inc_r32(RCX)
    st32(OFF_TOP, RCX)
    a.jmp('DISPATCH_END')

    # -- tok 1: TANCH: pop v; mem[reg0&3]=v; if fdepth==0 halt --
    a.label('TOK_1')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RDX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T1_POPPED')
    a.dec_r32(RCX)
    st32(OFF_TOP, RCX)
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_STACK)
    a.label('T1_POPPED')
    ld8z(RAX, OFF_REG + 0)
    a.alu_r32_imm32('and', RAX, 3)
    a.mov_m8_r8_bi(B, RAX, OFF_MEM, RDX)
    ld32(RAX, OFF_FDEPTH)
    a.cmp_r32_imm32(RAX, 0)
    a.jne('DISPATCH_END')
    sti8(OFF_HALTED, 1)
    a.jmp('DONE')

    # -- tok 2: AFWD: reg0 = (reg0+1)&3 --
    a.label('TOK_2')
    ld8z(RAX, OFF_REG + 0)
    a.inc_r32(RAX)
    a.alu_r32_imm32('and', RAX, 3)
    st8(OFF_REG + 0, RAX)
    a.jmp('DISPATCH_END')

    # -- tok 3: AREV: pop v; push bnot(v); reg0=(reg0-1)&3 --
    a.label('TOK_3')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RDX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T3_POPPED')
    a.dec_r32(RCX)
    st32(OFF_TOP, RCX)
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_STACK)
    a.label('T3_POPPED')
    a.mov_r32_r32(RAX, RDX)
    a.alu_r32_imm32('and', RAX, 1)
    a.shl_r32_imm8(RAX, 1)
    a.mov_r32_r32(R8, RDX)
    a.alu_r32_imm32('and', R8, 2)
    a.shr_r32_imm8(R8, 1)
    a.or_r32_r32(RAX, R8)
    ld32(RCX, OFF_TOP)
    a.cmp_r32_imm32(RCX, SDEPTH)
    a.jae('T3_NOPUSH')
    a.mov_m8_r8_bi(B, RCX, OFF_STACK, RAX)
    a.inc_r32(RCX)
    st32(OFF_TOP, RCX)
    a.label('T3_NOPUSH')
    ld8z(RAX, OFF_REG + 0)
    a.dec_r32(RAX)
    a.alu_r32_imm32('and', RAX, 3)
    st8(OFF_REG + 0, RAX)
    a.jmp('DISPATCH_END')

    # -- tok 4: CLINK: reg3 = reg1 & reg2 --
    a.label('TOK_4')
    ld8z(RAX, OFF_REG + 1)
    ld8z(RCX, OFF_REG + 2)
    a.and_r32_r32(RAX, RCX)
    st8(OFF_REG + 3, RAX)
    a.jmp('DISPATCH_END')

    # -- tok 5: EVALT: pop v; push (v==1)?1:0 --
    a.label('TOK_5')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RDX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T5_POPPED')
    a.dec_r32(RCX)
    st32(OFF_TOP, RCX)
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_STACK)
    a.label('T5_POPPED')
    a.cmp_r32_imm32(RDX, 1)
    a.je('T5_ISONE')
    a.mov_r32_imm32(RAX, 0)
    a.jmp('T5_PUSH')
    a.label('T5_ISONE')
    a.mov_r32_imm32(RAX, 1)
    a.label('T5_PUSH')
    ld32(RCX, OFF_TOP)
    a.cmp_r32_imm32(RCX, SDEPTH)
    a.jae('DISPATCH_END')
    a.mov_m8_r8_bi(B, RCX, OFF_STACK, RAX)
    a.inc_r32(RCX)
    st32(OFF_TOP, RCX)
    a.jmp('DISPATCH_END')

    # -- tok 9: EVALF: pop v; push (v==2)?2:0 --
    a.label('TOK_9')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RDX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T9_POPPED')
    a.dec_r32(RCX)
    st32(OFF_TOP, RCX)
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_STACK)
    a.label('T9_POPPED')
    a.cmp_r32_imm32(RDX, 2)
    a.je('T9_ISTWO')
    a.mov_r32_imm32(RAX, 0)
    a.jmp('T9_PUSH')
    a.label('T9_ISTWO')
    a.mov_r32_imm32(RAX, 2)
    a.label('T9_PUSH')
    ld32(RCX, OFF_TOP)
    a.cmp_r32_imm32(RCX, SDEPTH)
    a.jae('DISPATCH_END')
    a.mov_m8_r8_bi(B, RCX, OFF_STACK, RAX)
    a.inc_r32(RCX)
    st32(OFF_TOP, RCX)
    a.jmp('DISPATCH_END')

    # -- tok 10: ENGAGR: engagr=1; push 3 --
    a.label('TOK_10')
    sti8(OFF_ENGAGR, 1)
    ld32(RCX, OFF_TOP)
    a.cmp_r32_imm32(RCX, SDEPTH)
    a.jae('DISPATCH_END')
    a.mov_m8_imm8_bi(B, RCX, OFF_STACK, 3)
    a.inc_r32(RCX)
    st32(OFF_TOP, RCX)
    a.jmp('DISPATCH_END')

    # -- tok 11: IFIX: pop v; mem[reg0&3]=v --
    a.label('TOK_11')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RDX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T11_POPPED')
    a.dec_r32(RCX)
    st32(OFF_TOP, RCX)
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_STACK)
    a.label('T11_POPPED')
    ld8z(RAX, OFF_REG + 0)
    a.alu_r32_imm32('and', RAX, 3)
    a.mov_m8_r8_bi(B, RAX, OFF_MEM, RDX)
    a.jmp('DISPATCH_END')

    # -- tok 8: IMSCRIB: copy this word's precomputed fields into reg4..7 --
    a.label('TOK_8')
    ld8z(RAX, OFF_CDIV);  st8(OFF_REG + 4, RAX)
    ld8z(RAX, OFF_CSREF); st8(OFF_REG + 5, RAX)
    ld8z(RAX, OFF_CFROB); st8(OFF_REG + 6, RAX)
    ld8z(RAX, OFF_CDIAL); st8(OFF_REG + 7, RAX)
    a.jmp('DISPATCH_END')

    # -- tok 6: FSPLIT --
    a.label('TOK_6')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RAX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T6_HAVEV')
    a.mov_r32_r32(RDX, RCX)
    a.dec_r32(RDX)
    a.movzx_r32_m8_bi(RAX, B, RDX, OFF_STACK)  # peek
    a.label('T6_HAVEV')
    ld32(RCX, OFF_IP)
    a.inc_r32(RCX)
    cmp_plen(RCX, R14)
    a.jb('T6_ISTART')
    a.mov_r32_imm32(RCX, 0)
    a.label('T6_ISTART')
    a.mov_r32_r32(R9, RCX)          # start
    a.mov_r32_imm32(R10, 1)         # depth
    ld32(R11, OFF_CPLEN)            # ff default = plen ("not found")
    a.label('T6_SCAN')
    prog_read(RDX, RCX, R14)
    a.cmp_r32_imm32(RDX, 6)
    a.jne('T6_NOTOPEN')
    a.inc_r32(R10)
    a.jmp('T6_SCANNEXT')
    a.label('T6_NOTOPEN')
    a.cmp_r32_imm32(RDX, 7)
    a.jne('T6_SCANNEXT')
    a.dec_r32(R10)
    a.cmp_r32_imm32(R10, 0)
    a.jne('T6_SCANNEXT')
    a.mov_r32_r32(R11, RCX)
    a.jmp('T6_FOUND')
    a.label('T6_SCANNEXT')
    a.inc_r32(RCX)
    cmp_plen(RCX, R14)
    a.jb('T6_NOWRAP')
    a.mov_r32_imm32(RCX, 0)
    a.label('T6_NOWRAP')
    a.cmp_r32_r32(RCX, R9)
    a.jne('T6_SCAN')
    a.label('T6_FOUND')
    a.mov_r32_r32(RDX, R11)
    a.inc_r32(RDX)
    cmp_plen(RDX, R14)
    a.jb('T6_RESOK')
    a.mov_r32_imm32(RDX, 0)
    a.label('T6_RESOK')
    ld32(RCX, OFF_FDEPTH)
    a.cmp_r32_imm32(RCX, FCAP)
    a.jae('T6_NOPUSHFRAME')
    a.mov_m32_r32_bi(B, RCX, OFF_FRESUME, RDX, scale=4)
    a.mov_m8_imm8_bi(B, RCX, OFF_FRIGHT, 0)
    a.mov_m8_imm8_bi(B, RCX, OFF_FSET, 0)
    a.inc_r32(RCX)
    st32(OFF_FDEPTH, RCX)
    a.label('T6_NOPUSHFRAME')
    ld32(RCX, OFF_FDEPTH)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T6_NOTOPSET')
    a.dec_r32(RCX)
    a.mov_m8_r8_bi(B, RCX, OFF_FRIGHT, RAX)
    a.mov_m8_imm8_bi(B, RCX, OFF_FSET, 1)
    a.label('T6_NOTOPSET')
    ld32(RCX, OFF_TOP)
    a.cmp_r32_imm32(RCX, SDEPTH)
    a.jae('DISPATCH_END')
    a.mov_m8_r8_bi(B, RCX, OFF_STACK, RAX)
    a.inc_r32(RCX)
    st32(OFF_TOP, RCX)
    a.jmp('DISPATCH_END')

    # -- tok 7: FFUSE --
    a.label('TOK_7')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RAX, 0)   # left
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T7_HAVELEFT')
    a.dec_r32(RCX)
    st32(OFF_TOP, RCX)
    a.movzx_r32_m8_bi(RAX, B, RCX, OFF_STACK)
    a.label('T7_HAVELEFT')
    ld32(RCX, OFF_FDEPTH)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T7_NOFRAME')
    a.dec_r32(RCX)
    st32(OFF_FDEPTH, RCX)
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_FSET)
    a.mov_r32_imm32(R8, 0)
    a.cmp_r32_imm32(RDX, 0)
    a.je('T7_RIGHTZERO')
    a.movzx_r32_m8_bi(R8, B, RCX, OFF_FRIGHT)
    a.label('T7_RIGHTZERO')
    a.alu_r32_imm32('and', R8, 3)
    a.or_r32_r32(RAX, R8)
    ld32(RDX, OFF_TOP)
    a.cmp_r32_imm32(RDX, SDEPTH)
    a.jae('T7_NOPUSH2')
    a.mov_m8_r8_bi(B, RDX, OFF_STACK, RAX)
    a.inc_r32(RDX)
    st32(OFF_TOP, RDX)
    a.label('T7_NOPUSH2')
    a.mov_r32_m32_bi(RDX, B, RCX, OFF_FRESUME, scale=4)
    st32(NEXT_IP_SLOT, RDX)
    a.jmp('DISPATCH_END')
    a.label('T7_NOFRAME')
    ld32(RDX, OFF_TOP)
    a.cmp_r32_imm32(RDX, SDEPTH)
    a.jae('DISPATCH_END')
    a.mov_m8_r8_bi(B, RDX, OFF_STACK, RAX)
    a.inc_r32(RDX)
    st32(OFF_TOP, RDX)
    a.jmp('DISPATCH_END')

    a.label('DISPATCH_END')
    ld32(RAX, NEXT_IP_SLOT)
    st32(OFF_IP, RAX)
    ld32(RAX, OFF_IP)
    cmp_plen(RAX, R14)
    a.jb('IPOK2')
    sti32(OFF_IP, 0)
    ld32(RAX, OFF_TOP)
    a.cmp_r32_imm32(RAX, 200)
    a.jle('IPOK2')
    prog_write0_1(R14)
    a.label('IPOK2')
    a.jmp('LOOP_TOP')

    a.label('DONE')

    # ================= per-word result block =================
    print_lit(b"halted: ")
    ld8z(RAX, OFF_HALTED)
    a.cmp_r32_imm32(RAX, 0)
    a.je('PR_HFALSE')
    print_lit(b"true")
    a.jmp('PR_HDONE')
    a.label('PR_HFALSE')
    print_lit(b"false")
    a.label('PR_HDONE')
    print_lit(b"   ticks: ")
    a.mov_r64_m64(RAX, B, OFF_TICK)
    print_decimal_u64()
    print_lit(b"\n")

    print_lit(b"stack (bottom..top): [")
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RDX, 0)
    a.label('PS_LOOP')
    a.cmp_r32_r32(RDX, RCX)
    a.jae('PS_DONE')
    a.cmp_r32_imm32(RDX, 0)
    a.je('PS_NOSPACE')
    print_lit(b" ")
    a.label('PS_NOSPACE')
    print_valbyte_from_mem_bi(B, RDX, OFF_STACK)
    a.inc_r32(RDX)
    a.jmp('PS_LOOP')
    a.label('PS_DONE')
    print_lit(b"]  depth ")
    ld32(RAX, OFF_TOP)
    print_decimal_u64()
    print_lit(b"\n")

    print_lit(b"registers: [")
    for i in range(8):
        if i:
            print_lit(b" ")
        a.mov_r8_m8(RAX, B, OFF_REG + i)
        a.alu_r32_imm32('and', RAX, 3)
        a.movzx_r32_m8_bi(RAX, B, RAX, OFF_STRTAB)
        a.mov_m8_r8(R13, 0, RAX)
        advance(1)
    print_lit(b"]  engagr ")
    ld8z(RAX, OFF_ENGAGR)
    a.cmp_r32_imm32(RAX, 0)
    a.je('PR_EFALSE')
    print_lit(b"true")
    a.jmp('PR_EDONE')
    a.label('PR_EFALSE')
    print_lit(b"false")
    a.label('PR_EDONE')
    print_lit(b"\n")

    print_lit(b"memory[0..4]: [")
    for i in range(4):
        if i:
            print_lit(b" ")
        a.mov_r8_m8(RAX, B, OFF_MEM + i)
        a.alu_r32_imm32('and', RAX, 3)
        a.movzx_r32_m8_bi(RAX, B, RAX, OFF_STRTAB)
        a.mov_m8_r8(R13, 0, RAX)
        advance(1)
    print_lit(b"]\n\n")

    # write(1, outbuf, R13 - (B+OFF_OUTBUF))
    a.emit(bytes([0x4C, 0x89, 0xE8]))  # mov rax, r13
    a.lea_r64(RCX, B, OFF_OUTBUF)
    a.emit(bytes([0x48, 0x29, 0xC8]))  # sub rax, rcx
    a.emit(bytes([0x48, 0x89, 0xC2]))  # mov rdx, rax  (len)
    a.lea_r64(RSI, B, OFF_OUTBUF)
    a.mov_r32_imm32(RDI, 1)
    a.mov_r32_imm32(RAX, 1)
    a.syscall()

    # widx++ ; next word
    ld32(RAX, OFF_WIDX)
    a.inc_r32(RAX)
    st32(OFF_WIDX, RAX)
    a.jmp('WORD_TOP')

    # ================= all words done =================
    a.label('ALL_DONE')
    a.mov_r32_imm32(RDI, 0)
    a.mov_r32_imm32(RAX, 60)
    a.syscall()

    code = a.finish()
    entry_off = a.labels['_start']

    blob_out, code_va, data_va = build_elf64_exec(code, bytes(data), entry_off)
    blob_out = bytearray(blob_out)
    code_start_in_file = 64 + 56 * 2
    struct.pack_into('<Q', blob_out, code_start_in_file + DATA_PTR_FIXUP, data_va)

    with open(out_path, 'wb') as f:
        f.write(blob_out)
    os.chmod(out_path, 0o755)
    return nwords, len(blob), len(code), DATA_SIZE


if __name__ == '__main__':
    here = os.path.dirname(os.path.abspath(__file__))
    imasm_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(here, 'imasm')
    out_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(here, 'gmomonados.imasm.bin')
    words = collect_words(imasm_dir)
    if not words:
        print(f"no words found under {imasm_dir}", file=sys.stderr)
        sys.exit(1)
    nwords, blob_len, code_len, data_len = build(words, out_path)
    print(f"linked {nwords} words ({blob_len} marks) -> {out_path}")
    print(f"  code {code_len} bytes, data {data_len} bytes, one executable")
