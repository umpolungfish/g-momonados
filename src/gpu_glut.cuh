// Device interpretation of Vox's exported shared multiplier gates.
__device__ unsigned glut_bit(const unsigned *value,unsigned bit){return (value[bit/32]>>(bit%32))&1u;}
// Sequential banked observation transport for the compiled dissolved carrier.
// Deposits refer to the resident payload, never copies of its mutable masks.
// The device work runs while this enclosing frame is open. Only its verified
// return executes the final fuse/fix/readout segment.
extern "C" __global__ void glut_carrier_transport(const unsigned *word,unsigned length,
 unsigned begin,unsigned end,const unsigned *source,const unsigned *masks,
 unsigned *live,unsigned *bank,unsigned *state,unsigned long long *identity) {
 if(blockIdx.x||threadIdx.x)return;
 unsigned long long source_id=(unsigned long long)source,payload_id=(unsigned long long)masks;
 // A validated serialized held frame rebinds its source/payload handles at
 // import. Its observations, word addresses and bank remain unchanged.
 if(begin==0&&end==0){identity[0]=source_id;identity[1]=payload_id;return;}
 for(unsigned ip=begin;ip<end&&ip<length;++ip){unsigned op=word[ip];
   if(op==0){for(unsigned i=0;i<4*length;++i)live[i]=bank[i]=0;
     for(unsigned i=0;i<10;++i)state[i]=0;identity[0]=source_id;identity[1]=payload_id;}
   if(identity[0]!=source_id||identity[1]!=payload_id){state[3]++;return;}
   if(state[2]&&op!=11&&op!=8){++state[6];continue;}
   ++state[9];
   if(op==6){++state[1];if(state[1]!=1){++state[3];return;}}
   else if(op==7){
     if(state[1]!=1){++state[3];return;}
     for(unsigned i=0;i<4*length;++i)if(bank[i]){
       if(!live[i])++state[4];live[i]=1;state[0]|=1u<<(i%4);}
     state[1]=0;
   }else if(op==0||op==3){
     for(unsigned i=0;i<4*length;++i)if(live[i]){
       ++state[8];if(op==0||!bank[i])++state[3];live[i]=0;}
     state[0]=0;
   }else if(op==2||op==8){if(!state[0])state[0]=1;}
   else if(op==5||op==9||op==10){unsigned lanes=op==5?1:op==9?2:12;
     ++state[5];state[0]|=lanes;
     for(unsigned lane=0;lane<4;++lane)if(lanes&(1u<<lane)){
       unsigned id=4*ip+lane;live[id]=1;if(state[1])bank[id]=1;}
   }else if(op==11)state[2]=1;
   // phase_word treats TANCH as a boundary observation after fixation.
   // IFIX freezes its action; terminal release is recorded at the word end.
 }
 if(end==length&&state[2]&&state[1]==0&&!state[3])state[7]=1;
}

// Geometry labels remain attached to selector cells inside the shared word.
extern "C" __global__ void glut_admission_read(const unsigned *labels,unsigned rows,
 const unsigned *masks,unsigned *flags,unsigned *receipt) {
 if(blockIdx.x||threadIdx.x)return;
 receipt[0]=receipt[1]=0;
 for(unsigned row=0;row<rows;++row){unsigned mask=masks[labels[3*row+2]];
   receipt[0]+=(mask&2)!=0;receipt[1]+=(mask==2);}
 if(!receipt[0])flags[1]=1;
}
// The Quantum component consumes the SAME factor cells as Correlation.
// Execute the canonical reversible gate stream and retain its relative sign.
// Workspace starts at zero and is checked after the ascending reverse rail.
extern "C" __global__ void glut_factor_phase(const unsigned *offsets,
 const unsigned *controls,const unsigned *values,const unsigned *targets,
 unsigned gates,unsigned width,const unsigned *p,unsigned plen,
 const unsigned *q,unsigned qlen,const unsigned *masks,unsigned *workspace,
 unsigned cells,unsigned *receipt) {
 if(blockIdx.x||threadIdx.x)return;
 if(plen>width||qlen>width){receipt[0]=0;receipt[1]=0;receipt[2]=1;receipt[3]=0;return;}
 for(unsigned i=0;i<cells;++i)workspace[i]=0;
 receipt[0]=0;receipt[1]=0;receipt[2]=0;receipt[3]=1;
 for(unsigned i=0;i<plen;++i){unsigned m=masks[p[i]];
   if(m!=1&&m!=2){receipt[3]=0;return;}workspace[i]=(m==2);}
 for(unsigned i=0;i<qlen;++i){unsigned m=masks[q[i]];
   if(m!=1&&m!=2){receipt[3]=0;return;}workspace[width+i]=(m==2);}
 for(unsigned gate=0;gate<gates;++gate){
   bool enabled=true;
   for(unsigned j=offsets[gate];j<offsets[gate+1];++j)
     if(workspace[controls[j]]!=values[j]){enabled=false;break;}
   if(enabled){if(targets[gate]==~0u)receipt[0]^=1;
     else workspace[targets[gate]]^=1;}
   ++receipt[1];
 }
 for(unsigned i=2*width;i<cells;++i)receipt[2]|=workspace[i];
 for(unsigned i=0;i<width;++i){
   unsigned left=i<plen?(masks[p[i]]==2):0;
   unsigned right=i<qlen?(masks[q[i]]==2):0;
   if(workspace[i]!=left||workspace[width+i]!=right)receipt[3]=0;
 }
}

