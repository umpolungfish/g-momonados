#define MF_LIMBS 32
#define MF_WIDE (2 * MF_LIMBS)

__device__ __forceinline__ int mf_cmp(const unsigned *a, const unsigned *b, int n) {
    for (int i = n - 1; i >= 0; --i) {
        if (a[i] < b[i]) return -1;
        if (a[i] > b[i]) return 1;
    }
    return 0;
}

__device__ __forceinline__ unsigned mf_add_small(unsigned *a, int n, unsigned v) {
    unsigned long long c = v;
    for (int i = 0; i < n && c; ++i) {
        c += a[i]; a[i] = (unsigned)c; c >>= 32;
    }
    return (unsigned)c;
}

__device__ __forceinline__ unsigned mf_sub(unsigned *a, const unsigned *b, int n) {
    unsigned long long borrow = 0;
    for (int i = 0; i < n; ++i) {
        unsigned long long x = a[i], y = (unsigned long long)b[i] + borrow;
        a[i] = (unsigned)(x - y); borrow = x < y;
    }
    return (unsigned)borrow;
}

__device__ void mf_mul(const unsigned *a, const unsigned *b, unsigned *out, int n) {
    for (int i = 0; i < 2 * n; ++i) out[i] = 0;
    for (int i = 0; i < n; ++i) {
        unsigned long long carry = 0;
        for (int j = 0; j < n; ++j) {
            unsigned long long v = (unsigned long long)a[i] * b[j] + out[i+j] + carry;
            out[i+j] = (unsigned)v; carry = v >> 32;
        }
        int k = i + n;
        while (carry && k < 2*n) {
            unsigned long long v = (unsigned long long)out[k] + carry;
            out[k++] = (unsigned)v; carry = v >> 32;
        }
    }
}

__device__ bool mf_isqrt(const unsigned *x, int n, unsigned *root, unsigned *rem) {
    for (int i = 0; i < 2*n; ++i) root[i] = 0;
    for (int i = 0; i < 2*n+1; ++i) rem[i] = 0;
    unsigned trial[2*MF_LIMBS+1];
    const int pairs = 16 * (2*n);
    for (int pair = pairs - 1; pair >= 0; --pair) {
        unsigned bit0 = (x[(2*pair)/32] >> ((2*pair)%32)) & 1u;
        unsigned bit1 = (x[(2*pair+1)/32] >> ((2*pair+1)%32)) & 1u;
        unsigned carry = (bit1 << 1) | bit0;
        for (int i = 0; i < 2*n+1; ++i) {
            unsigned next = rem[i] >> 30;
            rem[i] = (rem[i] << 2) | carry;
            carry = next;
        }
        unsigned c = 0;
        for (int i = 0; i < 2*n; ++i) {
            unsigned next = root[i] >> 31;
            root[i] = (root[i] << 1) | c; c = next;
        }
        c = 1;
        for (int i = 0; i < 2*n+1; ++i) {
            unsigned long long v = ((unsigned long long)root[i] << 1) + c;
            trial[i] = (unsigned)v; c = (unsigned)(v >> 32);
        }
        if (mf_cmp(rem, trial, 2*n+1) >= 0) {
            mf_sub(rem, trial, 2*n+1);
            mf_add_small(root, 2*n, 1u);
        }
    }
    for (int i = 0; i < 2*n+1; ++i) if (rem[i]) return false;
    return true;
}

extern "C" __global__ void membrane_prepare(
    const unsigned *nval, int nlimbs, unsigned *start, unsigned *hi)
{
    if (blockIdx.x || threadIdx.x || nlimbs > MF_LIMBS) return;
    unsigned wide[MF_WIDE] = {0}, root[MF_WIDE] = {0}, rem[MF_WIDE+1] = {0};
    for (int i=0;i<nlimbs;++i) wide[i]=nval[i];
    mf_isqrt(wide,nlimbs,root,rem);
    for (int i=0;i<nlimbs;++i) start[i]=root[i];
    unsigned carry=0;
    for (int i=nlimbs-1;i>=0;--i) {
        unsigned next=wide[i]&1u;
        hi[i]=(wide[i]>>1)|(carry<<31);
        carry=next;
    }
}

