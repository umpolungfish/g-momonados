#define W 138
#define Y_NODE_COUNT 70
#define Y_ROOT 69
#define Y_RESUME 0
#define Y_PREFIX 9
__constant__ int Y_TAG[]={6,3,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,4,8,5,8};
__constant__ int Y_LEFT[]={0,0,0,0,0,0,4,3,0,0,0,9,8,0,12,7,2,0,0,0,0,19,18,0,0,23,22,17,0,0,28,27,16,1,0,0,0,0,36,35,0,0,0,41,40,0,44,39,34,0,0,0,0,51,50,0,0,55,54,49,0,0,60,59,48,33,0,65,0,67};
__constant__ int Y_RIGHT[]={0,0,0,0,0,0,5,6,0,0,0,10,11,0,13,14,15,0,0,0,0,20,21,0,0,24,25,26,0,0,29,30,31,32,0,0,0,0,37,38,0,0,0,42,43,0,45,46,47,0,0,0,0,52,53,0,0,56,57,58,0,0,61,62,63,64,0,66,0,68};
__constant__ int Y_OFFSETS[]={0,69,138,207,276,341,438,535,628,685,740};
__constant__ int Y_TOKENS[]={3,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,4,8,5,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,3,2,3,8,8,3,2,2,8,8,1,8,8,4,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,2,3,8,4,8,3,2,2,8,8,1,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,3,3,2,2,8,8,1,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,3,2,2,8,8,1,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,2,2,8,4,8,1,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,2,1,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,1,4,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,4,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8};
__constant__ unsigned char SOURCE_ARMS[]={9,9,9,9,8,8,8,8,8,8,8,8,8,8,9,9,9,8,9,9,9,8,8,9,8,8,9,9,9,8,8,8,9,8,8,9,9,8,8,9,9,8,9,9,8,8,8,9,8,8,9,9,8,8,9,8,8,9,8,9,9,8,9,9,8,9,9,8,8,9,9,8,9,8,9,9,9,8,8,8,8,8,8,8,8,8,8,8,9,9,9,8,8,9,9,8,8,8,9,9,9,9,8,9,8,9,9,9,9,8,9,9,8,8,9,9,8,9,8,8,9,9,9,9,9,8,9,9,8,8,8,8,8,8,8,8,8,8};

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

