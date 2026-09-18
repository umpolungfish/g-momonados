#ifndef PROGRAM_CAP
#define PROGRAM_CAP 64
#endif

// One IMASM twelve-mark program run to a halt or max_ticks. P is the program
// (mutable: ACT may self-modify a wrapped slot), plen its length. out_stack
// must have room for 256 marks, out_meta for 32 words. This is the single copy
// of the stepper: the batch kernel run_programs and the resident OS kernel both
// call it, so the card has one engine, not two.
__device__ void imasm_exec_one(unsigned char* P, unsigned int plen,
                               unsigned long long max_ticks,
                               unsigned char* out_stack, unsigned int* out_meta)
{
    const int SDEPTH = 256, FCAP = 64;
    unsigned char stack[256]; int top = 0;
    unsigned char reg[8]; for (int i=0;i<8;i++) reg[i]=0;
    unsigned char engagr = 0;
    unsigned char mem[4]; mem[0]=mem[1]=mem[2]=mem[3]=0;
    unsigned int fresume[64]; unsigned char fright[64]; unsigned char fset[64];
    int fdepth = 0;
    unsigned int ip = 0;
    unsigned long long tick = 0;
    unsigned char halted = 0;

    // IMSCRIB reads four snapshot fields; all are functions of the program,
    // which is static over a non-dynamic run, so compute them once.
    int diversity = 0;
    { unsigned char seen[16]; for (int i=0;i<16;i++) seen[i]=0;
      for (unsigned int i=0;i<plen;i++){ unsigned char t=P[i]; if (t<16 && !seen[t]) { seen[t]=1; diversity++; } } }
    unsigned char self_ref = (plen>0 && P[0]==P[plen-1]) ? 1 : 0;
    unsigned char frob_pos = 0;
    for (unsigned int i=0;i<plen;i++){ unsigned char t=P[i]; if (t==6||t==7||t==12||t==13){ frob_pos=1; break; } }
    unsigned char dial = 0;
    { unsigned char h5=0,h9=0,h10=0;
      for (unsigned int i=0;i<plen;i++){ if (P[i]==5)h5=1; else if (P[i]==9)h9=1; else if (P[i]==10)h10=1; }
      if (h5 && h9 && h10) {
          dial = 1;
          for (unsigned int i=0;i<plen && dial;i++){
              if (P[i]==10){
                  unsigned char found=0;
                  for (unsigned int off=1; off<plen; off++){ unsigned char t=P[(i+off)%plen]; if (t==5||t==9){ found=1; break; } }
                  if (!found) dial=0;
              }
          }
      }
    }

    while (!halted && tick < max_ticks && plen > 0) {
        tick++;
        // ACT: wrap + try_self_modify (non-dynamic: inject TANCH if stack deep)
        if (ip >= plen) { ip = 0; if (top > 200) P[ip] = 1; }
        unsigned char tok = P[ip];
        unsigned int next_ip = ip + 1;
        if (next_ip >= plen) next_ip = 0;   // winding count omitted (internal)

        if (tok == 0) {                     // VINIT: push N
            if (top < SDEPTH) stack[top++] = 0;
        } else if (tok == 1) {              // TANCH
            unsigned char v = (top>0)? stack[--top] : 0;
            mem[reg[0] & 3] = v;
            if (fdepth == 0) { halted = 1; break; }
        } else if (tok == 2) {              // AFWD
            reg[0] = (reg[0] + 1) & 3;
        } else if (tok == 3) {              // AREV: bnot top, dec reg0
            unsigned char v = (top>0)? stack[--top] : 0;
            unsigned char nb = ((v & 1) << 1) | ((v & 2) >> 1);
            if (top < SDEPTH) stack[top++] = nb;
            reg[0] = (reg[0] - 1) & 3;
        } else if (tok == 4) {              // CLINK: reg3 = meet(reg1,reg2)
            reg[3] = reg[1] & reg[2];
        } else if (tok == 5) {              // EVALT
            unsigned char v = (top>0)? stack[--top] : 0;
            if (top < SDEPTH) stack[top++] = (v == 1) ? 1 : 0;
        } else if (tok == 6) {              // FSPLIT
            unsigned char v = (top>0)? stack[top-1] : 0;
            // find matching FFUSE
            unsigned int ff = plen; unsigned int depth = 1;
            unsigned int i = (ip + 1) % plen; unsigned int start = i;
            do {
                if (P[i] == 6) depth++;
                else if (P[i] == 7) { depth--; if (depth==0) { ff = i; break; } }
                i = (i + 1) % plen;
            } while (i != start);
            unsigned int resume = (ff + 1 >= plen) ? 0 : ff + 1;
            // push_fork: add a frame only if there is room (overflow drops it)
            if (fdepth < FCAP) { fresume[fdepth]=resume; fright[fdepth]=0; fset[fdepth]=0; fdepth++; }
            // fork_top_mut: set the (new or, on overflow, existing) top frame's right
            if (fdepth > 0) { fright[fdepth-1]=v; fset[fdepth-1]=1; }
            if (top < SDEPTH) stack[top++] = v;
        } else if (tok == 7) {              // FFUSE
            unsigned char left = (top>0)? stack[--top] : 0;
            if (fdepth > 0) {
                fdepth--;
                unsigned char right = fset[fdepth] ? fright[fdepth] : 0;
                if (top < SDEPTH) stack[top++] = left | (right & 3);  // b4_join(left, collapse(right))
                next_ip = fresume[fdepth];
            } else {
                if (top < SDEPTH) stack[top++] = left;
            }
        } else if (tok == 12) {             // FSPLIT3 (three-way delta)
            unsigned char v = (top>0)? stack[top-1] : 0;   // B4
            unsigned char r = v & 3;                        // b4_to_reg16_3, small bits 0
            unsigned int ff = plen; unsigned int depth = 1;
            unsigned int i = (ip + 1) % plen; unsigned int start = i;
            do {
                if (P[i] == 12) depth++;
                else if (P[i] == 13) { depth--; if (depth==0) { ff = i; break; } }
                i = (i + 1) % plen;
            } while (i != start);
            unsigned int resume = (ff + 1 >= plen) ? 0 : ff + 1;
            // constructive_part(r).falsity_part().union(info_part(r)), as 4-bit T,F,t,f
            unsigned char rv = ((r & 3) & 0xA) | (r & 0xC);
            if (fdepth < FCAP) { fresume[fdepth]=resume; fright[fdepth]=0; fset[fdepth]=0; fdepth++; }
            if (fdepth > 0) { fright[fdepth-1]=rv; fset[fdepth-1]=1; }
            if (top < SDEPTH) stack[top++] = v;
        } else if (tok == 13) {             // FFUSE3 (union of arms)
            unsigned char left = (top>0)? stack[--top] : 0;
            if (fdepth > 0) {
                fdepth--;
                unsigned char right = fset[fdepth] ? fright[fdepth] : 0;
                unsigned char fused = (left & 3) | right;   // union in 4-bit
                if (top < SDEPTH) stack[top++] = fused & 3;  // reg16_3_to_b4
                next_ip = fresume[fdepth];
            } else {
                if (top < SDEPTH) stack[top++] = left;
            }
        } else if (tok == 14) {             // EVALI (info part)
            unsigned char v = (top>0)? stack[--top] : 0;
            unsigned char info = (v & 3) & 0xC;   // info_part of a B4 value is empty
            if (top < SDEPTH) stack[top++] = info & 3;
        } else if (tok == 8) {              // IMSCRIB: snapshot fields to reg4-7
            reg[4] = (unsigned char)(diversity & 3);
            reg[5] = self_ref ? 1 : 2;   // T : F
            reg[6] = frob_pos ? 1 : 2;
            reg[7] = dial ? 1 : 2;
        } else if (tok == 9) {              // EVALF
            unsigned char v = (top>0)? stack[--top] : 0;
            if (top < SDEPTH) stack[top++] = (v == 2) ? 2 : 0;
        } else if (tok == 10) {             // ENGAGR
            engagr = 1;
            if (top < SDEPTH) stack[top++] = 3;
        } else if (tok == 11) {             // IFIX
            unsigned char v = (top>0)? stack[--top] : 0;
            mem[reg[0] & 3] = v;
        } else if (tok == 15) {             // ROTAT: rotate stack right by k
            unsigned char kv = (top>0)? stack[--top] : 0;
            int k = (int)kv; if (k < 1) k = 1;
            int nn = top;
            if (nn > 1) {
                k = k % nn; if (k < 0) k += nn;
                if (k > 0) {
                    // reverse [0,nn), reverse [0,k), reverse [k,nn)
                    for (int a=0,b=nn-1; a<b; a++,b--) { unsigned char t=stack[a]; stack[a]=stack[b]; stack[b]=t; }
                    for (int a=0,b=k-1;  a<b; a++,b--) { unsigned char t=stack[a]; stack[a]=stack[b]; stack[b]=t; }
                    for (int a=k,b=nn-1; a<b; a++,b--) { unsigned char t=stack[a]; stack[a]=stack[b]; stack[b]=t; }
                }
            }
        }
        // tok==8 (IMSCRIB) and 12/13/14 (three-way) are excluded from verified programs.

        ip = next_ip;
        // UPDATE: wrap + try_self_modify again
        if (ip >= plen) { ip = 0; if (top > 200) P[ip] = 1; }
    }

    for (int i = 0; i < top; i++) out_stack[i] = stack[i];
    unsigned int* m = out_meta;
    m[0] = halted;
    m[1] = (unsigned int)tick;
    m[2] = (unsigned int)top;
    for (int i=0;i<8;i++) m[3+i] = reg[i];
    m[11] = engagr;
    for (int i=0;i<4;i++) m[12+i] = mem[i];
}
