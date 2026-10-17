#define W 302
#define Y_NODE_COUNT 70
#define Y_ROOT 69
#define Y_RESUME 0
#define Y_PREFIX 9
__constant__ int Y_TAG[]={6,3,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,4,8,5,8};
__constant__ int Y_LEFT[]={0,0,0,0,0,0,4,3,0,0,0,9,8,0,12,7,2,0,0,0,0,19,18,0,0,23,22,17,0,0,28,27,16,1,0,0,0,0,36,35,0,0,0,41,40,0,44,39,34,0,0,0,0,51,50,0,0,55,54,49,0,0,60,59,48,33,0,65,0,67};
__constant__ int Y_RIGHT[]={0,0,0,0,0,0,5,6,0,0,0,10,11,0,13,14,15,0,0,0,0,20,21,0,0,24,25,26,0,0,29,30,31,32,0,0,0,0,37,38,0,0,0,42,43,0,45,46,47,0,0,0,0,52,53,0,0,56,57,58,0,0,61,62,63,64,0,66,0,68};
__constant__ int Y_OFFSETS[]={0,69,138,207,276,341,438,535,628,685,740};
__constant__ int Y_TOKENS[]={3,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,8,4,8,5,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,3,2,3,8,8,3,2,2,8,8,1,8,8,4,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,2,3,8,4,8,3,2,2,8,8,1,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,3,3,2,2,8,8,1,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,5,8,3,2,2,8,8,1,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,2,2,8,4,8,1,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,2,1,4,8,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,1,4,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8,4,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,4,8,3,3,2,3,8,8,3,2,2,8,8,1,8,8,8,3,3,2,3,8,8,2,1,8,8,8,2,1,8,8,8,4,8,8,8,5,8};
__constant__ unsigned char SOURCE_ARMS[]={9,9,9,8,9,8,9,9,8,8,8,9,8,9,9,9,8,9,9,9,8,8,8,9,9,8,9,9,9,9,9,8,8,8,8,9,8,8,8,8,8,8,9,9,8,8,9,8,9,8,9,8,8,8,9,8,8,8,8,9,8,8,9,9,8,8,9,8,9,9,9,8,8,8,8,9,9,8,8,8,9,8,9,8,8,8,9,8,9,9,8,9,8,8,8,9,9,9,9,8,9,8,9,9,8,8,9,8,9,9,9,8,9,8,9,9,9,8,8,8,8,9,8,8,8,8,8,8,8,8,9,9,9,8,9,9,9,8,8,8,9,8,9,9,9,9,8,8,8,9,9,9,9,8,8,9,9,8,9,9,8,8,8,9,9,8,8,8,8,9,9,9,8,8,8,8,9,9,8,8,8,8,9,8,9,9,9,9,9,9,8,8,8,9,8,8,8,8,8,8,8,8,8,9,9,8,8,8,8,9,8,8,9,9,9,8,9,8,9,9,9,8,9,9,9,8,9,8,8,8,8,8,9,9,9,8,8,9,9,9,8,9,8,8,9,9,9,8,8,8,9,8,9,8,8,9,8,8,9,8,8,8,8,9,8,9,8,8,8,8,8,9,9,8,8,9,9,8,9,9,8,8,9,8,8,9,8,9,8,8,9,9,9,9,8,9,8,8,8,9,8,8};

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
__device__ unsigned powerset_bind(unsigned members,const unsigned *successors){
    unsigned result=0;
    for(unsigned i=0;i<1;++i)if(members&(1u<<i))result|=successors[i];
    return result;
}
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
    Word n,e,residual,unit,complement,d,q,r,tmp,product;
    load_word(SOURCE_ARMS,n);small(1,unit);
    if(initialize){small(2,e);small(2,residual);}
    else{load_word(state,e);load_word(state+W,residual);}
    unsigned long long contractions=receipt[0],calls=receipt[1],binds=receipt[2];
    int status=(int)receipt[3];
    unsigned held=initialize?powerset_unit(0):(unsigned)receipt[4];
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
        // A deterministic successor returns this same resident object through
        // union; an empty successor is distinct and terminates the family.
        unsigned successors[1]={powerset_unit(0)};
        held=powerset_bind(held,successors);++binds;
        if(compare(e,n)>=0){status=2;break;}
        if(zero(residual)) {
            regular_mulmod(e,e,n,tmp);
            if(compare(tmp,e)!=0){status=4;break;}
            add(n,unit,complement);subtract(complement,e,complement);
            regular_mulmod(e,complement,n,tmp);
            if(!zero(tmp)){status=4;break;}
            addmod(e,complement,n,tmp);
            if(!one(tmp)){status=4;break;}
            regular_mulmod(complement,complement,n,tmp);
            if(compare(tmp,complement)!=0){status=4;break;}
            gcd(e,n,d);if(!proper(d,n)){status=4;break;}
            quotient_word(n,d,q,r);if(!zero(r)){status=4;break;}
            for(int k=0;k<S;++k){int i=32*k+lane();if(i<W){paired[2*i]=d.v[k];paired[2*i+1]=q.v[k];}}
            __syncwarp();
            Word dback,qback;clear(dback);clear(qback);
            for(int k=0;k<S;++k){int i=32*k+lane();if(i<W){dback.v[k]=paired[2*i];qback.v[k]=paired[2*i+1];}}
            if(compare(d,dback)||compare(q,qback)){status=4;break;}
            for(int i=0;i<W;++i)if(digit(dback,i)!=paired[2*i]||digit(qback,i)!=paired[2*i+1]){status=4;break;}
            if(status)break;
            clear(product);
            // Positive prefixes of d*q are bounded by N, so the exact
            // multiplication fits W=source-bits+2 without truncation.
            int top=W-1;while(top>0&&digit(q,top)==T)--top;
            for(int i=top;i>=0;--i){add(product,product,product);if(digit(qback,i)==F)add(product,dback,product);}
            if(compare(product,n)!=0){status=4;break;}
            save_word(complement,state+2*W);save_word(d,state+3*W);save_word(q,state+4*W);
            status=1;break;
        }
        // Exact source-bound recurrence (e+1)e = e(e-1)+2e modulo N.
        addmod(e,e,n,tmp);addmod(residual,tmp,n,residual);add(e,unit,e);
    }
    save_word(e,state);save_word(residual,state+W);
    if(lane()==0){receipt[0]=contractions;receipt[1]=calls;receipt[2]=binds;receipt[3]=status;receipt[4]=held;}
}