// Verify's running-product checkpoints share the selected factor cells.
// Each entry is the exact carry entering the source position. Reconstructing
// every source bit and the terminal zero carry checks the complete ancestry.
extern "C" __global__ void glut_verify_carries(const unsigned *masks,
 const unsigned *p,unsigned plen,const unsigned *q,unsigned qlen,
 const unsigned *n,unsigned bits,unsigned *carries,unsigned *receipt) {
 if(blockIdx.x||threadIdx.x)return;
 unsigned carry=0;carries[0]=0;receipt[0]=1;receipt[1]=0;
 for(unsigned k=0;k<bits;++k){
   unsigned sum=carry;
   for(unsigned i=0;i<=k;++i){unsigned j=k-i;
     unsigned a=i<plen?masks[p[i]]:1,b=j<qlen?masks[q[j]]:1;
     if((a!=1&&a!=2)||(b!=1&&b!=2)){receipt[0]=0;return;}
     sum+=(a==2&&b==2);
   }
   if((sum&1u)!=((n[k/32]>>(k%32))&1u)){receipt[0]=0;return;}
   carry=sum>>1;carries[k+1]=carry;++receipt[1];
 }
 // Include product diagonals above the source width as well.
 for(unsigned k=bits;k<plen+qlen;++k){unsigned sum=carry;
   for(unsigned i=0;i<=k;++i){unsigned j=k-i;
     if(i<plen&&j<qlen)sum+=(masks[p[i]]==2&&masks[q[j]]==2);}
   if(sum){receipt[0]=0;return;}carry=sum>>1;
 }
 if(carry)receipt[0]=0;
}

extern "C" __global__ void glut_propagate(const unsigned *offsets,
 const unsigned *cells, const unsigned *supports, const unsigned *all,
 unsigned gates, unsigned *masks, unsigned *flags) {
 unsigned g=blockIdx.x*blockDim.x+threadIdx.x;
 if(g>=gates) return;
 unsigned viable=all[g];
 for(unsigned j=offsets[g];j<offsets[g+1];++j) {
   unsigned m=atomicAdd(masks+cells[j],0u);
   viable&=((m&1)?supports[2*j]:0)|((m&2)?supports[2*j+1]:0);
 }
 if(!viable) {atomicExch(flags+1,1u);return;}
 for(unsigned j=offsets[g];j<offsets[g+1];++j) {
   unsigned allowed=((viable&supports[2*j])?1:0)|((viable&supports[2*j+1])?2:0);
   unsigned old=atomicAnd(masks+cells[j],allowed);
   if((old&allowed)!=old) atomicExch(flags,1u);
   if(!(old&allowed)) atomicExch(flags+1,1u);
 }
}

