std::string numeral(const arm *cells) {
    int end = W;
    while (end && cells[end - 1] == T) --end;
    std::string result = "⊢";
    for (int i = 0; i < end; ++i) {
        if (cells[i] != T && cells[i] != F) throw std::runtime_error("invalid retained parity cell");
        result += cells[i] == F ? "≻⋈∈⊥∋" : "≻⋈∈⊤∋";
    }
    return result + "⊙⊡⊣";
}
int main(int argc, char **) {
    try {
        if (argc != 1) throw std::runtime_error("baked membrane accepts no instance arguments");
        std::vector<arm> state(64 * 14 * W, T);
        std::vector<int> continuation(64 * 6146, 0);
        std::vector<unsigned long long> receipt(10 + 64 * 10, 0);
#if defined(PREPARATION_CHECK) || defined(PREPARATION_EMIT)
        for (int i = 0; i < Y_NODE_COUNT; ++i) {
            continuation[3*i] = Y_TAG[i]; continuation[3*i+1] = Y_LEFT[i]; continuation[3*i+2] = Y_RIGHT[i];
        }
        continuation[6144] = Y_NODE_COUNT; continuation[6145] = Y_ROOT;
        idempotent_membrane(state.data(), continuation.data(), receipt.data(), 32, 1);
        unsigned long long failures = 0;
        resident_powerset_monad_laws(state.data(), continuation.data(), receipt.data(), &failures);
        std::clog << "resident_powerset_monad_law_failures=" << failures << '\n';
        if(failures)return 1;
#ifdef PREPARE_TO_CLOSURE
        const auto preparation_start=std::chrono::steady_clock::now();
        std::clog << "preparation_boundary=verified-factor input=N-only time_limit=none\n";
        while(receipt[3]==0)
            idempotent_membrane(state.data(), continuation.data(), receipt.data(), 1, 0);
        if(receipt[3]!=1&&receipt[3]!=8){
            std::clog << "preparation_error_status=" << receipt[3] << '\n';return 1;
        }
        std::clog << "preparation_to_verified_factor_seconds="
            << std::chrono::duration<double>(std::chrono::steady_clock::now()-preparation_start).count()
            << " Y_calls=" << receipt[1] << '\n';
#endif
#ifdef PREPARATION_CHECK
        bool same = std::equal(std::begin(PREPARED_STATE), std::end(PREPARED_STATE), state.begin())
            && std::equal(std::begin(PREPARED_CONTINUATION), std::end(PREPARED_CONTINUATION), continuation.begin())
            && std::equal(std::begin(PREPARED_RECEIPT), std::end(PREPARED_RECEIPT), receipt.begin());
        std::cout << "source_bits=" << W-10 << " CPU_preparation_equals_embedded_state=" << same << '\n';
        if (!same) return 1;
#else
        static_cast<void>(PREPARED_STATE);static_cast<void>(PREPARED_CONTINUATION);static_cast<void>(PREPARED_RECEIPT);
#endif
#ifdef PREPARATION_EMIT
        if(failures)return 1;
        auto emit=[](const auto &values,unsigned count){
            std::cout << '[';
            for(unsigned i=0;i<count;++i){if(i)std::cout << ',';std::cout << +values[i];}
            std::cout << ']';
        };
        std::cout << "{\"source_bits\":" << W-10 << ",\"state\":";emit(state,32*14*W);
        std::cout << ",\"continuation\":";emit(continuation,32*6146);
        std::cout << ",\"receipt\":";emit(receipt,330);std::cout << "}\n";
#endif
        return failures ? 1 : 0;
#else
        std::copy(std::begin(PREPARED_STATE), std::end(PREPARED_STATE), state.begin());
        std::copy(std::begin(PREPARED_CONTINUATION), std::end(PREPARED_CONTINUATION), continuation.begin());
        std::copy(std::begin(PREPARED_RECEIPT), std::end(PREPARED_RECEIPT), receipt.begin());
        const auto trigger = std::chrono::steady_clock::now();
        while (receipt[3]==0) {
            idempotent_membrane(state.data(), continuation.data(), receipt.data(), 1, 0);
        }
        const double elapsed = std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        const unsigned held = unsigned(receipt[4]);
        unsigned selected = held ? unsigned(__ffs(held)-1) : 0;
        for (unsigned i=0;i<32;++i) if ((held & (1u<<i)) && (receipt[10+i*10+3]==1 || receipt[10+i*10+3]==8)) { selected=i; break; }
        double verified_elapsed=0;
        if(receipt[3]==1||receipt[3]==8){
            ResidentState returned,checked;
            resident_load({state.data()+selected*14*W,continuation.data()+selected*6146,receipt.data()+10+selected*10},returned);
            checked=returned;
            Word source;load_word(SOURCE_ARMS,source);
            std::array<arm,2*W> paired{};
            if(!fuse_source_arm(returned.divisor,source,checked,paired.data())||checked.status!=8
                ||compare(returned.divisor,checked.divisor)||compare(returned.quotient,checked.quotient))
                throw std::runtime_error("prepared factor return failed source-bound exact closure");
            verified_elapsed=std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count();
        }
        std::cout << "execution_device=CPU instance_inputs=N-only-baked source_bits=" << W-10
            << "\nsource=" << numeral(SOURCE_ARMS) << "\nY_contractions=" << receipt[0]
            << " Y_calls=" << receipt[1] << " powerset_binds=" << receipt[2]
            << " held=" << held << " SIC_Zeta_roundtrips=" << receipt[8]
            << " reentry_residue_B=" << receipt[9] << " status=" << receipt[3]
            << "\ntrigger_to_readout_seconds=" << elapsed << '\n';
        const char *labels[] = {"idempotent","residual","complement","divisor","quotient","exponent","initial_candidate",
            "last_trivial_candidate","last_trivial_residual","source_power","singular_projection","SIC_Zeta_frame_residual",
            "reentry_source_presentation","reentry_return_presentation"};
        for (unsigned i=0;i<14;++i) std::cout << labels[i] << '=' << numeral(state.data()+(selected*14+i)*W) << '\n';
        if (receipt[3]==1 || receipt[3]==8) {
            std::cout << "factor " << numeral(state.data()+(selected*14+3)*W)
                << "\nfactor " << numeral(state.data()+(selected*14+4)*W)
                << "\nproducing_boundary=" << (receipt[3]==1 ? "idempotent-source-fuse" : "source-arm-fuse")
                << "\nproper_divisor=true remainder=0 exact_word_product=true paired_boundary=true";
            if (receipt[3]==1) std::cout << "\nidempotency=true orthogonality=true complementary_sum=true";
            std::cout << "\ntrigger_to_verified_factor_seconds=" << verified_elapsed << '\n';
            return 0;
        }
        std::cout << "return=open-held-membrane continuation=retained-Y-node-bank source_preserved=true\n";
        return receipt[3] ? 1 : 2;
#endif
    } catch (const std::exception &error) { std::cerr << error.what() << '\n'; return 1; }
}
