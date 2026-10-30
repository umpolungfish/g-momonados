#include <cuda_runtime.h>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <vector>
#include <fstream>
#include <stdexcept>
#define CUDA_OK(expr) do {cudaError_t err=(expr);if(err!=cudaSuccess){fprintf(stderr,"%s: %s\n",#expr,cudaGetErrorString(err));exit(1);}} while(0)
using Counter=unsigned long long;
struct Image {
    arm *state;int *continuation;Counter *receipt;
    Image(){CUDA_OK(cudaMalloc(&state,64*14*W));CUDA_OK(cudaMalloc(&continuation,64*6146*sizeof(int)));CUDA_OK(cudaMalloc(&receipt,650*sizeof(Counter)));}
    ~Image(){cudaFree(state);cudaFree(continuation);cudaFree(receipt);}
    void copy_from(const Image &other){
        CUDA_OK(cudaMemcpy(state,other.state,64*14*W,cudaMemcpyDeviceToDevice));
        CUDA_OK(cudaMemcpy(continuation,other.continuation,64*6146*sizeof(int),cudaMemcpyDeviceToDevice));
        CUDA_OK(cudaMemcpy(receipt,other.receipt,650*sizeof(Counter),cudaMemcpyDeviceToDevice));
    }
    void run(unsigned steps,int init=0){idempotent_membrane<<<1,32>>>(state,continuation,receipt,steps,init);CUDA_OK(cudaGetLastError());CUDA_OK(cudaDeviceSynchronize());}
    std::vector<arm> states(){std::vector<arm> v(32*14*W);CUDA_OK(cudaMemcpy(v.data(),state,v.size(),cudaMemcpyDeviceToHost));return v;}
    std::vector<int> continuations(){std::vector<int> v(32*6146);CUDA_OK(cudaMemcpy(v.data(),continuation,v.size()*sizeof(int),cudaMemcpyDeviceToHost));return v;}
    std::vector<Counter> receipts(){std::vector<Counter> v(330);CUDA_OK(cudaMemcpy(v.data(),receipt,v.size()*sizeof(Counter),cudaMemcpyDeviceToHost));return v;}
};
void require(bool pass,const char *name){if(!pass)throw std::runtime_error(name);}
void initialize(Image &image){
    CUDA_OK(cudaMemset(image.state,8,64*14*W));CUDA_OK(cudaMemset(image.continuation,0,64*6146*sizeof(int)));CUDA_OK(cudaMemset(image.receipt,0,650*sizeof(Counter)));
    std::vector<int> tags(Y_NODE_COUNT),left(Y_NODE_COUNT),right(Y_NODE_COUNT),cont(6146);
    CUDA_OK(cudaMemcpyFromSymbol(tags.data(),Y_TAG,tags.size()*sizeof(int)));
    CUDA_OK(cudaMemcpyFromSymbol(left.data(),Y_LEFT,left.size()*sizeof(int)));
    CUDA_OK(cudaMemcpyFromSymbol(right.data(),Y_RIGHT,right.size()*sizeof(int)));
    for(int i=0;i<Y_NODE_COUNT;++i){cont[3*i]=tags[i];cont[3*i+1]=left[i];cont[3*i+2]=right[i];}
    cont[6144]=Y_NODE_COUNT;cont[6145]=Y_ROOT;
    CUDA_OK(cudaMemcpy(image.continuation,cont.data(),cont.size()*sizeof(int),cudaMemcpyHostToDevice));
    image.run(32,1);
}
__global__ void branch_fixture(arm *state,int *continuation,Counter *receipt){
    ResidentBank bank{state,continuation,receipt+10};
    resident_copy(bank,resident_slot(bank,31));
    // Same candidate and residual, distinct retained bank evidence. This must
    // survive union and each member must retain its own Y continuation.
    if(lane()==0){state[31*14*W+7*W]^=1;receipt[4]=1u|0x80000000u;}
}
void check_laws(Image &prepared){
    auto before=prepared.states();auto y_before=prepared.continuations();auto r_before=prepared.receipts();
    Counter *device_failures,failures=~0ull;CUDA_OK(cudaMalloc(&device_failures,sizeof(Counter)));
    powerset_monad_laws<<<1,32>>>(device_failures);CUDA_OK(cudaGetLastError());
    CUDA_OK(cudaMemcpy(&failures,device_failures,sizeof(failures),cudaMemcpyDeviceToHost));require(failures==0,"mask monad laws");
    resident_powerset_monad_laws<<<1,32>>>(prepared.state,prepared.continuation,prepared.receipt,device_failures);CUDA_OK(cudaGetLastError());
    CUDA_OK(cudaMemcpy(&failures,device_failures,sizeof(failures),cudaMemcpyDeviceToHost));cudaFree(device_failures);require(failures==0,"resident monad laws");
    require(before==prepared.states()&&y_before==prepared.continuations()&&r_before==prepared.receipts(),"law instrument changed prepared state");
    Image combined,chunked,single;combined.copy_from(prepared);
    branch_fixture<<<1,32>>>(combined.state,combined.continuation,combined.receipt);CUDA_OK(cudaGetLastError());CUDA_OK(cudaDeviceSynchronize());
    chunked.copy_from(combined);single.copy_from(prepared);
    combined.run(2);chunked.run(1);chunked.run(1);single.run(2);
    auto a=combined.states(),b=single.states();auto ay=combined.continuations(),by=single.continuations();auto ar=combined.receipts(),br=single.receipts();
    require(ar[4]==(1u|0x80000000u),"two distinct full states collapsed");
    require(a==chunked.states()&&ay==chunked.continuations()&&ar==chunked.receipts(),"chunking changes resident state");
    require(std::equal(a.begin(),a.begin()+14*W,b.begin())&&std::equal(ay.begin(),ay.begin()+6146,by.begin())&&std::equal(ar.begin()+10,ar.begin()+20,br.begin()+10),"other branch changes first branch");
    Counter empty=0;CUDA_OK(cudaMemcpy(chunked.receipt+4,&empty,sizeof(empty),cudaMemcpyHostToDevice));chunked.run(1);require(chunked.receipts()[4]==0,"empty family resurrected");
}
static void save(const char *name,const void *data,size_t bytes){std::ofstream out(name,std::ios::binary);out.write((const char*)data,bytes);require(bool(out),"checkpoint write");}
int main(int argc,char **argv){
    try {
        unsigned seconds=argc>1?unsigned(std::stoul(argv[1])):300;
        CUDA_OK(cudaSetDevice(1));cudaDeviceProp prop;CUDA_OK(cudaGetDeviceProperties(&prop,1));
        require(std::string(prop.name).find("4070")!=std::string::npos,"expected RTX 4070");
        auto start=std::chrono::steady_clock::now();Image prepared;initialize(prepared);check_laws(prepared);
        double preparation=std::chrono::duration<double>(std::chrono::steady_clock::now()-start).count();
        // Extraction begins from the retained N-only preparation. Host polls
        // only terminal control status; register readout happens after stopping.
        auto trigger=std::chrono::steady_clock::now();Counter status=0;unsigned chunks=0;
        do {
            prepared.run(32);++chunks;
            CUDA_OK(cudaMemcpy(&status,prepared.receipt+3,sizeof(status),cudaMemcpyDeviceToHost));
        }while(status==0&&std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count()<seconds);
        double elapsed=std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        auto s=prepared.states();auto c=prepared.continuations();auto r=prepared.receipts();
        save("measurements/powerset_resident_20261015/returned.state",s.data(),s.size());
        save("measurements/powerset_resident_20261015/returned.continuation",c.data(),c.size()*sizeof(int));
        save("measurements/powerset_resident_20261015/returned.receipt",r.data(),r.size()*sizeof(Counter));
        printf("device=%s laws=passed branch_retention=passed chunk_continuation=passed N_only=true\npreparation_seconds=%.9f execution_seconds=%.9f chunks=%u status=%llu Y_calls=%llu binds=%llu\n",prop.name,preparation,elapsed,chunks,status,r[1],r[2]);
        for(unsigned slot=0;slot<32;++slot)if(r[4]&(1ull<<slot)){
            printf("slot=%u status=%llu\n",slot,r[10+slot*10+3]);
            for(unsigned field=0;field<14;++field){printf("field%u=",field);for(int bit=W-1;bit>=0;--bit)printf("%u",s[slot*14*W+field*W+bit]==F);printf("\n");}
        }
        return status==1?0:status==0?2:1;
    }catch(const std::exception &err){fprintf(stderr,"%s\n",err.what());return 1;}
}