__device__ bool proper(const Word &g,const Word &n){return !zero(g)&&!one(g)&&compare(g,n)<0;}
// Uses the existing warp-distributed IMASM parity-arm arithmetic.
// The certified SKI tree and its first reduction witnesses are compiler data.
struct YNode { int tag, left, right; };
__device__ int ynode(YNode *nodes,int &used,int left,int right) {
    if(used>=2048)return -1;
    int i=used++;nodes[i]={8,left,right};return i;
}
__device__ bool y_witness(YNode *nodes,int root,int step) {
    if(step>Y_PREFIX)return true;
    int stack[512],visit[512],top=0,position=Y_OFFSETS[step];
    stack[0]=root;visit[0]=0;
    while(top>=0) {
        YNode n=nodes[stack[top]];
        if(n.tag==8&&visit[top]<2) {
            int child=visit[top]++==0?n.left:n.right;
            if(top==511)return false;
            stack[++top]=child;visit[top]=0;continue;
        }
        if(position>=Y_OFFSETS[step+1]||Y_TOKENS[position++]!=n.tag)return false;
        --top;
    }
    return position==Y_OFFSETS[step+1];
}
// Leftmost outermost S/K/I contraction, using retained tree references.
// OP f PAY becomes RESUME after one payload transition, exactly as ResidentY.
__device__ int y_reduce(YNode *nodes,int &used,int &root) {
    int stack[512],visits[512],top=0;
    stack[0]=root;visits[0]=0;
    while(top>=0) {
        int at=stack[top];
        if(visits[top]) {
            YNode n=nodes[at];
            if(n.tag!=8||visits[top]==3){--top;continue;}
            int child=visits[top]++==1?n.left:n.right;
            if(top==511)return -1;
            stack[++top]=child;visits[top]=0;continue;
        }
        YNode n=nodes[at];int replacement=-1,call=0;
        if(n.tag==6){replacement=at;call=1;}
        else if(n.tag==8) {
            YNode f=nodes[n.left];
            if(f.tag==1)replacement=n.right;
            else if(f.tag==8) {
                YNode g=nodes[f.left];
                if(g.tag==2)replacement=f.right;
                else if(g.tag==4&&nodes[n.right].tag==5){replacement=Y_RESUME;call=1;}
                else if(g.tag==8&&nodes[g.left].tag==3) {
                    int a=ynode(nodes,used,g.right,n.right);
                    int b=ynode(nodes,used,f.right,n.right);
                    if(a<0||b<0)return -1;
                    replacement=ynode(nodes,used,a,b);if(replacement<0)return -1;
                }
            }
        }
        if(replacement>=0) {
            // Rebuild the ancestor path, preserving shared immutable subterms.
            for(int p=top-1;p>=0;--p){YNode ancestor=nodes[stack[p]];
                replacement=visits[p]==2?ynode(nodes,used,replacement,ancestor.right)
                    :ynode(nodes,used,ancestor.left,replacement);
                if(replacement<0)return -1;
            }
            root=replacement;
            return call?2:1;
        }
        visits[top]=1;
    }
    return -1;
}
__device__ void load_word(const arm *p,Word &w) {
    for(int k=0;k<S;k++){int i=32*k+lane();w.v[k]=i<W?p[i]:T;}
}
__device__ void save_word(const Word &w,arm *p) {
    for(int k=0;k<S;k++){int i=32*k+lane();if(i<W)p[i]=w.v[k];}
}
// Bitwise quotient/remainder and exact product on the same parity registers.
__device__ void quotient_word(const Word &n,const Word &d,Word &q,Word &r) {
    clear(q);clear(r);Word one;small(1,one);
    for(int i=W-1;i>=0;--i){
        add(r,r,r);if(digit(n,i)==F)add(r,one,r);
        if(compare(r,d)>=0){subtract(r,d,r);if(lane()==i%32)q.v[i/32]=F;}
    }
}
// The singleton names the complete resident register, never a candidate copy.
__device__ unsigned powerset_unit(unsigned coordinate){return 1u<<coordinate;}
template<class Arrow> __device__ unsigned powerset_bind(unsigned members,Arrow &arrow){
    unsigned result=0;
    for(unsigned i=0;i<1;++i)if(members&(1u<<i))result|=arrow(i);
    return result;
}
__device__ void power_word(const Word &base,const Word &exponent,const Word &n,Word &out){
    Word acc;small(1,acc);
    int top=W-1;while(top>0&&digit(exponent,top)==T)--top;
    for(int i=top;i>=0;--i){
        regular_mulmod(acc,acc,n,acc);
        if(digit(exponent,i)==F)regular_mulmod(acc,base,n,acc);
    }
    out=acc;
}
__device__ unsigned divide_small(const Word &value,unsigned divisor,Word &quotient){
    clear(quotient);unsigned remainder=0;
    for(int bit=W-1;bit>=0;--bit){
        remainder=(remainder<<1)|(digit(value,bit)==F);
        if(remainder>=divisor){remainder-=divisor;if(lane()==bit%32)quotient.v[bit/32]=F;}
    }
    return remainder;
}
// Exact ququart SIC-Zeta round trip on the resident complex coordinate map.
// Lean's Gram is (4 I + J)/5 and its Zeta(-1) dual is (5 I/4 - J/16).
// Keep the common denominator: 16 * dual(Gram(c)) = 80*c.  No floating point,
// terminal rounding, host reconstruction, or factor data enters this map.
__device__ bool sic_zeta_channel(const Word (&coordinate)[4],Word (&recovered)[4],Word &residual){
    Word sum;clear(sum);
    for(int i=0;i<4;++i)add(sum,coordinate[i],sum);
    Word gram[16],gram_sum;clear(gram_sum);
    for(int i=0;i<16;++i){
        Word c;clear(c);if(i<4)c=coordinate[i];
        Word twice,four_times;add(c,c,twice);add(twice,twice,four_times);
        add(four_times,sum,gram[i]);add(gram_sum,gram[i],gram_sum);
    }
    clear(residual);
    for(int i=0;i<16;++i){
        Word five,twenty,dual_numerator,expected,difference;
        add(gram[i],gram[i],five);add(five,five,five); // 4*x
        add(five,gram[i],twenty); // 5*x
        add(twenty,twenty,twenty);add(twenty,twenty,twenty); // 20*x
        subtract(twenty,gram_sum,dual_numerator); // 16 * dual(x)
        Word c;clear(c);if(i<4)c=coordinate[i];
        Word two,four,eight,sixteen,thirty_two,sixty_four;
        add(c,c,two);add(two,two,four);add(four,four,eight);add(eight,eight,sixteen);
        add(sixteen,sixteen,thirty_two);add(thirty_two,thirty_two,sixty_four);
        add(sixty_four,sixteen,expected); // 80*c
        subtract(dual_numerator,expected,difference);
        if(!zero(difference)){residual=difference;return false;}
        if(i<4){Word five_times;unsigned remainder=divide_small(dual_numerator,16,five_times);
            if(remainder){residual=difference;return false;}
            remainder=divide_small(five_times,5,recovered[i]);
            if(remainder){residual=difference;return false;}}
    }
    return true;
}
__device__ bool sic_zeta_resident(Word &e,Word &residual_word,
    Word &exponent,Word &seed,Word &power,Word &frame_residual){
    Word real_coordinates[4]={e,exponent,seed,power};
    Word imaginary_coordinates[4];
    imaginary_coordinates[0]=residual_word;
    for(int i=1;i<4;++i)clear(imaginary_coordinates[i]);
    clear(frame_residual);
    Word channel_residual,real_recovered[4],imaginary_recovered[4];
    if(!sic_zeta_channel(real_coordinates,real_recovered,channel_residual)){frame_residual=channel_residual;return false;}
    if(!sic_zeta_channel(imaginary_coordinates,imaginary_recovered,channel_residual)){frame_residual=channel_residual;return false;}
    // Re-entry consumes the reconstructed coordinates, not the pre-frame
    // candidates. The dual's exact denominator is 16, then Gram's is 5.
    e=real_recovered[0];exponent=real_recovered[1];seed=real_recovered[2];power=real_recovered[3];
    residual_word=imaginary_recovered[0];
    return zero(frame_residual);
}
struct IdempotentArrow {
    const Word &n;Word &e,&residual,&exponent,&seed,&power;
    arm *state,*paired;int &status;unsigned &projecting;
    unsigned long long &restarts,&singularities,&sic_roundtrips;
    Word &frame_residual;
    __device__ unsigned operator()(unsigned coordinate){
        if(coordinate!=0){status=5;return 0;}
        // SIC-Zeta acts on the same resident register before each bound Y
        // transition. Its exact signed reconstruction remainder must close.
        if(!sic_zeta_resident(e,residual,exponent,seed,power,frame_residual)){
            status=7;return 0;
        }
        ++sic_roundtrips;
        Word unit,complement,d,q,r,tmp,product;small(1,unit);
        if(compare(e,n)>=0){status=2;return 0;}
        if(zero(e)||one(e)){
            // Retain the trivial fixed-point observation and re-enter the
            // same source register with an internally advanced starting state.
            save_word(e,state+7*W);save_word(residual,state+8*W);++restarts;
            add(seed,unit,seed);if(compare(seed,n)>=0){status=2;return 0;}
            e=seed;small(2,exponent);subtract(e,unit,tmp);
            regular_mulmod(e,tmp,n,residual);
            addmod(e,e,n,tmp);submod(tmp,unit,n,power);projecting=0;
            return powerset_unit(coordinate);
        }
        if(!zero(residual)){
            // Transport the resident candidate through its involution
            // coordinate s=2e-1. Each Y call applies s -> s^k modulo N;
            // k is itself retained as a parity word, starting at two.
            // No modular period, root, factor, or factor width is supplied.
            Word powered,inverse_two,squared,projection,inverse_probe;
            if(projecting){
                // A source-produced nonunit retains its zero component under
                // powers. Re-entry acts on that same candidate until its
                // remaining component reaches one and e*e=e closes.
                power_word(e,exponent,n,e);
                regular_mulmod(e,e,n,tmp);submod(tmp,e,n,residual);
                add(exponent,unit,exponent);
                return powerset_unit(coordinate);
            }
            power_word(power,exponent,n,powered);power=powered;
            submod(powered,unit,n,projection);
            if(!zero(projection)&&!inverse(projection,n,inverse_probe)){
                // Observe singularity without extracting or supplying a
                // divisor. The candidate remains the source-produced word
                // s-1; no factor is used to manufacture an idempotent.
                e=projection;projecting=1;++singularities;
                save_word(projection,state+10*W);
                regular_mulmod(e,e,n,tmp);submod(tmp,e,n,residual);
                add(exponent,unit,exponent);
                return powerset_unit(coordinate);
            }
            regular_mulmod(powered,powered,n,squared);
            if(compare(squared,powered)==0&&!zero(powered)&&!one(powered)){
                // A nonunit power can itself reach a nontrivial idempotent.
                e=powered;clear(residual);
            }else{
                add(n,unit,inverse_two);half(inverse_two);
                addmod(powered,unit,n,tmp);regular_mulmod(tmp,inverse_two,n,e);
                regular_mulmod(e,e,n,tmp);submod(tmp,e,n,residual);
            }
            add(exponent,unit,exponent);
            return powerset_unit(coordinate);
        }
        regular_mulmod(e,e,n,tmp);
        if(compare(tmp,e)!=0){status=4;return powerset_unit(coordinate);}
        add(n,unit,complement);subtract(complement,e,complement);
        regular_mulmod(e,complement,n,tmp);
        if(!zero(tmp)){status=4;return powerset_unit(coordinate);}
        addmod(e,complement,n,tmp);
        if(!one(tmp)){status=4;return powerset_unit(coordinate);}
        regular_mulmod(complement,complement,n,tmp);
        if(compare(tmp,complement)!=0){status=4;return powerset_unit(coordinate);}
        gcd(e,n,d);if(!proper(d,n)){status=4;return powerset_unit(coordinate);}
        quotient_word(n,d,q,r);if(!zero(r)){status=4;return powerset_unit(coordinate);}
        for(int k=0;k<S;++k){int i=32*k+lane();if(i<W){paired[2*i]=d.v[k];paired[2*i+1]=q.v[k];}}
        __syncwarp();
        Word dback,qback;clear(dback);clear(qback);
        for(int k=0;k<S;++k){int i=32*k+lane();if(i<W){dback.v[k]=paired[2*i];qback.v[k]=paired[2*i+1];}}
        if(compare(d,dback)||compare(q,qback)){status=4;return powerset_unit(coordinate);}
        for(int i=0;i<W;++i)if(digit(dback,i)!=paired[2*i]||digit(qback,i)!=paired[2*i+1]){
            status=4;return powerset_unit(coordinate);
        }
        clear(product);
        // Positive prefixes of d*q are bounded by N, so the exact
        // multiplication fits W=source-bits+2 without truncation.
        int top=W-1;while(top>0&&digit(q,top)==T)--top;
        for(int i=top;i>=0;--i){add(product,product,product);if(digit(qback,i)==F)add(product,dback,product);}
        if(compare(product,n)!=0){status=4;return powerset_unit(coordinate);}
        save_word(complement,state+2*W);save_word(d,state+3*W);save_word(q,state+4*W);
        status=1;return powerset_unit(coordinate);
    }
};
extern "C" __global__ void idempotent_membrane(arm *state,
    unsigned long long *receipt,unsigned steps,int initialize) {
    if(blockIdx.x||threadIdx.x>=32)return;
    __shared__ YNode nodes[2048];
    __shared__ int used,root,action;
    __shared__ arm paired[2*W];
    // Persist the actual Y residual tree across bounded device launches.
    // Its compact resume node is a certified ResidentY operation boundary.
    if(initialize){
        if(lane()==0){for(int i=0;i<Y_NODE_COUNT;++i)nodes[i]={Y_TAG[i],Y_LEFT[i],Y_RIGHT[i]};
            used=Y_NODE_COUNT;root=Y_ROOT;}
    }else if(lane()==0){nodes[Y_RESUME]={6,0,0};root=Y_RESUME;used=Y_NODE_COUNT;}
    Word n,e,residual,exponent,seed,power;
    load_word(SOURCE_ARMS,n);
    if(initialize){small(2,e);small(2,residual);small(2,exponent);small(2,seed);small(3,power);}
    else{load_word(state,e);load_word(state+W,residual);load_word(state+5*W,exponent);load_word(state+6*W,seed);load_word(state+9*W,power);}
    unsigned long long contractions=receipt[0],calls=receipt[1],binds=receipt[2];
    int status=(int)receipt[3];
    unsigned held=initialize?powerset_unit(0):(unsigned)receipt[4];
    unsigned long long restarts=receipt[5];
    unsigned projecting=initialize?0:(unsigned)receipt[6];
    unsigned long long singularities=receipt[7],sic_roundtrips=receipt[8];
    unsigned long long reentry_residue=receipt[9];
    Word frame_residual;clear(frame_residual);
    Word reentry_source,reentry_return;clear(reentry_source);clear(reentry_return);
    if(!initialize){load_word(state+11*W,frame_residual);load_word(state+12*W,reentry_source);load_word(state+13*W,reentry_return);}
    for(unsigned step=0;step<steps&&!status;){
        if(lane()==0){
            action=(!calls&&!y_witness(nodes,root,(int)contractions))?-1:y_reduce(nodes,used,root);
        }
        __syncwarp();
        if(action<0){status=3;break;}
        ++contractions;if(action!=2)continue;
        // unit(payload-coordinate) >>= transition. The coordinate still names
        // this source-bound resident warp, including its residual and evidence.
        ++calls;++step;
        if(held!=powerset_unit(0)){status=5;break;}
        if(calls==1)reentry_source=e;
        // Bind invokes the transformation on the same resident register.
        // Its empty successor differs from the identity singleton.
        IdempotentArrow arrow{n,e,residual,exponent,seed,power,state,paired,status,projecting,restarts,singularities,sic_roundtrips,frame_residual};
        held=powerset_bind(held,arrow);++binds;
        if(held){
            // Keep both presentations as resident history: the original
            // source occurrence and this returned occurrence. Their union
            // records the re-entry residue as Belnap's both-state (B=11).
            reentry_return=e;reentry_residue|=3;
        }
    }
    if(!status){
        Word square,checked;regular_mulmod(e,e,n,square);submod(square,e,n,checked);
        if(compare(checked,residual)!=0)status=6;
    }
    save_word(e,state);save_word(residual,state+W);
    save_word(exponent,state+5*W);save_word(seed,state+6*W);
    save_word(power,state+9*W);
    save_word(frame_residual,state+11*W);
    save_word(reentry_source,state+12*W);save_word(reentry_return,state+13*W);
    if(lane()==0){receipt[0]=contractions;receipt[1]=calls;receipt[2]=binds;receipt[3]=status;receipt[4]=held;receipt[5]=restarts;receipt[6]=projecting;receipt[7]=singularities;receipt[8]=sic_roundtrips;receipt[9]=reentry_residue;}
}