// Masks component: the resident fold's low-diagonal carry rule. Restrict a
// single unresolved corner from N; retain a diagonal with several alternatives.
// This acts directly on Correlation's cell IDs and never selects a candidate.
extern "C" __global__ void glut_mask_carry(unsigned *masks,
 const unsigned *p,unsigned plen,const unsigned *q,unsigned qlen,
 const unsigned *n,unsigned bits,unsigned *flags,unsigned *stats) {
 if(blockIdx.x||threadIdx.x)return;
 unsigned carry=0;
 for(unsigned k=0;k<bits;++k){
   unsigned sum=carry,free=0,corner=0;
   for(unsigned i=0;i<=k;++i){
     unsigned j=k-i,a=i<plen?masks[p[i]]:1,b=j<qlen?masks[q[j]]:1;
     if(!a||!b){flags[1]=1;return;}
     if(a==1||b==1)continue;
     if(a==2&&b==2){++sum;continue;}
     if(a==3){++free;corner=p[i];}
     if(b==3){++free;corner=q[j];}
   }
   if(free>1)return;
   unsigned source=(n[k/32]>>(k%32))&1u;
   if(free){
     unsigned bit=(sum&1u)^source,allowed=bit?2:1;
     unsigned old=atomicAnd(masks+corner,allowed);
     if(!(old&allowed)){flags[1]=1;return;}
     if(old!=allowed){flags[0]=1;++stats[1];}
     sum+=bit;
   }else if((sum&1u)!=source){flags[1]=1;return;}
   carry=sum>>1;++stats[0];
 }
 if(carry)flags[1]=1;
}

// The complete carry envelope propagates the terminal zero carry back to
// every product diagonal, including those beyond the source's highest bit.
extern "C" __global__ void glut_mask_envelope(unsigned *masks,
 const unsigned *p,unsigned plen,const unsigned *q,unsigned qlen,
 const unsigned *n,unsigned bits,unsigned *flags,unsigned *stats) {
 if(blockIdx.x||threadIdx.x||flags[1])return;
 unsigned extent=bits>plen+qlen?bits:plen+qlen;
 unsigned lower[64*MF_LIMBS+1],upper[64*MF_LIMBS+1];
 unsigned lo_sum[64*MF_LIMBS],hi_sum[64*MF_LIMBS];
 lower[0]=upper[0]=0;
 for(unsigned k=0;k<extent;++k){unsigned lo=0,hi=0;
   unsigned start=k>=qlen?k-qlen+1:0,end=plen<k+1?plen:k+1;
   for(unsigned i=start;i<end;++i){unsigned a=masks[p[i]],b=masks[q[k-i]];
     if(!a||!b){flags[1]=1;return;}lo+=(a==2&&b==2);hi+=((a&2)&&(b&2));}
   lo_sum[k]=lo;hi_sum[k]=hi;
   unsigned source=k<bits?glut_bit(n,k):0;
   unsigned low=lower[k]+lo,high=upper[k]+hi;
   if((low&1u)!=source)++low;
   if((high&1u)!=source){if(!high){flags[1]=1;return;}--high;}
   if(low>high){flags[1]=1;return;}
   lower[k+1]=low>>1;upper[k+1]=high>>1;++stats[0];
 }
 if(lower[extent]){flags[1]=1;return;}upper[extent]=0;
 for(int k=(int)extent-1;k>=0;--k){unsigned source=(unsigned)k<bits?glut_bit(n,k):0;
   unsigned low=2*lower[k+1]+source,high=2*upper[k+1]+source;
   if(high<lo_sum[k]){flags[1]=1;return;}
   unsigned incoming_lo=low>hi_sum[k]?low-hi_sum[k]:0,incoming_hi=high-lo_sum[k];
   if(incoming_lo>lower[k])lower[k]=incoming_lo;
   if(incoming_hi<upper[k])upper[k]=incoming_hi;
   if(lower[k]>upper[k]){flags[1]=1;return;}
 }
 for(unsigned k=0;k<extent;++k){unsigned source=k<bits?glut_bit(n,k):0;
   unsigned low=2*lower[k+1]+source,high=2*upper[k+1]+source;
   if(high<lower[k]){flags[1]=1;return;}
   unsigned required_lo=low>upper[k]?low-upper[k]:0,required_hi=high-lower[k];
   if(required_lo>hi_sum[k]||required_hi<lo_sum[k]){flags[1]=1;return;}
   bool force_one=required_lo==hi_sum[k],force_zero=required_hi==lo_sum[k];
   if(!force_one&&!force_zero)continue;
   unsigned start=k>=qlen?k-qlen+1:0,end=plen<k+1?plen:k+1;
   for(unsigned i=start;i<end;++i){unsigned j=k-i,a=masks[p[i]],b=masks[q[j]];
     if(a==1||b==1||(a==2&&b==2))continue;
     unsigned ca=0,cb=0;
     if(force_one){if(a==3)ca=2;if(b==3)cb=2;}
     else if(a==2&&b==3)cb=1;else if(b==2&&a==3)ca=1;
     if(ca){unsigned old=atomicAnd(masks+p[i],ca);if(!(old&ca)){flags[1]=1;return;}
       if(old!=ca){flags[0]=1;++stats[1];}}
     if(cb){unsigned old=atomicAnd(masks+q[j],cb);if(!(old&cb)){flags[1]=1;return;}
       if(old!=cb){flags[0]=1;++stats[1];}}
   }
 }
}

