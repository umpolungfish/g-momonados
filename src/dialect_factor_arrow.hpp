// Inserted after the existing resident state and exact source fuse. The
// generated table is the live Rust meet/join/tensor, not a second ruleset.
#include <set>
#include <csignal>
#include <fstream>
struct ProductFrameState {
    Word matrix[4],left,right,anchor_left,anchor_right;
    bool initialized=false;
};
struct FibreStep {
    Word before, residual;
    std::vector<unsigned char> labels;
    std::vector<unsigned> addresses;
    unsigned rotation=0,dialect=0,promotion_axis=0;
    std::vector<unsigned> promoted_addresses;
    Word frame_left,frame_right,frame_determinant;
    bool frame_checked=false,frame_unit=false,frame_changed=false,frame_closed=false;
    bool point_updated=false,point_closed=false;
    Word support_value,support_derivative,promotion_shift,promotion_point,support_other_value,support_divided;
    bool support_evaluated=false,support_unit=false,support_closed=false,support_reseeded=false;
};
struct DialectMember {
    unsigned dialect=0, operation=0;
    std::vector<arm> state=std::vector<arm>(RESIDENT_WORDS*W,T);
    std::vector<int> continuation=std::vector<int>(Y_CONTINUATION,0);
    std::vector<unsigned long long> receipt=std::vector<unsigned long long>(RESIDENT_RECEIPT,0);
    std::vector<FibreStep> ancestry;
    std::vector<Word> deferred_source_arms;
    ProductFrameState product_frame;
    unsigned conflicts=0;
    unsigned long long changed=0, gate_admitted=0, t_admitted=0;
    ResidentBank bank(){return {state.data(),continuation.data(),receipt.data()};}
};
static DialectMember *dialect_member=nullptr;
static volatile std::sig_atomic_t dialect_return_requested=0;
void dialect_request_return(int){dialect_return_requested=1;}
unsigned nibble(const Word &w,unsigned index){
    unsigned value=0;
    for(unsigned bit=0;bit<4;++bit){unsigned i=4*index+bit;if(i<W&&digit(w,int(i))==F)value|=1u<<bit;}
    return value;
}
// Horner evaluation in the Crystal's actual mixed-radix tensor space,
// reduced in N's retained parity bank. Tuple coordinates remain in ancestry.
Word dialect_tensor_residue(const std::vector<unsigned> &coordinates,
    const std::vector<unsigned char> &labels,const Word &n){
    Word value,radix,entry,tmp;
    const unsigned long long base=DIALECT_KEEP_FIBRES?256ull*DIALECT_CRYSTAL_RADIX:DIALECT_CRYSTAL_RADIX;
    small(base,radix);
    for(std::size_t i=0;i<coordinates.size();++i){
        const unsigned long long address=DIALECT_KEEP_FIBRES?256ull*coordinates[i]+labels[i]:coordinates[i];
        regular_mulmod(value,radix,n,tmp);small(address,entry);addmod(tmp,entry,n,value);
    }
    return value;
}
Word dialect_image(const Word &source,const Word &value,unsigned rotation,
    unsigned dialect,unsigned operation,const Word &n){
    const unsigned count=(W-10+3)/4;
    std::vector<unsigned> addresses;std::vector<unsigned char> labels;
    for(unsigned j=0;j<count;++j){unsigned a=nibble(source,(j+rotation)%count),b=nibble(value,j);
        labels.push_back(static_cast<unsigned char>((a<<4)|b));
        addresses.push_back(DIALECT_ALGEBRA[dialect][operation][16*a+b].address);}
    return dialect_tensor_residue(addresses,labels,n);
}
Word dialect_plain_product(const Word &a,const Word &b){
    Word product;
    for(int i=W-1;i>=0;--i){add(product,product,product);if(digit(b,i)==F)add(product,a,product);}
    return product;
}
// The same ordered source-support polynomial as Vox phase_partners:
// coefficient i is N's parity digit i, hence P(2)=N exactly.
// Simultaneous Horner evaluation retains P(x) and its formal derivative.
void dialect_support_polynomial(const Word &source,const Word &x,const Word &n,
    Word &value,Word &derivative){
    Word next_value,next_derivative,unit;small(1,unit);
    clear(value);clear(derivative);
    for(int i=W-11;i>=0;--i){
        regular_mulmod(derivative,x,n,next_derivative);
        addmod(next_derivative,value,n,derivative);
        regular_mulmod(value,x,n,next_value);
        if(digit(source,i)==F)addmod(next_value,unit,n,value);else value=next_value;
    }
}
// The same coefficient object acts at both retained dialect points. The
// divided coefficient satisfies P(x)-P(y)=(x-y)D(x,y), including x=y.
void dialect_support_pair(const Word &source,const Word &x,const Word &y,const Word &n,
    Word &value,Word &derivative,Word &other,Word &divided){
    Word nv,nd,no,nq,unit;small(1,unit);
    clear(value);clear(derivative);clear(other);clear(divided);
    for(int i=W-11;i>=0;--i){
        regular_mulmod(divided,y,n,nq);addmod(nq,value,n,divided);
        regular_mulmod(other,y,n,no);
        regular_mulmod(derivative,x,n,nd);addmod(nd,value,n,derivative);
        regular_mulmod(value,x,n,nv);
        if(digit(source,i)==F){addmod(nv,unit,n,value);addmod(no,unit,n,other);}
        else{value=nv;other=no;}
    }
}
// A finite-ring operator acts on BOTH pending source arms. Its adjugate
// transports their product equation, and a unit determinant recovers the
// exact input pair. Nonunit determinants meet the existing source fuse.
void dialect_product_frame(FibreStep &step,const ResidentState &input,
    ResidentState &output,const Word &n,arm *paired,const Word &a,unsigned operation){
    auto &chart=dialect_member->product_frame;
    if(!chart.initialized){
        small(1,chart.matrix[0]);small(1,chart.matrix[3]);
        chart.anchor_left=input.divisor;chart.anchor_right=input.quotient;
        chart.left=chart.anchor_left;chart.right=chart.anchor_right;chart.initialized=true;
    }
    const Word p=chart.anchor_left,q=chart.anchor_right;
    const Word product=dialect_plain_product(p,q);
    Word residual;
    if(compare(n,product)>=0)subtract(n,product,residual);else subtract(product,n,residual);
    const Word previous_left=chart.left,previous_right=chart.right;
    Word b=dialect_image(n,chart.left,step.rotation,step.dialect,operation,n);
    Word c=dialect_image(n,chart.right,step.rotation,step.dialect,operation,n);
    Word d=dialect_image(n,residual,step.rotation,step.dialect,operation,n);
    Word ap,bq,cp,dq,ad,bc,back_p,back_q,tmp,expected_p,expected_q,det2,product_mod,transported;
    regular_mulmod(a,chart.left,n,ap);regular_mulmod(b,chart.right,n,bq);addmod(ap,bq,n,step.frame_left);
    regular_mulmod(c,chart.left,n,cp);regular_mulmod(d,chart.right,n,dq);addmod(cp,dq,n,step.frame_right);
    Word matrix[4];
    for(int col=0;col<2;++col){
        regular_mulmod(a,chart.matrix[col],n,ap);regular_mulmod(b,chart.matrix[2+col],n,bq);addmod(ap,bq,n,matrix[col]);
        regular_mulmod(c,chart.matrix[col],n,cp);regular_mulmod(d,chart.matrix[2+col],n,dq);addmod(cp,dq,n,matrix[2+col]);
    }
    for(int i=0;i<4;++i)chart.matrix[i]=matrix[i];
    chart.left=step.frame_left;chart.right=step.frame_right;
    const Word &ma=chart.matrix[0],&mb=chart.matrix[1],&mc=chart.matrix[2],&md=chart.matrix[3];
    regular_mulmod(ma,md,n,ad);regular_mulmod(mb,mc,n,bc);submod(ad,bc,n,step.frame_determinant);
    regular_mulmod(ma,p,n,ap);regular_mulmod(mb,q,n,bq);addmod(ap,bq,n,tmp);
    if(!dialect_transport_active&&compare(tmp,chart.left)){output.status=12;return;}
    regular_mulmod(mc,p,n,cp);regular_mulmod(md,q,n,dq);addmod(cp,dq,n,tmp);
    if(!dialect_transport_active&&compare(tmp,chart.right)){output.status=12;return;}
    regular_mulmod(md,step.frame_left,n,back_p);regular_mulmod(mb,step.frame_right,n,tmp);submod(back_p,tmp,n,back_p);
    regular_mulmod(ma,step.frame_right,n,back_q);regular_mulmod(mc,step.frame_left,n,tmp);submod(back_q,tmp,n,back_q);
    regular_mulmod(step.frame_determinant,p,n,expected_p);regular_mulmod(step.frame_determinant,q,n,expected_q);
    regular_mulmod(step.frame_determinant,step.frame_determinant,n,det2);
    regular_mulmod(p,q,n,product_mod);regular_mulmod(det2,product_mod,n,expected_p);
    regular_mulmod(back_p,back_q,n,transported);
    if(!dialect_transport_active&&(compare(transported,expected_p)||compare(back_q,expected_q))){
        output.status=12;return;
    }
    regular_mulmod(step.frame_determinant,p,n,expected_p);
    if(!dialect_transport_active&&compare(back_p,expected_p)){output.status=12;return;}
    Word inverse_det,recovered_p,recovered_q;
    step.frame_unit=!dialect_transport_active&&inverse(step.frame_determinant,n,inverse_det);
    if(step.frame_unit){
        regular_mulmod(back_p,inverse_det,n,recovered_p);regular_mulmod(back_q,inverse_det,n,recovered_q);
        if(compare(recovered_p,p)||compare(recovered_q,q)||compare(dialect_plain_product(recovered_p,recovered_q),product)){
            output.status=12;return;
        }
    }
    step.frame_checked=!dialect_transport_active;
    step.frame_changed=compare(step.frame_left,previous_left)||compare(step.frame_right,previous_right);
    for(const Word *probe:{&step.frame_determinant,&step.frame_left,&step.frame_right,&back_p,&back_q}){
        if(fuse_source_arm(*probe,n,output,paired)){step.frame_closed=output.status==8;return;}
    }
    if(DIALECT_COFACTOR_FEEDBACK){
        // The source parity split of the carried chart supplies an operand.
        // Its cofactor and exact carry stay coupled to N in this same frame.
        const Word &boundary=operation==2?step.frame_right:step.frame_left;
        const unsigned parity=operation==1?1u:0u;
        Word proposal;
        for(unsigned i=0;i<unsigned(W);++i)
            proposal.v[i]=2*i+parity<unsigned(W)?digit(boundary,int(2*i+parity)):T;
        if(DIALECT_SUPPORT_NEWTON){
            Word support_point=p;
            if(DIALECT_PROMOTION_RETURN){
                const Word unpromoted=dialect_tensor_residue(step.addresses,step.labels,n);
                submod(a,unpromoted,n,step.promotion_shift);
                addmod(p,step.promotion_shift,n,support_point);
                step.promotion_point=support_point;
                Word recovered;submod(support_point,step.promotion_shift,n,recovered);
                if(!dialect_transport_active&&compare(recovered,p)){output.status=14;return;}
            }
            if(DIALECT_SUPPORT_SECANT)
                dialect_support_pair(n,support_point,p,n,step.support_value,step.support_derivative,step.support_other_value,step.support_divided);
            else dialect_support_polynomial(n,support_point,n,step.support_value,step.support_derivative);
            step.support_evaluated=true;
            Word inverse_derivative,two,root_return,correction,next_point;
            small(2,two);submod(support_point,two,n,root_return);
            const Word &denominator=DIALECT_SUPPORT_SECANT?step.support_divided:step.support_derivative;
            step.support_unit=inverse(denominator,n,inverse_derivative);
            for(const Word *probe:{&step.support_derivative,&step.support_value,&root_return}){
                if(fuse_source_arm(*probe,n,output,paired)){step.support_closed=output.status==8;return;}
            }
            if(DIALECT_SUPPORT_SECANT){
                for(const Word *witness:{&step.support_divided,&step.support_other_value,&step.promotion_shift})
                    fuse_source_arm(*witness,n,output,paired);
            }
            if(step.support_unit&&!zero(step.support_value)){
                regular_mulmod(step.support_value,inverse_derivative,n,correction);
                submod(support_point,correction,n,next_point);
                if(!zero(next_point)&&!one(next_point)&&compare(next_point,n)<0)proposal=next_point;
                else step.support_reseeded=true;
            }else step.support_reseeded=true;
        }
        if(!zero(proposal)&&!one(proposal)&&compare(proposal,n)<0){
            Word quotient,remainder,check;
            quotient_word(n,proposal,quotient,remainder);
            check=dialect_plain_product(proposal,quotient);add(check,remainder,check);
            if(!dialect_transport_active&&compare(check,n)){output.status=13;return;}
            chart.anchor_left=proposal;chart.anchor_right=quotient;
            regular_mulmod(ma,proposal,n,ap);regular_mulmod(mb,quotient,n,bq);addmod(ap,bq,n,chart.left);
            regular_mulmod(mc,proposal,n,cp);regular_mulmod(md,quotient,n,dq);addmod(cp,dq,n,chart.right);
            if(step.frame_unit){
                regular_mulmod(md,chart.left,n,back_p);regular_mulmod(mb,chart.right,n,tmp);submod(back_p,tmp,n,back_p);
                regular_mulmod(ma,chart.right,n,back_q);regular_mulmod(mc,chart.left,n,tmp);submod(back_q,tmp,n,back_q);
                regular_mulmod(back_p,inverse_det,n,recovered_p);regular_mulmod(back_q,inverse_det,n,recovered_q);
                if(compare(recovered_p,proposal)||compare(recovered_q,quotient)){output.status=13;return;}
            }
            step.point_updated=true;
            if(fuse_source_arm(proposal,n,output,paired))step.point_closed=output.status==8;
        }
    }
}
bool dialect_recover_fibre(const FibreStep &step,const Word &n,const Word &image,
    unsigned dialect,unsigned operation){
    const unsigned count=(W-10+3)/4;
    if(step.labels.size()!=count||step.addresses.size()!=count)return false;
    Word source,preimage;
    for(unsigned j=0;j<count;++j){
        unsigned a=step.labels[j]>>4,b=step.labels[j]&15u;
        unsigned position=(j+step.rotation)%count;
        const auto &cell=DIALECT_ALGEBRA[dialect][operation][16*a+b];
        if(cell.address!=step.addresses[j])return false;
        for(unsigned bit=0;bit<4;++bit){
            if(4*position+bit<unsigned(W))source.v[4*position+bit]=(a&(1u<<bit))?F:T;
            if(4*j+bit<unsigned(W))preimage.v[4*j+bit]=(b&(1u<<bit))?F:T;
        }
    }
    if(compare(source,n)||compare(preimage,step.before))return false;
    Word square,residual;regular_mulmod(preimage,preimage,n,square);submod(square,preimage,n,residual);
    if(compare(residual,step.residual))return false;
    return compare(dialect_tensor_residue(step.addresses,step.labels,n),image)==0;
}
struct DialectArrow {
    const Word &n;arm *paired;
    unsigned operator()(unsigned coordinate,const ResidentState &input,ResidentState &output)const{
        output=input;
        if(!dialect_member){output.status=10;return powerset_unit(coordinate);}
        if(!dialect_transport_active&&!sic_zeta_resident(output.e,output.residual,output.exponent,output.seed,
            output.power,output.frame_residual)){output.status=7;return powerset_unit(coordinate);}
        if(!dialect_transport_active)++output.sic_roundtrips;
        auto &member=*dialect_member;
        const unsigned count=(W-10+3)/4;
        FibreStep step;step.before=output.e;step.residual=output.residual;
        const unsigned target=DIALECT_HOP
            ?unsigned((member.dialect+input.y_calls-PREPARED_DIALECT_RECEIPT[1]-1)%DIALECT_COUNT)
            :member.dialect;
        step.dialect=target;
        // ROTAT moves the source ring against the candidate ring. Both label
        // sequences and their relative phase survive the return.
        step.rotation=unsigned((input.y_calls-1)%count);
        for(unsigned j=0;j<count;++j){
            unsigned a=nibble(n,(j+step.rotation)%count),b=nibble(output.e,j);
            const auto &cell=DIALECT_ALGEBRA[target][member.operation][16*a+b];
            step.labels.push_back(static_cast<unsigned char>((a<<4)|b));
            step.addresses.push_back(cell.address);
            member.conflicts|=cell.conflicts;
            member.gate_admitted+=cell.gates;
            member.t_admitted+=cell.time;
        }
        Word image=dialect_tensor_residue(step.addresses,step.labels,n);
        if(!dialect_transport_active&&!dialect_recover_fibre(step,n,image,target,member.operation)){
            output.status=11;return powerset_unit(coordinate);
        }
        if(DIALECT_PROMOTION_RETURN){
            step.promotion_axis=unsigned((input.y_calls-1)%12);
            const unsigned axis=step.promotion_axis,stride=DIALECT_PROMOTION_STRIDES[axis];
            for(unsigned address:step.addresses){
                unsigned from=(address/stride)%DIALECT_PROMOTION_CARDS[axis];
                unsigned promoted=address-from*stride+DIALECT_PROMOTION_TARGET[axis]*stride;
                // The original address, signed coordinate change, exact labels,
                // ruleset and selected axis form the return certificate.
                if(promoted-DIALECT_PROMOTION_TARGET[axis]*stride+from*stride!=address){
                    output.status=14;return powerset_unit(coordinate);
                }
                step.promoted_addresses.push_back(promoted);
            }
            image=dialect_tensor_residue(step.promoted_addresses,step.labels,n);
        }
        member.changed+=compare(image,output.e)!=0;
        output.e=image;
        Word square,unit;small(1,unit);
        regular_mulmod(image,image,n,square);submod(square,image,n,output.residual);
        addmod(image,image,n,output.power);submod(output.power,unit,n,output.power);
        add(output.exponent,unit,output.exponent);
        add(n,unit,output.complement);subtract(output.complement,image,output.complement);
        if(DIALECT_PRODUCT_FRAME){
            dialect_product_frame(step,input,output,n,paired,image,member.operation);
            if(output.status){member.ancestry.push_back(std::move(step));return powerset_unit(coordinate);}
        }
        member.ancestry.push_back(std::move(step));
        if(fuse_source_arm(image,n,output,paired)||fuse_source_arm(output.complement,n,output,paired))
            return powerset_unit(coordinate);
        if(DIALECT_COMPOSE_IDEMPOTENT){
            ResidentState continued;
            IdempotentArrow{n,paired}(coordinate,output,continued);
            output=continued;
            if(!output.status){
                if(fuse_source_arm(output.e,n,output,paired)||fuse_source_arm(output.complement,n,output,paired))
                    return powerset_unit(coordinate);
            }
        }
        return powerset_unit(coordinate);
    }
};
