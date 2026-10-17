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
struct IdempotentArrow {
    const Word &n;Word &e,&residual,&exponent,&seed;
    arm *state,*paired;int &status;unsigned long long &restarts;
    __device__ unsigned operator()(unsigned coordinate){
        if(coordinate!=0){status=5;return 0;}
        Word unit,complement,d,q,r,tmp,product;small(1,unit);
        if(compare(e,n)>=0){status=2;return 0;}
        if(zero(e)||one(e)){
            // Retain the trivial fixed-point observation and re-enter the
            // same source register with an internally advanced starting state.
            save_word(e,state+7*W);save_word(residual,state+8*W);++restarts;
            add(seed,unit,seed);if(compare(seed,n)>=0){status=2;return 0;}
            e=seed;small(2,exponent);subtract(e,unit,tmp);
            regular_mulmod(e,tmp,n,residual);
            return powerset_unit(coordinate);
        }
        if(!zero(residual)){
            // Transport the resident candidate through its involution
            // coordinate s=2e-1. Each Y call applies s -> s^k modulo N;
            // k is itself retained as a parity word, starting at two.
            // No modular period, root, factor, or factor width is supplied.
            Word s,powered,inverse_two,squared;
            addmod(e,e,n,tmp);submod(tmp,unit,n,s);
            power_word(s,exponent,n,powered);
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
    Word n,e,residual,exponent,seed;
    load_word(SOURCE_ARMS,n);
    if(initialize){small(2,e);small(2,residual);small(2,exponent);small(2,seed);}
    else{load_word(state,e);load_word(state+W,residual);load_word(state+5*W,exponent);load_word(state+6*W,seed);}
    unsigned long long contractions=receipt[0],calls=receipt[1],binds=receipt[2];
    int status=(int)receipt[3];
    unsigned held=initialize?powerset_unit(0):(unsigned)receipt[4];
    unsigned long long restarts=receipt[5];
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
        // Bind invokes the transformation on the same resident register.
        // Its empty successor differs from the identity singleton.
        IdempotentArrow arrow{n,e,residual,exponent,seed,state,paired,status,restarts};
        held=powerset_bind(held,arrow);++binds;
    }
    if(!status){
        Word square,checked;regular_mulmod(e,e,n,square);submod(square,e,n,checked);
        if(compare(checked,residual)!=0)status=6;
    }
    save_word(e,state);save_word(residual,state+W);
    save_word(exponent,state+5*W);save_word(seed,state+6*W);
    if(lane()==0){receipt[0]=contractions;receipt[1]=calls;receipt[2]=binds;receipt[3]=status;receipt[4]=held;receipt[5]=restarts;}
}
