std::string dialect_numeral(const arm *cells){
    int end=W;while(end&&cells[end-1]==T)--end;
    std::string word="⊢";
    for(int i=0;i<end;++i){if(cells[i]!=T&&cells[i]!=F)throw std::runtime_error("invalid parity cell");
        word+=cells[i]==F?"≻⋈∈⊥∋":"≻⋈∈⊤∋";}
    return word+"⊙⊡⊣";
}
template<class Values>void dialect_array(const Values &values){
    std::cout<<'[';bool first=true;for(const auto &v:values){if(!first)std::cout<<',';first=false;std::cout<<+v;}std::cout<<']';
}
void dialect_step(DialectMember &member,const Word &n){
    DialectMember next; // banked input includes the complete continuation
    next.dialect=member.dialect;next.operation=member.operation;
    next.state=member.state;next.continuation=member.continuation;next.receipt=member.receipt;
    next.product_frame=member.product_frame;
    next.conflicts=member.conflicts;next.changed=member.changed;
    next.gate_admitted=member.gate_admitted;next.t_admitted=member.t_admitted;
    next.deferred_source_arms=std::move(member.deferred_source_arms);
    dialect_member=&next;dialect_pending_arms=&next.deferred_source_arms;
    const bool previous_transport=dialect_transport_active;dialect_transport_active=true;
    YNode nodes[2048]{};int used=0,root=0,action=0;arm paired[2*W]{};
    resident_transition(member.bank(),next.bank(),n,nodes,used,root,action,paired);
    dialect_member=nullptr;dialect_pending_arms=nullptr;dialect_transport_active=previous_transport;
    member.deferred_source_arms=std::move(next.deferred_source_arms);
    auto returned=std::move(next.ancestry);
    member.state=std::move(next.state);member.continuation=std::move(next.continuation);
    member.receipt=std::move(next.receipt);member.conflicts=next.conflicts;
    member.product_frame=next.product_frame;
    member.changed=next.changed;member.gate_admitted=next.gate_admitted;member.t_admitted=next.t_admitted;
    for(auto &fibre:returned)member.ancestry.push_back(std::move(fibre));
}
// The dynamic family is a powerset of complete resident coordinates. Unlike
// the GPU's warp mask it admits every dialect and all operation alternatives.
using DialectHeld=std::set<std::size_t>;
using DialectRelation=std::vector<DialectHeld>;
DialectHeld dialect_bind(const DialectHeld &held,const DialectRelation &arrow){
    DialectHeld out;for(auto x:held){const auto &ys=arrow.at(x);out.insert(ys.begin(),ys.end());}return out;
}
bool dialect_same_member(const DialectMember &a,const DialectMember &b){
    if(a.deferred_source_arms.size()!=b.deferred_source_arms.size())return false;
    for(std::size_t i=0;i<a.deferred_source_arms.size();++i)
        if(compare(a.deferred_source_arms[i],b.deferred_source_arms[i]))return false;
    if(a.product_frame.initialized!=b.product_frame.initialized||compare(a.product_frame.left,b.product_frame.left)||compare(a.product_frame.right,b.product_frame.right))return false;
    if(compare(a.product_frame.anchor_left,b.product_frame.anchor_left)||compare(a.product_frame.anchor_right,b.product_frame.anchor_right))return false;
    for(int i=0;i<4;++i)if(compare(a.product_frame.matrix[i],b.product_frame.matrix[i]))return false;
    if(a.dialect!=b.dialect||a.operation!=b.operation||a.state!=b.state||a.continuation!=b.continuation||
        a.receipt!=b.receipt||a.conflicts!=b.conflicts||a.changed!=b.changed||a.gate_admitted!=b.gate_admitted||
        a.t_admitted!=b.t_admitted||a.ancestry.size()!=b.ancestry.size())return false;
    for(std::size_t i=0;i<a.ancestry.size();++i){const auto &x=a.ancestry[i],&y=b.ancestry[i];
        if(compare(x.before,y.before)||compare(x.residual,y.residual)||x.labels!=y.labels||x.addresses!=y.addresses||x.promotion_axis!=y.promotion_axis||x.promoted_addresses!=y.promoted_addresses||x.rotation!=y.rotation||x.dialect!=y.dialect||
            compare(x.frame_left,y.frame_left)||compare(x.frame_right,y.frame_right)||compare(x.frame_determinant,y.frame_determinant)||
            x.frame_checked!=y.frame_checked||x.frame_unit!=y.frame_unit||x.frame_changed!=y.frame_changed||x.frame_closed!=y.frame_closed||x.point_updated!=y.point_updated||x.point_closed!=y.point_closed||compare(x.promotion_shift,y.promotion_shift)||compare(x.promotion_point,y.promotion_point)||compare(x.support_other_value,y.support_other_value)||compare(x.support_divided,y.support_divided)||compare(x.support_value,y.support_value)||compare(x.support_derivative,y.support_derivative)||x.support_evaluated!=y.support_evaluated||x.support_unit!=y.support_unit||x.support_closed!=y.support_closed||x.support_reseeded!=y.support_reseeded||x.support_split!=y.support_split||compare(x.support_corrected,y.support_corrected)||compare(x.support_split_left,y.support_split_left)||compare(x.support_split_right,y.support_split_right))return false;}
    return true;
}
void dialect_laws(const std::vector<DialectMember> &family){
    if(family.size()<4)throw std::runtime_error("missing resident alternatives");
    DialectHeld s{0,1,2,3};DialectRelation id(family.size()),f(family.size()),g(family.size()),fg(family.size());
    for(std::size_t i=0;i<family.size();++i){id[i]={i};f[i]={i,(i+1)%family.size()};g[i]={(i+3)%family.size()};}
    f[3]={};
    for(std::size_t i=0;i<family.size();++i){
        fg[i]=dialect_bind(f[i],g);
        if(dialect_bind({i},f)!=f[i])throw std::runtime_error("left unit failed");
    }
    if(dialect_bind(s,id)!=s||dialect_bind(dialect_bind(s,f),g)!=dialect_bind(s,fg)||
        !dialect_bind({},g).empty())throw std::runtime_error("resident powerset law failed");
    std::set<DialectHeld> grouped{s},separated;for(auto x:s)separated.insert({x});
    if(grouped==separated)throw std::runtime_error("nested distinctions lost before multiplication");
    auto flatten=[](const std::set<DialectHeld> &nested){DialectHeld out;for(const auto &xs:nested)out.insert(xs.begin(),xs.end());return out;};
    if(flatten(grouped)!=flatten(separated))throw std::runtime_error("multiplication failed");
    // Actual Y/SIC/dialect arrows on the executing banks, with equality of
    // the complete value and continuation rather than slot-number equality.
    Word n;load_word(SOURCE_ARMS,n);
    std::vector<DialectMember> pool{family[0],family[3]};
    auto intern=[&](DialectMember member){
        for(std::size_t i=0;i<pool.size();++i)if(dialect_same_member(pool[i],member))return i;
        pool.push_back(std::move(member));return pool.size()-1;
    };
    auto arrow=[&](std::size_t i){auto next=pool.at(i);dialect_step(next,n);return DialectHeld{intern(std::move(next))};};
    auto bind_arrow=[&](const DialectHeld &xs,auto &&fn){DialectHeld result;for(auto x:xs){auto ys=fn(x);result.insert(ys.begin(),ys.end());}return result;};
    DialectHeld actual{0,1};
    for(auto x:actual)if(bind_arrow(DialectHeld{x},arrow)!=arrow(x))throw std::runtime_error("executing left unit failed");
    if(bind_arrow(actual,[](std::size_t x){return DialectHeld{x};})!=actual)throw std::runtime_error("executing right unit failed");
    auto lhs=bind_arrow(bind_arrow(actual,arrow),arrow);
    auto rhs=bind_arrow(actual,[&](std::size_t x){return bind_arrow(arrow(x),arrow);});
    if(lhs!=rhs)throw std::runtime_error("executing bind associativity failed");
    if(!dialect_same_member(pool[0],family[0])||!dialect_same_member(pool[1],family[3]))throw std::runtime_error("banked inputs changed during composition");
}
int main(int argc,char **){
    try{
        if(argc!=1)throw std::runtime_error("baked dialect membrane accepts no instance inputs");
        std::signal(SIGUSR1,dialect_request_return);
        const auto trigger=std::chrono::steady_clock::now();
        Word n;load_word(SOURCE_ARMS,n);
        std::vector<DialectMember> family;
        for(unsigned dialect=0;dialect<DIALECT_COUNT;++dialect)for(unsigned operation=0;operation<3;++operation){
            DialectMember member;member.dialect=dialect;member.operation=operation;
            const unsigned slot=3*dialect+operation;
            std::copy_n(dialect_initial_state(slot),RESIDENT_WORDS*W,member.state.begin());
            std::copy_n(dialect_initial_continuation(slot),Y_CONTINUATION,member.continuation.begin());
            std::copy_n(dialect_initial_receipt(slot),RESIDENT_RECEIPT,member.receipt.begin());
            const auto metadata=dialect_initial_metadata(slot);
            member.conflicts=unsigned(metadata[0]);member.changed=metadata[1];member.gate_admitted=metadata[2];member.t_admitted=metadata[3];
            if(dialect_initial_frame_ready(slot)){
                const arm *words=dialect_initial_frame(slot);
                for(unsigned i=0;i<4;++i)load_word(words+i*W,member.product_frame.matrix[i]);
                load_word(words+4*W,member.product_frame.left);load_word(words+5*W,member.product_frame.right);
                load_word(words+6*W,member.product_frame.anchor_left);load_word(words+7*W,member.product_frame.anchor_right);
                member.product_frame.initialized=true;
            }
            for(auto j=RESUMED_DEFERRED_OFFSETS[slot];j<RESUMED_DEFERRED_OFFSETS[slot+1];++j){
                Word witness;load_word(RESUMED_DEFERRED_ARMS+j*W,witness);member.deferred_source_arms.push_back(witness);
            }
            family.push_back(std::move(member));
        }
        bool runtime_laws=false;
#ifdef DIALECT_ONE_SWEEP
        {
            ResidentState isolated{};Word witness;small(1,witness);arm paired[2*W]{};
            bool rejected=false;dialect_transport_active=true;
            try{terminal_source_arm(witness,n,isolated,paired);}catch(const std::runtime_error &){rejected=true;}
            dialect_transport_active=false;
            if(!rejected||dialect_transport_probe_calls!=1)throw std::runtime_error("transport readout guard failed");
            dialect_transport_probe_calls=0;
        }
        dialect_laws(family);runtime_laws=true;
#endif
        DialectHeld held;DialectRelation relation(family.size());
        for(std::size_t i=0;i<family.size();++i){held.insert(i);relation[i]={i};}
        std::size_t closed=family.size();unsigned long long rounds=0;
        dialect_transport_active=true;
        do{
            for(auto index:held){
                auto &member=family[index];dialect_step(member,n);
                const int status=int(member.receipt[3]);
                if(status==1||status==8){closed=index;break;}
                if(status)throw std::runtime_error("dialect transition status "+std::to_string(status));
                if(dialect_return_requested)break;
            }
            held=dialect_bind(held,relation);++rounds;
#ifdef DIALECT_ONE_SWEEP
            break;
#endif
        }while(closed==family.size()&&!dialect_return_requested);
        dialect_transport_active=false;dialect_pending_arms=nullptr;
        const double elapsed=std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        bool exact=false,paired_ok=false;double verified=0;
        ResidentState readout{};std::size_t readout_member=family.size(),readout_witness=0;
        unsigned long long readout_count=0,terminal_sic_readbacks=0,terminal_unchanged_members=0,terminal_frame_readbacks=0,terminal_support_parity_returns=0;
        for(std::size_t i=0;i<family.size()&&readout_member==family.size();++i){
            const DialectMember before_readout=family[i];
            for(const auto &step:before_readout.ancestry)if(step.support_split){
                for(int cell=0;cell<W;++cell)
                    if(digit(step.support_corrected,cell)!=digit(cell%2?step.support_split_right:step.support_split_left,cell/2))
                        throw std::runtime_error("terminal support parity return failed");
                ++terminal_support_parity_returns;
            }
            ResidentState frozen;resident_load(family[i].bank(),frozen);
            if(!sic_zeta_resident(frozen.e,frozen.residual,frozen.exponent,frozen.seed,frozen.power,frozen.frame_residual))
                throw std::runtime_error("terminal SIC recovery failed");
            ++terminal_sic_readbacks;
            const auto &frame=family[i].product_frame;
            if(frame.initialized){
                Word ad,bc,det,inverse_det,tmp,left,right;
                regular_mulmod(frame.matrix[0],frame.matrix[3],n,ad);
                regular_mulmod(frame.matrix[1],frame.matrix[2],n,bc);submod(ad,bc,n,det);
                if(inverse(det,n,inverse_det)){
                    regular_mulmod(frame.matrix[3],frame.left,n,left);regular_mulmod(frame.matrix[1],frame.right,n,tmp);submod(left,tmp,n,left);
                    regular_mulmod(frame.matrix[0],frame.right,n,right);regular_mulmod(frame.matrix[2],frame.left,n,tmp);submod(right,tmp,n,right);
                    regular_mulmod(left,inverse_det,n,left);regular_mulmod(right,inverse_det,n,right);
                    if(compare(left,frame.anchor_left)||compare(right,frame.anchor_right))throw std::runtime_error("terminal frame recovery failed");
                }
                ++terminal_frame_readbacks;
            }
            for(std::size_t j=0;j<family[i].deferred_source_arms.size();++j){
                ResidentState candidate=frozen;arm paired[2*W]{};++readout_count;
                if(terminal_source_arm(family[i].deferred_source_arms[j],n,candidate,paired)){
                    if(candidate.status!=8)throw std::runtime_error("terminal source closure failed");
                    readout=candidate;readout_member=i;readout_witness=j;break;
                }
            }
            if(!dialect_same_member(before_readout,family[i]))throw std::runtime_error("terminal readout perturbed frozen member");
            ++terminal_unchanged_members;
        }
        if(readout_member<family.size()){
            exact=true;paired_ok=true;
            verified=std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        }
        if(closed<family.size()){
            ResidentState result;resident_load(family[closed].bank(),result);ResidentState check=result;arm paired[2*W]{};
            exact=fuse_source_arm(result.divisor,n,check,paired)&&compare(check.divisor,result.divisor)==0&&compare(check.quotient,result.quotient)==0;
            paired_ok=exact;
            if(!exact)throw std::runtime_error("returned factor failed exact canonical source closure");
            verified=std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        }
        const double readout_elapsed=std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        std::cout<<"{\"format\":\"resident-dialect-return-v1\",\"source_word\":\""<<dialect_numeral(SOURCE_ARMS)
            <<"\",\"rounds\":"<<rounds<<",\"held\":"<<held.size()<<",\"monad_laws\":"<<(runtime_laws?"true":"false")<<",\"source_preserved\":true"
            <<",\"history_encoding\":\"exact-source-candidate-fibres\""
            <<",\"keep_fibres\":"<<(DIALECT_KEEP_FIBRES?"true":"false")
            <<",\"compose_idempotent\":"<<(DIALECT_COMPOSE_IDEMPOTENT?"true":"false")
            <<",\"successive_dialect_hops\":"<<(DIALECT_HOP?"true":"false")
            <<",\"product_frame\":"<<(DIALECT_PRODUCT_FRAME?"true":"false")
            <<",\"terminal_only_frame_recovery\":true,\"terminal_only_source_readout\":true,\"transport_source_probe_calls\":"<<dialect_transport_probe_calls
            <<",\"terminal_witnesses_checked\":"<<readout_count<<",\"terminal_sic_readbacks\":"<<terminal_sic_readbacks
            <<",\"terminal_support_parity_returns\":"<<terminal_support_parity_returns
            <<",\"terminal_unchanged_members\":"<<terminal_unchanged_members<<",\"terminal_frame_readbacks\":"<<terminal_frame_readbacks
            <<",\"cumulative_product_frame\":"<<(DIALECT_PRODUCT_FRAME?"true":"false")
            <<",\"promotion_return\":"<<(DIALECT_PROMOTION_RETURN?"true":"false")
            <<",\"support_secant\":"<<(DIALECT_SUPPORT_SECANT?"true":"false")
            <<",\"support_parity\":"<<(DIALECT_SUPPORT_PARITY?"true":"false")
            <<",\"support_newton\":"<<(DIALECT_SUPPORT_NEWTON?"true":"false")
            <<",\"cofactor_feedback\":"<<(DIALECT_COFACTOR_FEEDBACK?"true":"false")
            <<",\"trigger_to_transport_end_seconds\":"<<elapsed<<",\"terminal_readout_seconds\":"<<(readout_elapsed-elapsed)
            <<",\"trigger_to_readout_seconds\":"<<readout_elapsed<<",\"closed\":"<<(exact?"true":"false")
            <<",\"paired_boundary\":"<<(paired_ok?"true":"false")<<",\"trigger_to_verified_factor_seconds\":"<<verified
            <<",\"parent_return_sha256\":\""<<PARENT_RETURN_SHA256<<"\",\"parent_return_gzip_hex\":\""<<PARENT_RETURN_GZIP_HEX<<"\"";
        std::cout<<",\"terminal_factor_readout\":{\"closed\":"<<(exact?"true":"false");
        if(exact){std::cout<<",\"member\":"<<readout_member<<",\"witness\":"<<readout_witness
            <<",\"divisor\":\""<<dialect_numeral(readout.divisor.v)<<"\",\"quotient\":\""<<dialect_numeral(readout.quotient.v)<<"\"";}
        std::cout<<"},\"members\":[";
        for(std::size_t i=0;i<family.size();++i){if(i)std::cout<<',';auto &m=family[i];
            std::cout<<"{\"dialect\":"<<m.dialect<<",\"operation\":"<<m.operation<<",\"changed\":"<<m.changed
                <<",\"conflicts\":"<<m.conflicts<<",\"gate_admitted\":"<<m.gate_admitted<<",\"t_admitted\":"<<m.t_admitted<<",\"state\":";
            dialect_array(m.state);std::cout<<",\"continuation\":";dialect_array(m.continuation);
            std::cout<<",\"receipt\":";dialect_array(m.receipt);
            std::cout<<",\"deferred_source_arms\":[";
            for(std::size_t j=0;j<m.deferred_source_arms.size();++j){if(j)std::cout<<',';dialect_array(m.deferred_source_arms[j].v);}std::cout<<']';
            if(DIALECT_PRODUCT_FRAME){
                std::cout<<",\"product_frame_state\":{\"initialized\":"<<(m.product_frame.initialized?"true":"false")<<",\"matrix\":[";
                for(int row=0;row<4;++row){if(row)std::cout<<',';dialect_array(m.product_frame.matrix[row].v);}
                std::cout<<"],\"left\":";dialect_array(m.product_frame.left.v);
                std::cout<<",\"right\":";dialect_array(m.product_frame.right.v);
                std::cout<<",\"anchor_left\":";dialect_array(m.product_frame.anchor_left.v);
                std::cout<<",\"anchor_right\":";dialect_array(m.product_frame.anchor_right.v);std::cout<<'}';
            }
            std::cout<<",\"ancestry\":[";
            for(std::size_t j=0;j<m.ancestry.size();++j){if(j)std::cout<<',';const auto &a=m.ancestry[j];
                // Labels recover both source and candidate exactly; their
                // residual is fixed by e(e-1) mod N. Avoid repeating large
                // numeral strings whose values the fibre already retains.
                std::cout<<"{\"ruleset\":"<<a.dialect<<",\"rotation\":"<<a.rotation<<",\"labels\":";
                dialect_array(a.labels);std::cout<<",\"addresses\":";dialect_array(a.addresses);
                if(DIALECT_PROMOTION_RETURN){
                    std::cout<<",\"promotion_axis\":"<<a.promotion_axis<<",\"promoted_addresses\":";
                    dialect_array(a.promoted_addresses);
                }
                if(DIALECT_PRODUCT_FRAME){
                    std::cout<<",\"frame_left\":";dialect_array(a.frame_left.v);
                    std::cout<<",\"frame_right\":";dialect_array(a.frame_right.v);
                    std::cout<<",\"frame_determinant\":";dialect_array(a.frame_determinant.v);
                    std::cout<<",\"frame_checked\":"<<(a.frame_checked?"true":"false")
                        <<",\"frame_unit\":"<<(a.frame_unit?"true":"false")
                        <<",\"frame_changed\":"<<(a.frame_changed?"true":"false")
                        <<",\"frame_closed\":"<<(a.frame_closed?"true":"false")
                        <<",\"point_updated\":"<<(a.point_updated?"true":"false")
                        <<",\"point_closed\":"<<(a.point_closed?"true":"false");
                    if(DIALECT_SUPPORT_NEWTON){
                        if(DIALECT_PROMOTION_RETURN){
                            std::cout<<",\"promotion_shift\":";dialect_array(a.promotion_shift.v);
                            std::cout<<",\"promotion_point\":";dialect_array(a.promotion_point.v);
                        }
                        std::cout<<",\"support_value\":";dialect_array(a.support_value.v);
                        std::cout<<",\"support_derivative\":";dialect_array(a.support_derivative.v);
                        if(DIALECT_SUPPORT_SECANT){
                            std::cout<<",\"support_other_value\":";dialect_array(a.support_other_value.v);
                            std::cout<<",\"support_divided\":";dialect_array(a.support_divided.v);
                        }
                        if(DIALECT_SUPPORT_PARITY){
                            std::cout<<",\"support_split\":"<<(a.support_split?"true":"false");
                            std::cout<<",\"support_corrected\":";dialect_array(a.support_corrected.v);
                            std::cout<<",\"support_split_left\":";dialect_array(a.support_split_left.v);
                            std::cout<<",\"support_split_right\":";dialect_array(a.support_split_right.v);
                        }
                        std::cout<<",\"support_evaluated\":"<<(a.support_evaluated?"true":"false")
                            <<",\"support_unit\":"<<(a.support_unit?"true":"false")
                            <<",\"support_closed\":"<<(a.support_closed?"true":"false")
                            <<",\"support_reseeded\":"<<(a.support_reseeded?"true":"false");
                    }
                }
                std::cout<<'}';}
            std::cout<<"]}";
        }
        std::cout<<"]}\n";
        if(exact){
            std::cerr<<"factor "<<dialect_numeral(readout.divisor.v)<<"\nfactor "<<dialect_numeral(readout.quotient.v)<<'\n';}
        return exact?0:2;
    }catch(const std::exception &e){std::cerr<<e.what()<<'\n';return 1;}
}