// The existing prepared DQI echelon rows constrain the same resident cells.
extern "C" __global__ void glut_parity(const unsigned *offsets,
 const unsigned *cells,const unsigned *marks,unsigned rows,
 unsigned *masks,unsigned *flags) {
 unsigned row=blockIdx.x*blockDim.x+threadIdx.x;if(row>=rows)return;
 unsigned residual=marks[row],free=0,cell=0;
 for(unsigned j=offsets[row];j<offsets[row+1];++j) {
   unsigned value=atomicAdd(masks+cells[j],0u);
   if(!value){atomicExch(flags+1,1u);return;}
   if(value==3){++free;cell=cells[j];}else residual^=(value==2);
 }
 if(!free&&residual)atomicExch(flags+1,1u);
 if(free==1){unsigned allow=residual?2:1;
   unsigned old=atomicAnd(masks+cell,allow);
   if((old&allow)!=old)atomicExch(flags,1u);
   if(!(old&allow))atomicExch(flags+1,1u);
 }
}

// State: depth, terminal (0 held / 1 closed / 2 empty), decisions, conflicts.
// Two-free-cell DQI rows are phase edges. Weighted union closes their relative
// turns, matching Correlation::parity_holonomy's odd-loop rejection.
extern "C" __global__ void glut_holonomy(const unsigned *offsets,
 const unsigned *cells,const unsigned *marks,unsigned rows,const unsigned *masks,
 unsigned count,unsigned *parents,unsigned *turns,unsigned *flags,unsigned *stats) {
 if(blockIdx.x||threadIdx.x)return;
 for(unsigned i=0;i<count;++i){parents[i]=i;turns[i]=0;}
 for(unsigned row=0;row<rows;++row) {
   unsigned residual=marks[row],free=0,a=0,b=0;
   for(unsigned j=offsets[row];j<offsets[row+1];++j) {
     unsigned value=masks[cells[j]];
     if(value==3){if(!free)a=cells[j];else b=cells[j];++free;}
     else residual^=(value==2);
   }
   if(free!=2)continue;
   ++stats[0];
   unsigned ra=a,rb=b,ta=0,tb=0;
   while(parents[ra]!=ra){ta^=turns[ra];ra=parents[ra];}
   while(parents[rb]!=rb){tb^=turns[rb];rb=parents[rb];}
   if(ra==rb){if((ta^tb)!=residual){flags[1]=1;++stats[1];stats[2]=row;return;}}
   else { // Attach roots in canonical cell order, retaining the relative turn.
     unsigned from=ra>rb?ra:rb,to=ra>rb?rb:ra;
     parents[from]=to;turns[from]=ta^tb^residual;
   }
 }
}

// Exact floor/ceiling quotient for source-bound interval transport. This is
// fixed-width long division, with no candidate enumeration.
__device__ bool glut_quotient(const unsigned *n,const unsigned *d,unsigned limbs,
 unsigned *quotient,bool ceiling) {
 unsigned remainder[MF_LIMBS]={0};bool nonzero=false;
 for(unsigned i=0;i<limbs;++i){quotient[i]=0;nonzero|=(d[i]!=0);}
 if(!nonzero)return false;
 for(int bit=(int)(32*limbs)-1;bit>=0;--bit){
   unsigned carry=(n[bit/32]>>(bit%32))&1u;
   for(unsigned i=0;i<limbs;++i){unsigned next=remainder[i]>>31;
     remainder[i]=(remainder[i]<<1)|carry;carry=next;}
   if(carry||mf_cmp(remainder,d,limbs)>=0){mf_sub(remainder,d,limbs);quotient[bit/32]|=1u<<(bit%32);}
 }
 nonzero=false;for(unsigned i=0;i<limbs;++i)nonzero|=(remainder[i]!=0);
 if(ceiling&&nonzero&&mf_add_small(quotient,limbs,1))return false;
 return true;
}

