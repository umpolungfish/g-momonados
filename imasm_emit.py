#!/usr/bin/env python3
"""imasm_emit.py — compile an IMASM glyph word straight to x86-64 machine
code and a hand-written ELF64 executable. No gcc, no as, no ld: every
instruction is chosen and encoded by imasm_asm.Asm; the file bytes come
from imasm_asm.build_elf64_exec.

The semantics compiled are the exact tick loop kernel.rs / gpu_kernel.rs's
CUDA kernel both run (VINIT..IFIX, tokens 0-11; FSPLIT3/FFUSE3/EVALI/ROTAT
are excluded, same as vox's own lift, which never emits them).
"""
import sys, os
from imasm_asm import Asm, build_elf64_exec, RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI, R8, R9, R10, R11, R12, R13, R14, R15

GLYPH_TO_ID = {
    '⊢': 0, '⊣': 1, '≻': 2, '≺': 3, '⋈': 4, '⊤': 5,
    '∈': 6, '∋': 7, '⊙': 8, '⊥': 9, '⊞': 10, '⊡': 11,
}

MAX_TICKS = 65536
SDEPTH = 256
FCAP = 64


def compile_word(word: str, out_path: str):
    ids = []
    for c in word:
        if c not in GLYPH_TO_ID:
            raise ValueError(f"not a compilable mark: {c!r}")
        ids.append(GLYPH_TO_ID[c])
    plen = len(ids)

    # ---- static precompute (IMSCRIB's four snapshot fields; computed from
    # the INITIAL program, exactly as the reference does before its loop) ----
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

    # ---- data layout ----
    OFF_PROG = 0
    OFF_STACK = OFF_PROG + plen
    OFF_REG = OFF_STACK + SDEPTH
    OFF_ENGAGR = OFF_REG + 8
    OFF_MEM = OFF_ENGAGR + 1
    OFF_FRESUME = OFF_MEM + 4          # u32[64]
    OFF_FRIGHT = OFF_FRESUME + FCAP * 4
    OFF_FSET = OFF_FRIGHT + FCAP
    OFF_TOP = OFF_FSET + FCAP          # u32
    OFF_IP = OFF_TOP + 4               # u32
    OFF_FDEPTH = OFF_IP + 4            # u32
    OFF_TICK = OFF_FDEPTH + 4          # u64
    OFF_HALTED = OFF_TICK + 8          # u8
    OFF_OUTBUF = OFF_HALTED + 1
    OUTBUF_CAP = 4096
    OFF_STRTAB = OFF_OUTBUF + OUTBUF_CAP   # "NTFB"
    OFF_DIGITS = OFF_STRTAB + 4             # scratch, 20 bytes
    DATA_SIZE = OFF_DIGITS + 20

    data = bytearray(DATA_SIZE)
    for i, t in enumerate(ids):
        data[OFF_PROG + i] = t
    data[OFF_STRTAB:OFF_STRTAB + 4] = b"NTFB"

    a = Asm()
    B = R12  # permanent data-segment base pointer

    def ld32(reg, off):
        a.mov_r32_m32(reg, B, off)

    def st32(off, reg):
        a.mov_m32_r32(B, off, reg)

    def ld8z(reg, off):
        a.mov_r8_m8(reg, B, off)

    def st8(off, reg):
        a.mov_m8_r8(B, off, reg)

    def sti8(off, imm):
        a.mov_m8_imm8(B, off, imm)

    def sti32(off, imm):
        a.mov_r32_imm32(RAX, imm)
        st32(off, RAX)

    # ---- entry: load data base pointer ----
    a.label('_start')
    a.mov_r64_imm64(B, 0)  # patched to data_vaddr after assembly
    DATA_PTR_FIXUP = len(a.code) - 8

    # init: top=0, ip=0, fdepth=0, tick=0, halted=0, engagr=0
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
    # cursor register R13 = &outbuf, advances as we print
    a.lea_r64(R13, B, OFF_OUTBUF)

    # ---- main tick loop ----
    a.label('LOOP_TOP')
    # tick < MAX_TICKS ?
    a.mov_r64_m64(RAX, B, OFF_TICK)
    a.mov_r64_imm64(RCX, MAX_TICKS)
    a.cmp_r64 = None  # placeholder, real cmp below
    a.emit(bytes([0x48, 0x39, 0xC8]))  # cmp rax, rcx
    a.jae('DONE')

    # tick++
    a.inc_m64(B, OFF_TICK)

    # if (ip >= plen) { ip=0; if (top>200) P[0]=1 }
    ld32(RAX, OFF_IP)
    a.cmp_r32_imm32(RAX, plen)
    a.jb('IPOK1')
    sti32(OFF_IP, 0)
    ld32(RAX, OFF_TOP)
    a.cmp_r32_imm32(RAX, 200)
    a.jle('IPOK1')
    sti8(OFF_PROG + 0, 1)
    a.label('IPOK1')

    # tok = P[ip]; next_ip = ip+1 (wrap)
    ld32(RCX, OFF_IP)                        # ecx = ip (also used as index)
    a.movzx_r32_m8_bi(RAX, B, RCX, OFF_PROG)  # eax = tok
    a.mov_r32_r32(RDX, RCX)
    a.inc_r32(RDX)
    a.cmp_r32_imm32(RDX, plen)
    a.jb('NEXTOK')
    a.mov_r32_imm32(RDX, 0)
    a.label('NEXTOK')
    st32(OFF_TICK - OFF_TICK + OFF_FDEPTH - OFF_FDEPTH + OFF_TOP - OFF_TOP + 0, RDX) if False else None
    # stash next_ip in a scratch memory slot: reuse OFF_DIGITS[0..4) (not used until printing)
    NEXT_IP_SLOT = OFF_DIGITS
    st32(NEXT_IP_SLOT, RDX)

    # dispatch
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
    # nb = ((v&1)<<1) | ((v&2)>>1)
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

    # -- tok 8: IMSCRIB: precomputed constants --
    a.label('TOK_8')
    sti8(OFF_REG + 4, diversity & 3)
    sti8(OFF_REG + 5, 1 if self_ref else 2)
    sti8(OFF_REG + 6, 1 if frob_pos else 2)
    sti8(OFF_REG + 7, 1 if dial else 2)
    a.jmp('DISPATCH_END')

    # -- tok 6: FSPLIT --
    a.label('TOK_6')
    ld32(RCX, OFF_TOP)
    a.mov_r32_imm32(RAX, 0)
    a.cmp_r32_imm32(RCX, 0)
    a.jle('T6_HAVEV')
    a.mov_r32_r32(RDX, RCX)
    a.dec_r32(RDX)
    a.movzx_r32_m8_bi(RAX, B, RDX, OFF_STACK)  # v = stack[top-1] (peek, no pop)
    a.label('T6_HAVEV')
    # search for matching FFUSE starting at i=(ip+1)%plen
    ld32(RCX, OFF_IP)
    a.inc_r32(RCX)
    a.cmp_r32_imm32(RCX, plen)
    a.jb('T6_ISTART')
    a.mov_r32_imm32(RCX, 0)
    a.label('T6_ISTART')
    a.mov_r32_r32(R9, RCX)          # r9 = start
    a.mov_r32_imm32(R10, 1)         # r10 = depth
    a.mov_r32_imm32(R11, plen)      # r11 = ff (default: plen, "not found")
    a.label('T6_SCAN')
    a.movzx_r32_m8_bi(RDX, B, RCX, OFF_PROG)
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
    a.cmp_r32_imm32(RCX, plen)
    a.jb('T6_NOWRAP')
    a.mov_r32_imm32(RCX, 0)
    a.label('T6_NOWRAP')
    a.cmp_r32_r32(RCX, R9)
    a.jne('T6_SCAN')
    a.label('T6_FOUND')
    # resume = (ff+1 >= plen) ? 0 : ff+1
    a.mov_r32_r32(RDX, R11)
    a.inc_r32(RDX)
    a.cmp_r32_imm32(RDX, plen)
    a.jb('T6_RESOK')
    a.mov_r32_imm32(RDX, 0)
    a.label('T6_RESOK')
    # push fork frame if fdepth<FCAP
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
    # push v
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
    # next_ip = fresume[fdepth]  (fdepth already decremented, in rcx)
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
    # ip = next_ip; if (ip>=plen) { ip=0; if(top>200) P[0]=1 }
    ld32(RAX, NEXT_IP_SLOT)
    st32(OFF_IP, RAX)
    ld32(RAX, OFF_IP)
    a.cmp_r32_imm32(RAX, plen)
    a.jb('IPOK2')
    sti32(OFF_IP, 0)
    ld32(RAX, OFF_TOP)
    a.cmp_r32_imm32(RAX, 200)
    a.jle('IPOK2')
    sti8(OFF_PROG + 0, 1)
    a.label('IPOK2')
    a.jmp('LOOP_TOP')

    a.label('DONE')

    # ---- output formatting ----
    def emit_str(s: bytes):
        for i, ch in enumerate(s):
            a.mov_m8_imm8(R13, i, ch)
        a.alu_r32_imm32('add', R13, len(s)) if False else None
        # advance R13 by len(s) (R13 is 64-bit; use lea for a clean add)
        a.lea_r64(R13, R13, 0)  # no-op placeholder removed below

    def advance(n):
        # R13 += n  (lea r13, [r13+n])
        a.lea_r64(R13, R13, n)

    def print_lit(s: bytes):
        for i, ch in enumerate(s):
            a.mov_m8_imm8(R13, i, ch)
        advance(len(s))

    def print_val_reg(reg_holding_index_addr_val):
        # writes STRTAB[val & 3] as one char at [R13], advances 1
        # val already in AL (low byte) of given reg
        pass

    def print_valbyte_from_mem_bi(base, idx, off):
        a.movzx_r32_m8_bi(RAX, base, idx, off)
        a.alu_r32_imm32('and', RAX, 3)
        a.movzx_r32_m8_bi(RAX, B, RAX, OFF_STRTAB)
        a.mov_m8_r8(R13, 0, RAX)
        advance(1)

    pd_counter = [0]

    def print_decimal_u64():
        # value in RAX (u64); writes decimal digits at [R13], advances
        pd_counter[0] += 1
        n = pd_counter[0]
        LOOP, EMIT, DONE = f'PD_LOOP_{n}', f'PD_EMIT_{n}', f'PD_DONE_{n}'
        a.emit(bytes([0x48, 0x89, 0xC1]))  # mov rcx, rax  (save value)
        a.mov_r32_imm32(RDX, 0)
        a.mov_r64_imm64(R8, 0)  # digit count
        a.label(LOOP)
        a.emit(bytes([0x48, 0x89, 0xC8]))  # mov rax, rcx
        a.mov_r64_imm64(R9, 10)
        a.emit(bytes([0x48, 0x31, 0xD2]))  # xor rdx, rdx
        a.emit(bytes([0x49, 0xF7, 0xF1]))  # div r9  (REX.B set: rm=001+8=r9; rax=rax/r9 rem rdx)
        a.emit(bytes([0x80, 0xC2, 0x30]))  # add dl, '0'
        a.lea_r64_bi(R10, B, R8, OFF_DIGITS)
        a.mov_m8_r8(R10, 0, RDX)
        a.emit(bytes([0x49, 0xFF, 0xC0]))  # inc r8
        a.emit(bytes([0x48, 0x89, 0xC1]))  # mov rcx, rax
        a.emit(bytes([0x48, 0x83, 0xF9, 0x00]))  # cmp rcx, 0
        a.jne(LOOP)
        # copy digits[0..r8) reversed to output
        a.emit(bytes([0x4D, 0x89, 0xC3]))  # mov r11, r8   (count)
        a.label(EMIT)
        a.emit(bytes([0x49, 0x83, 0xFB, 0x00]))  # cmp r11, 0
        a.je(DONE)
        a.emit(bytes([0x49, 0xFF, 0xCB]))  # dec r11
        a.lea_r64_bi(R10, B, R11, OFF_DIGITS)
        a.mov_r8_m8(RAX, R10, 0)
        a.mov_m8_r8(R13, 0, RAX)
        advance(1)
        a.jmp(EMIT)
        a.label(DONE)

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
    a.mov_r32_imm32(RDX, 0)  # index i
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
    a.emit(bytes([0x89, 0xC0]))  # mov eax, eax (zero-extend already since 32-bit op clears upper)
    print_decimal_u64()
    print_lit(b"\n")

    print_lit(b"registers: [")
    for i in range(8):
        if i:
            print_lit(b" ")
        a.movzx_r32_m8_bi(RAX, B, RSP, OFF_REG + i) if False else None
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
    print_lit(b"]\n")

    # outlen = R13 - (B + OFF_OUTBUF)
    a.emit(bytes([0x4C, 0x89, 0xE8]))  # mov rax, r13
    a.lea_r64(RCX, B, OFF_OUTBUF)
    a.emit(bytes([0x48, 0x29, 0xC8]))  # sub rax, rcx

    # write(1, outbuf, rax)
    a.emit(bytes([0x48, 0x89, 0xC2]))  # mov rdx, rax   (len)
    a.lea_r64(RSI, B, OFF_OUTBUF)
    a.mov_r32_imm32(RDI, 1)
    a.mov_r32_imm32(RAX, 1)
    a.syscall()

    # exit(halted ? 0 : 1)
    ld8z(RDI, OFF_HALTED)
    a.cmp_r32_imm32(RDI, 0)
    a.jne('EXIT_OK')
    a.mov_r32_imm32(RDI, 1)
    a.jmp('EXIT_GO')
    a.label('EXIT_OK')
    a.mov_r32_imm32(RDI, 0)
    a.label('EXIT_GO')
    a.mov_r32_imm32(RAX, 60)
    a.syscall()

    code = a.finish()
    entry_off = a.labels['_start']

    blob, code_va, data_va = build_elf64_exec(code, bytes(data), entry_off)
    # patch the data pointer immediate now that we know data_va
    import struct
    blob = bytearray(blob)
    code_start_in_file = 64 + 56 * 2
    struct.pack_into('<Q', blob, code_start_in_file + DATA_PTR_FIXUP, data_va)

    with open(out_path, 'wb') as f:
        f.write(blob)
    os.chmod(out_path, 0o755)
    return len(ids)


if __name__ == '__main__':
    if len(sys.argv) != 4:
        print("usage: imasm_emit.py <file.imasm> <line 1-based> <out>")
        sys.exit(1)
    imasm_file, lineno, out_path = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    with open(imasm_file, encoding='utf-8') as f:
        lines = [l.rstrip('\n') for l in f if l.strip()]
    word = lines[lineno - 1]
    n = compile_word(word, out_path)
    print(f"compiled {imasm_file}:{lineno} ({n} marks) -> {out_path}")
