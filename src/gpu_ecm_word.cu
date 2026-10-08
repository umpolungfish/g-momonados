// Every operand and intermediate is an IMASM Tape of EVALT/EVALF arms.
// W is the source cell count plus two carry cells, supplied by the host.
#define T 8
#define F 9
typedef unsigned char arm;
// Each warp owns one curve. A lane holds one parity arm from each 32-cell stripe.
#define S ((W+31)/32)
struct Word { arm v[S]; };
struct Point { Word x, z; };
__device__ __forceinline__ int lane() { return threadIdx.x & 31; }
__device__ __forceinline__ bool stopped(const int *found) {
    int value=lane()==0?*found:0;
    return __shfl_sync(0xffffffff,value,0)!=0;
}
__device__ __forceinline__ arm digit(const Word &a,int i) {
    return (arm)__shfl_sync(0xffffffff,(int)a.v[i/32],i%32);
}
__device__ __forceinline__ void clear(Word &a) {
    #pragma unroll
    for(int k=0;k<S;k++) a.v[k]=T;
}
__device__ __forceinline__ void copy(const Word &a, Word &b) { b=a; }
__device__ __forceinline__ void small(unsigned long long n, Word &a) {
    #pragma unroll
    for(int k=0;k<S;k++) { int i=32*k+lane(); a.v[k]=(i<64 && ((n>>(i&63))&1))?F:T; }
}
__device__ __forceinline__ bool zero(const Word &a) {
    bool any=false;
    #pragma unroll
    for(int k=0;k<S;k++) any|=a.v[k]==F;
    return !__any_sync(0xffffffff,any);
}
__device__ __forceinline__ bool one(const Word &a) {
    bool bad=false;
    #pragma unroll
    for(int k=0;k<S;k++) bad|=a.v[k]!=((k==0 && lane()==0)?F:T);
    return !__any_sync(0xffffffff,bad);
}
__device__ __forceinline__ int compare(const Word &a,const Word &b) {
    #pragma unroll
    for(int k=S-1;k>=0;k--) {
        unsigned mask=__ballot_sync(0xffffffff,a.v[k]!=b.v[k]);
        if(mask) { int highest=31-__clz(mask); return __shfl_sync(0xffffffff,(int)a.v[k],highest)==F?1:-1; }
    }
    return 0;
}
// Ballots are lane-control masks; numerical cells remain parity arms.
// The nearest non-propagating lane determines this lane's carry/borrow.
__device__ __forceinline__ bool prefix(bool generate,bool propagate,bool &carry) {
    unsigned generators=__ballot_sync(0xffffffff,generate);
    unsigned blockers=~__ballot_sync(0xffffffff,propagate);
    unsigned before=blockers&((1u<<lane())-1u);
    bool incoming=before?((generators>>(31-__clz(before)))&1)!=0:carry;
    carry=blockers?((generators>>(31-__clz(blockers)))&1)!=0:carry;
    return incoming;
}
__device__ __forceinline__ void add(const Word &a,const Word &b,Word &out) {
    bool carry=false;
    #pragma unroll
    for(int k=0;k<S;k++) {
        bool x=a.v[k]==F,y=b.v[k]==F;
        bool previous=prefix(x&&y,x^y,carry);
        out.v[k]=(x^y^previous)?F:T;
    }
}
__device__ __forceinline__ void subtract(const Word &a,const Word &b,Word &out) {
    bool borrow=false;
    #pragma unroll
    for(int k=0;k<S;k++) {
        bool x=a.v[k]==F,y=b.v[k]==F;
        bool previous=prefix(!x&&y,!(x^y),borrow);
        out.v[k]=(x^y^previous)?F:T;
    }
}
__device__ __forceinline__ void half(Word &a) {
    #pragma unroll
    for(int k=0;k<S;k++) {
        arm next=(arm)__shfl_down_sync(0xffffffff,(int)a.v[k],1);
        arm edge=T;
        if(k+1<S) edge=(arm)__shfl_sync(0xffffffff,(int)a.v[k+1],0);
        a.v[k]=lane()==31?edge:next;
    }
}
__device__ __forceinline__ void addmod(const Word &a,const Word &b,const Word &n,Word &out) {
    add(a,b,out); if(compare(out,n)>=0) subtract(out,n,out);
}
__device__ __forceinline__ void submod(const Word &a,const Word &b,const Word &n,Word &out) {
    if(compare(a,b)>=0) subtract(a,b,out);
    else { Word d; subtract(b,a,d); subtract(n,d,out); }
}
__device__ __noinline__ void regular_mulmod(const Word &a,const Word &b,const Word &n,Word &out) {
    Word acc; clear(acc);
    int top=W-1; while(top>0 && digit(b,top)==T) top--;
    for(int i=top;i>=0;i--) {
        addmod(acc,acc,n,acc); if(digit(b,i)==F) addmod(acc,a,n,acc);
    }
    copy(acc,out);
}
// Montgomery multiplication with radix 2^(W-2), staying on parity arms.
// Curve coordinates are converted once, then retained in this residue domain.
__device__ __noinline__ void mulmod(const Word &a,const Word &b,const Word &n,Word &out) {
    Word acc; clear(acc);
    for(int i=0;i<W-2;i++) {
        if(digit(b,i)==F) add(acc,a,acc);
        if(digit(acc,0)==F) add(acc,n,acc);
        half(acc);
    }
    if(compare(acc,n)>=0) subtract(acc,n,acc);
    copy(acc,out);
}
// n is odd, so binary gcd needs no common power-of-two accumulator.
__device__ __noinline__ void gcd(const Word &a,const Word &n,Word &out) {
    Word x,y; copy(a,x); copy(n,y);
    while(!zero(x)) {
        while(digit(x,0)==T) half(x);
        if(compare(x,y)<0) { Word swap; copy(x,swap); copy(y,x); copy(swap,y); }
        subtract(x,y,x);
    }
    copy(y,out);
}
__device__ __noinline__ bool inverse(const Word &a,const Word &n,Word &out) {
    Word u,v,x,y; copy(a,u); copy(n,v); small(1,x); clear(y);
    while(!one(u) && !one(v)) {
        if(zero(u)||zero(v)) return false;
        while(digit(u,0)==T) { half(u); if(digit(x,0)==F) add(x,n,x); half(x); }
        while(digit(v,0)==T) { half(v); if(digit(y,0)==F) add(y,n,y); half(y); }
        if(compare(u,v)>=0) { subtract(u,v,u); submod(x,y,n,x); }
        else { subtract(v,u,v); submod(y,x,n,y); }
    }
    if(one(u)) copy(x,out); else copy(y,out); return true;
}
__device__ __noinline__ bool proper(const Word &g,const Word &n) { return !zero(g)&&!one(g)&&compare(g,n)<0; }
__device__ __forceinline__ void publish(const Word &factor,arm *out,int *found) {
    int winner=0;
    if(lane()==0) winner=atomicCAS(found,0,1)==0;
    winner=__shfl_sync(0xffffffff,winner,0);
    if(winner) for(int k=0;k<S;k++) { int i=k*32+lane(); if(i<W) out[i]=factor.v[k]; }
}
__device__ __noinline__ void twice(const Point &p,const Word &a24,const Word &n,Point &out) {
    Word plus,minus,aa,bb,e,t;
    addmod(p.x,p.z,n,plus); submod(p.x,p.z,n,minus);
    mulmod(plus,plus,n,aa); mulmod(minus,minus,n,bb); submod(aa,bb,n,e);
    mulmod(aa,bb,n,out.x); mulmod(a24,e,n,t); addmod(bb,t,n,t); mulmod(e,t,n,out.z);
}
__device__ __noinline__ void difference_add(const Point &p,const Point &q,const Point &d,const Word &n,Point &out) {
    Word a,b,c,e,da,cb,sum,dif,t;
    submod(p.x,p.z,n,a); addmod(q.x,q.z,n,b); mulmod(a,b,n,da);
    addmod(p.x,p.z,n,c); submod(q.x,q.z,n,e); mulmod(c,e,n,cb);
    addmod(da,cb,n,sum); submod(da,cb,n,dif);
    mulmod(sum,sum,n,t); mulmod(d.z,t,n,out.x);
    mulmod(dif,dif,n,t); mulmod(d.x,t,n,out.z);
}
__device__ __noinline__ void ladder(const Point &p,const arm *scalar,int count,const Word &a24,const Word &n,Point &out) {
    if(count<=1) { out=p; return; }
    // Start from P and 2P, avoiding the redundant infinity step.
    Point r0=p,r1,next0,next1; twice(p,a24,n,r1);
    for(int i=count-2;i>=0;i--) {
        if(scalar[i]==T) { difference_add(r0,r1,p,n,next1); twice(r0,a24,n,next0); }
        else { difference_add(r0,r1,p,n,next0); twice(r1,a24,n,next1); }
        r0=next0; r1=next1;
    }
    out=r0;
}
__device__ __noinline__ void ladder_small(const Point &p,unsigned long long scalar,const Word &a24,const Word &n,Point &out) {
    arm digits[64]; int count=0;
    do { digits[count++]=(scalar&1)?F:T; scalar>>=1; } while(scalar);
    ladder(p,digits,count,a24,n,out);
}
extern "C" __global__ void ecm_word(const arm *input,const arm *mont_unit,const arm *exponent,const int *offsets,const int *repetitions,int step_count,
    const unsigned char *prime_flags,unsigned long long b1,unsigned long long b2,
    unsigned long long sigma_start,int curves,arm *out,int *found) {
    int index=(blockIdx.x*blockDim.x+threadIdx.x)/32; if(index>=curves || stopped(found)) return;
    Word unit,n,s,u,v,u2,u3,v3,t,diff,numerator,denominator,a24,g,inv,c;
    for(int k=0;k<S;k++) { int i=k*32+lane(); n.v[k]=i<W?input[i]:T; unit.v[k]=i<W?mont_unit[i]:T; }
    small(sigma_start+index,s); regular_mulmod(s,s,n,t); small(5,c); submod(t,c,n,u);
    small(4,c); regular_mulmod(c,s,n,v); regular_mulmod(u,u,n,u2); regular_mulmod(u2,u,n,u3);
    regular_mulmod(v,v,n,t); regular_mulmod(t,v,n,v3); submod(v,u,n,diff);
    regular_mulmod(diff,diff,n,t); regular_mulmod(t,diff,n,numerator);
    small(3,c); regular_mulmod(c,u,n,t); addmod(t,v,n,t); regular_mulmod(numerator,t,n,numerator);
    small(16,c); regular_mulmod(c,u3,n,t); regular_mulmod(t,v,n,denominator);
    gcd(denominator,n,g); if(proper(g,n)) { publish(g,out,found); return; }
    if(!one(g)||!inverse(denominator,n,inv)) return;
    regular_mulmod(numerator,inv,n,a24);
    // A^2-4 is a unit exactly when A24 and A24-1 are units (N is odd).
    // A non-unit yields a factor; a globally singular curve is discarded.
    gcd(a24,n,g); if(proper(g,n)) { publish(g,out,found); return; }
    if(!one(g)) return;
    small(1,c); submod(a24,c,n,t); gcd(t,n,g);
    if(proper(g,n)) { publish(g,out,found); return; }
    if(!one(g)) return;
    Point p,next; regular_mulmod(u3,unit,n,p.x); regular_mulmod(v3,unit,n,p.z); regular_mulmod(a24,unit,n,a24);
    for(int step=0;step<step_count;step++) {
        if(stopped(found)) return;
        int count=offsets[step+1]-offsets[step];
        if(step==0) {
            // The first scheduled scalar is a power of two.
            for(int bit=1;bit<count;bit++) { twice(p,a24,n,next); p=next; }
        } else if(step==1) {
            // 3P = 2P + P, with known difference P: eleven multiplications.
            for(int repeat=0;repeat<repetitions[step];repeat++) {
                Point doubled; twice(p,a24,n,doubled);
                difference_add(doubled,p,p,n,next); p=next;
            }
        } else { ladder(p,exponent+offsets[step],count,a24,n,next); p=next; }
        gcd(p.z,n,g); if(proper(g,n)) { publish(g,out,found); return; }
        if(zero(p.z)) return;
    }
    if(zero(p.z)||stopped(found)||b2<=b1) return;
    Point base=p,two_base,previous,current;
    twice(base,a24,n,two_base);
    unsigned long long first=(b1+1)|1ULL;
    ladder_small(base,first-2,a24,n,previous); ladder_small(base,first,a24,n,current);
    for(unsigned long long k=first;k<=b2;k+=2) {
        if(stopped(found)) return;
        if(prime_flags[k]==F) {
            gcd(current.z,n,g); if(proper(g,n)) { publish(g,out,found); return; }
            if(zero(current.z)) return;
        }
        difference_add(current,two_base,previous,n,next); previous=current; current=next;
    }
}
extern "C" __global__ void word_arithmetic(const arm *input,const arm *mont_unit,const arm *left,const arm *right,arm *product,arm *inverse_out,arm *gcd_out) {
    if(blockIdx.x || threadIdx.x>=32) return;
    Word unit,n,a,b,p,inv,g; for(int k=0;k<S;k++) { int i=k*32+lane(); n.v[k]=i<W?input[i]:T; unit.v[k]=i<W?mont_unit[i]:T; a.v[k]=i<W?left[i]:T; b.v[k]=i<W?right[i]:T; }
    gcd(a,n,g); regular_mulmod(a,unit,n,a); regular_mulmod(b,unit,n,b); mulmod(a,b,n,p); small(1,b); mulmod(p,b,n,p); clear(inv); for(int k=0;k<S;k++) { int i=k*32+lane(); a.v[k]=i<W?left[i]:T; } if(one(g)) inverse(a,n,inv);
    for(int k=0;k<S;k++) { int i=k*32+lane(); if(i<W) { product[i]=p.v[k]; inverse_out[i]=inv.v[k]; gcd_out[i]=g.v[k]; } }
}