__device__ void glut_copy(unsigned *to,const unsigned *from,unsigned limbs){
 for(unsigned i=0;i<limbs;++i)to[i]=from[i];
}
__device__ void glut_min(unsigned *to,const unsigned *value,unsigned limbs){
 if(mf_cmp(to,value,limbs)>0)glut_copy(to,value,limbs);
}
__device__ void glut_max(unsigned *to,const unsigned *value,unsigned limbs){
 if(mf_cmp(to,value,limbs)<0)glut_copy(to,value,limbs);
}

// The canonical four tightness states retain holes in each factor's masks.
// This is the same finite-state extrema operation as glut_fold::extrema.
__device__ bool glut_extrema(const unsigned *masks,const unsigned *cells,unsigned width,
 const unsigned *lo,const unsigned *hi,unsigned limbs,unsigned *minimum,unsigned *maximum) {
 if(width>32*limbs||mf_cmp(lo,hi,limbs)>0)return false;
 for(unsigned i=width;i<32*limbs;++i)if(glut_bit(lo,i))return false;
 unsigned char feasible[32*MF_LIMBS+1][4];
 for(unsigned s=0;s<4;++s)feasible[0][s]=1;
 for(unsigned k=1;k<=width;++k){unsigned i=k-1,lb=glut_bit(lo,i),ub=glut_bit(hi,i);
   for(unsigned state=0;state<4;++state){feasible[k][state]=0;
     for(unsigned bit=0;bit<2;++bit){
       if(!(masks[cells[i]]&(1u<<bit)))continue;
       if(((state&1)&&bit<lb)||((state&2)&&bit>ub))continue;
       unsigned next=(((state&1)&&bit==lb)?1:0)|(((state&2)&&bit==ub)?2:0);
       feasible[k][state]|=feasible[k-1][next];
     }
   }
 }
 if(!feasible[width][3])return false;
 for(unsigned pass=0;pass<2;++pass){unsigned *out=pass?maximum:minimum;
   for(unsigned i=0;i<limbs;++i)out[i]=0;
   unsigned state=3;
   for(int i=(int)width-1;i>=0;--i){unsigned lb=glut_bit(lo,i),ub=glut_bit(hi,i);
     for(unsigned first=0;first<2;++first){unsigned bit=pass?1-first:first;
       if(!(masks[cells[i]]&(1u<<bit)))continue;
       if(((state&1)&&bit<lb)||((state&2)&&bit>ub))continue;
       unsigned next=(((state&1)&&bit==lb)?1:0)|(((state&2)&&bit==ub)?2:0);
       if(feasible[i][next]){if(bit)out[i/32]|=1u<<(i%32);state=next;break;}
     }
   }
 }
 return true;
}

__device__ bool glut_constrain_interval(unsigned *masks,const unsigned *cells,unsigned width,
 const unsigned *lo,const unsigned *hi,unsigned limbs,unsigned *flags,unsigned *stats) {
 unsigned minimum[MF_LIMBS],maximum[MF_LIMBS];
 if(!glut_extrema(masks,cells,width,lo,hi,limbs,minimum,maximum))return false;
 for(int i=(int)width-1;i>=0;--i){unsigned bit=glut_bit(minimum,i);
   if(bit!=glut_bit(maximum,i))break;
   unsigned allowed=1u<<bit,old=atomicAnd(masks+cells[i],allowed);
   if(!(old&allowed))return false;
   if(old!=allowed){flags[0]=1;++stats[1];}
 }
 return true;
}