extern "C" __global__ void membrane_word_factor(
    const unsigned *nval, const unsigned *hi, int nlimbs,
    const unsigned *ops, int nops, const unsigned *start,
    unsigned long long count, unsigned long long *winner, unsigned long long *ticks,
    unsigned *out_p, unsigned *out_q)
{
    unsigned long long lane = (unsigned long long)blockIdx.x * blockDim.x + threadIdx.x;
    if (lane >= count || nlimbs > MF_LIMBS) return;
    unsigned a[MF_LIMBS] = {0}, delta_a[MF_LIMBS] = {0};
    unsigned sq[MF_WIDE] = {0}, delta[MF_WIDE] = {0};
    unsigned root[MF_WIDE] = {0}, rem[MF_WIDE+1] = {0};
    unsigned p[MF_WIDE] = {0}, q[MF_WIDE] = {0}, product[MF_WIDE] = {0};
    unsigned nwide[MF_WIDE] = {0}, hiwide[MF_WIDE] = {0}, two[MF_WIDE] = {2};
    unsigned split_stack[256]; int split_depth=0, fusion_count=0;
    bool all_fusions_closed=true;
    for (int i=0;i<nlimbs;++i) { a[i]=start[i]; nwide[i]=nval[i]; hiwide[i]=hi[i]; }
    unsigned long long add = lane;
    for (int i=0;i<nlimbs && add;++i) {
        unsigned long long v=(unsigned long long)a[i]+(unsigned)add;
        a[i]=(unsigned)v; add=(add>>32)+(v>>32);
    }
    bool square=false, root_exact=false, have=false, fixed=false, emitted=false, delta_nonnegative=false;
    for (int oi=0;oi<nops;++oi) {
        unsigned op=ops[oi];
        atomicAdd(ticks, 1ULL);
        if (op==0) { for(int i=0;i<2*MF_LIMBS;++i){delta[i]=0;p[i]=0;q[i]=0;}
            square=false;root_exact=false;have=false;fixed=false;emitted=false;
            split_depth=0;fusion_count=0;all_fusions_closed=true; }
        else if (op==6 || op==12) {
            if (split_depth>=256) all_fusions_closed=false;
            else split_stack[split_depth++]=op==6 ? 2u : 3u;
        }
        else if (op==7 || op==13) {
            unsigned arity=op==7 ? 2u : 3u;
            if (split_depth==0 || split_stack[split_depth-1]!=arity) {
                all_fusions_closed=false;
            } else {
                --split_depth;
                ++fusion_count;
                mf_mul(p,q,product,nlimbs);
                if (!have || mf_cmp(p,two,2*nlimbs)<0
                    || mf_cmp(q,hiwide,2*nlimbs)>0
                    || mf_cmp(product,nwide,2*nlimbs)!=0)
                    all_fusions_closed=false;
            }
        }
        else if (op==2) mf_add_small(a,nlimbs,1u);
        else if (op==5) {
            for(int i=0;i<nlimbs;++i) delta_a[i]=a[i];
            mf_mul(a,a,sq,nlimbs);
            delta_nonnegative=mf_cmp(sq,nwide,2*nlimbs)>=0;
            if(delta_nonnegative){ for(int i=0;i<2*nlimbs;++i) delta[i]=sq[i]; mf_sub(delta,nwide,2*nlimbs); }
            else { for(int i=0;i<2*nlimbs;++i) delta[i]=0; }
        }
        else if (op==3) root_exact=delta_nonnegative && mf_isqrt(delta,nlimbs,root,rem);
        else if (op==9) square=root_exact;
        else if (op==4 && square && !have) {
            for(int i=0;i<2*nlimbs;++i) { p[i]=delta_a[i]; q[i]=delta_a[i]; }
            if(mf_sub(p,root,2*nlimbs)==0) {
                unsigned carry=0; for(int i=0;i<2*nlimbs;++i){unsigned long long v=(unsigned long long)q[i]+root[i]+carry;q[i]=(unsigned)v;carry=v>>32;}
                have=true;
            }
        }
        else if (op==8 && have) {
            bool inrange=mf_cmp(p,two,2*nlimbs)>=0 && mf_cmp(q,hiwide,2*nlimbs)<=0;
            mf_mul(p,q,product,nlimbs);
            if(!inrange || mf_cmp(product,nwide,2*nlimbs)!=0) have=false;
        }
        else if (op==11) fixed=have && split_depth==0 && fusion_count>0 && all_fusions_closed;
        else if (op==1 && fixed) {
            mf_mul(p,q,product,nlimbs);
            emitted=mf_cmp(p,two,2*nlimbs)>=0
                 && mf_cmp(q,hiwide,2*nlimbs)<=0
                 && mf_cmp(product,nwide,2*nlimbs)==0;
        }
    }
    if (emitted) {
        atomicMin(winner,lane);
        for(int i=0;i<nlimbs;++i){out_p[lane*nlimbs+i]=p[i];out_q[lane*nlimbs+i]=q[i];}
    }
}
