// Boundary instrument only. Its reference arm is not a production input.
#define main baked_membrane_main
#include PREPARED_CPU_SOURCE
#undef main
#include SOURCE_FUSE_CONTROL

static Word from_bits(const char *bits) {
    Word word;
    for(int i=0;bits[i];++i)word.v[i]=bits[i]=='1'?F:T;
    return word;
}
int main(){
    const Word n=from_bits(CONTROL_N),p=from_bits(CONTROL_P),q=from_bits(CONTROL_Q);
    std::vector<arm> state(std::begin(PREPARED_STATE),std::end(PREPARED_STATE));
    std::vector<int> continuation(std::begin(PREPARED_CONTINUATION),std::end(PREPARED_CONTINUATION));
    std::vector<unsigned long long> receipt(std::begin(PREPARED_RECEIPT),std::end(PREPARED_RECEIPT));
    ResidentState original;resident_load({state.data(),continuation.data(),receipt.data()+10},original);
    std::array<arm,2*W+2> paired;paired.fill(0xab);
    auto equal_state=[&](const ResidentState &left,const ResidentState &right){
        std::vector<arm> a(14*W),b(14*W);
        std::array<unsigned long long,10> ar{},br{};
        resident_save(left,{a.data(),nullptr,ar.data()});
        resident_save(right,{b.data(),nullptr,br.data()});
        return a==b&&ar==br;
    };
    for(int choice=0;choice<3;++choice){
        Word arm_word;
        if(choice==0)small(2,arm_word);
        if(choice==1)arm_word=n;
        ResidentState returned=original;
        if(fuse_source_arm(arm_word,n,returned,paired.data()+1)||!equal_state(returned,original))return 1;
    }
    Word arm_word;add(p,p,arm_word);
    Word square;regular_mulmod(arm_word,arm_word,n,square);
    if(compare(square,arm_word)==0)return 2;
    ResidentState returned=original,expected=original;
    expected.divisor=p;expected.quotient=q;expected.status=8;
    if(!fuse_source_arm(arm_word,n,returned,paired.data()+1)||!equal_state(returned,expected))return 3;
    if(paired.front()!=0xab||paired.back()!=0xab)return 4;
    std::cout << "instrument=source-arm-fuse-boundary source_bits=" << CONTROL_BITS
        << " reference_arm_supplied=true production_receipt=false"
        << "\ncoprime_and_improper_rejected=true nonidempotent_arm_recovers_factor=true"
        << "\nexact_product=true remainder=0 retained_other_banks=true paired_neighbors_preserved=true\n";
}