// Masks interval rail returns its intersections to the existing cell masks.
extern "C" __global__ void glut_mask_intervals(unsigned *masks,
 const unsigned *p,unsigned plen,const unsigned *q,unsigned qlen,
 const unsigned *n,const unsigned *root,unsigned limbs,unsigned *flags,unsigned *stats) {
 if(blockIdx.x||threadIdx.x)return;
 unsigned plo[MF_LIMBS]={3},phi[MF_LIMBS],qlo[MF_LIMBS],qhi[MF_LIMBS]={0};
 unsigned pmin[MF_LIMBS],pmax[MF_LIMBS],qmin[MF_LIMBS],qmax[MF_LIMBS],tmp[MF_LIMBS];
 glut_copy(phi,root,limbs);
 if(!glut_quotient(n,root,limbs,qlo,true)){flags[1]=1;return;}
 for(unsigned i=0;i<qlen;++i)if(masks[q[i]]&2)qhi[i/32]|=1u<<(i%32);
 if(!glut_extrema(masks,p,plen,plo,phi,limbs,pmin,pmax)
  ||!glut_extrema(masks,q,qlen,qlo,qhi,limbs,qmin,qmax)){flags[1]=1;return;}
 unsigned low[MF_WIDE],high[MF_WIDE],source[MF_WIDE]={0};
 for(unsigned i=0;i<limbs;++i)source[i]=n[i];
 mf_mul(pmin,qmin,low,limbs);mf_mul(pmax,qmax,high,limbs);
 if(mf_cmp(low,source,2*limbs)>0||mf_cmp(high,source,2*limbs)<0||mf_cmp(pmin,qmax,limbs)>0){flags[1]=1;return;}
 glut_copy(plo,pmin,limbs);glut_quotient(n,qmax,limbs,tmp,true);glut_max(plo,tmp,limbs);
 glut_copy(phi,pmax,limbs);glut_quotient(n,qmin,limbs,tmp,false);glut_min(phi,tmp,limbs);glut_min(phi,qmax,limbs);
 glut_copy(qlo,qmin,limbs);glut_quotient(n,pmax,limbs,tmp,true);glut_max(qlo,tmp,limbs);glut_max(qlo,pmin,limbs);
 glut_copy(qhi,qmax,limbs);glut_quotient(n,pmin,limbs,tmp,false);glut_min(qhi,tmp,limbs);
 ++stats[0];
 if(!glut_constrain_interval(masks,p,plen,plo,phi,limbs,flags,stats)
   ||!glut_constrain_interval(masks,q,qlen,qlo,qhi,limbs,flags,stats))flags[1]=1;
}

// The host GLUT interval rule uses min/max products of the same factor cells.
// Apply its rejection on device, with the source-root bound prepared on CUDA.
extern "C" __global__ void glut_interval(const unsigned *masks,
 const unsigned *p,unsigned plen,const unsigned *q,unsigned qlen,
 const unsigned *n,const unsigned *root,unsigned limbs,unsigned *flags) {
 if(blockIdx.x||threadIdx.x)return;
 unsigned lo_p[MF_LIMBS]={0},hi_p[MF_LIMBS]={0};
 unsigned lo_q[MF_LIMBS]={0},hi_q[MF_LIMBS]={0};
 for(unsigned i=0;i<plen;++i){unsigned v=masks[p[i]];
   if(v==2)lo_p[i/32]|=1u<<(i%32);if(v&2)hi_p[i/32]|=1u<<(i%32);}
 for(unsigned i=0;i<qlen;++i){unsigned v=masks[q[i]];
   if(v==2)lo_q[i/32]|=1u<<(i%32);if(v&2)hi_q[i/32]|=1u<<(i%32);}
 if(mf_cmp(hi_p,root,limbs)>0)for(unsigned i=0;i<limbs;++i)hi_p[i]=root[i];
 if(mf_cmp(lo_p,hi_p,limbs)>0||mf_cmp(lo_p,hi_q,limbs)>0){atomicExch(flags+1,1u);return;}
 unsigned low[MF_WIDE],high[MF_WIDE],source[MF_WIDE]={0};
 for(unsigned i=0;i<limbs;++i)source[i]=n[i];
 mf_mul(lo_p,lo_q,low,limbs);mf_mul(hi_p,hi_q,high,limbs);
 if(mf_cmp(low,source,2*limbs)>0||mf_cmp(high,source,2*limbs)<0)atomicExch(flags+1,1u);
}

