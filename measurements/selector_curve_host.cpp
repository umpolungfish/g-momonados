
#define __device__
#define JOINT_ROOT_BITS 14
struct Dim { unsigned x; } blockIdx, blockDim, threadIdx;
void atomicMin(unsigned long long *p,unsigned long long v) { if(v<*p)*p=v; }
void atomicAdd(unsigned long long *p,unsigned long long v) { *p+=v; }
#include "../src/gpu_joint.cuh"
extern "C" long long count(unsigned long long n,unsigned long long p,
 unsigned long long q,unsigned t,unsigned long long lo,unsigned long long hi) {
 return (long long)jstrip_count(jstrip(n,jnode(p,q,t),lo,hi),0,(hi-lo)/(1ULL<<t));
}
extern "C" bool select_curve(unsigned long long n,unsigned long long p,
 unsigned t,unsigned m,unsigned long long *factor) {
 JNode node=jnode(p,(n*jinv(p,(1ULL<<t)-1))&((1ULL<<t)-1),t);
 JBox box; JU products=0; *factor=n;
 if(!jbounds(n,node,m,box)) return true;
 return jcurve(n,node,box,*factor,products);
}