// Each decision banks the entire correlated word, including internal carries.
extern "C" __global__ void glut_control(unsigned *masks,unsigned count,
 const unsigned *p,unsigned plen,const unsigned *q,unsigned qlen,
 unsigned *banks,unsigned *choices,unsigned *stack,unsigned *state,
 const unsigned *flags,const unsigned *n,unsigned limbs,unsigned *outp,unsigned *outq) {
 if(blockIdx.x||threadIdx.x||state[1]) return;
 unsigned depth=state[0];
 if(flags[1]) {
   ++state[3];
   while(depth) {
     --depth;
     if(!choices[depth]) {
       for(unsigned i=0;i<count;++i)masks[i]=banks[(unsigned long long)depth*count+i];
       choices[depth]=1;masks[stack[depth]]=1;state[0]=depth+1;return;
     }
   }
   state[0]=0;state[1]=2;return;
 }
 unsigned next=~0u;
 for(unsigned i=0;i<plen&&next==~0u;++i)if(masks[p[i]]==3)next=p[i];
 for(unsigned i=0;i<qlen&&next==~0u;++i)if(masks[q[i]]==3)next=q[i];
 if(next!=~0u) {
   for(unsigned i=0;i<count;++i)banks[(unsigned long long)depth*count+i]=masks[i];
   stack[depth]=next;choices[depth]=0;masks[next]=2;
   state[0]=depth+1;++state[2];return;
 }
 unsigned a[MF_LIMBS]={0},b[MF_LIMBS]={0},product[MF_WIDE];
 for(unsigned i=0;i<plen;++i)if(masks[p[i]]==2)a[i/32]|=1u<<(i%32);
 for(unsigned i=0;i<qlen;++i)if(masks[q[i]]==2)b[i/32]|=1u<<(i%32);
 mf_mul(a,b,product,limbs);
 bool equal=true;
 for(unsigned i=0;i<2*limbs;++i)if(product[i]!=(i<limbs?n[i]:0))equal=false;
 unsigned one[MF_LIMBS]={1};
 if(equal&&mf_cmp(a,one,limbs)>0&&mf_cmp(b,one,limbs)>0
    &&mf_cmp(a,n,limbs)<0&&mf_cmp(b,n,limbs)<0) {
   for(unsigned i=0;i<limbs;++i){outp[i]=a[i];outq[i]=b[i];}
   state[1]=1;
 } else {
   // Product refusal is retained as a conflict and handled by the next control.
   state[1]=3;
 }
}

// Powerset-of-FOUR relation propagation. Bit positions name N,T,F,B;
// a set of alternatives is not an evidence value. Incompatible local rows
// deposit B without clearing their inputs or unrelated neighboring cells.
extern "C" __global__ void glut_belnap_relations(
 const unsigned *offsets,const unsigned *cells,const unsigned long long *supports,
 const unsigned long long *rows,unsigned gates,unsigned *domains,
 unsigned *held,unsigned *receipt) {
 if(blockIdx.x||threadIdx.x)return;
 unsigned sweeps=0,updates=0;bool changed;
 do {
   changed=false;++sweeps;
   for(unsigned g=0;g<gates;++g) {
     unsigned long long viable=rows[g];
     for(unsigned at=offsets[g];at<offsets[g+1];++at) {
       unsigned long long admitted=0;unsigned domain=domains[cells[at]];
       for(unsigned value=0;value<4;++value)if(domain&(1u<<value))admitted|=supports[4*at+value];
       viable&=admitted;
     }
     if(!viable){held[g]=3;continue;}
     for(unsigned at=offsets[g];at<offsets[g+1];++at) {
       unsigned allowed=0;
       for(unsigned value=0;value<4;++value)if(viable&supports[4*at+value])allowed|=1u<<value;
       unsigned cell=cells[at],next=domains[cell]&allowed;
       if(next!=domains[cell]){domains[cell]=next;++updates;changed=true;}
     }
   }
 }while(changed);
 unsigned evidence=1,count=0;
 for(unsigned g=0;g<gates;++g)if(held[g]){evidence=3;++count;}
 receipt[0]=evidence;receipt[1]=sweeps;receipt[2]=updates;receipt[3]=count;
}
