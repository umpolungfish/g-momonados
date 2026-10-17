#define main baked_membrane_main
#include PREPARED_CPU_SOURCE
#undef main

[[maybe_unused]] static void parity_mulmod(const Word &a, const Word &b, const Word &n, Word &out) {
    Word accumulator;
    int top = W - 1;
    while (top > 0 && digit(b, top) == T) --top;
    for (int i = top; i >= 0; --i) {
        addmod(accumulator, accumulator, n, accumulator);
        if (digit(b, i) == F) addmod(accumulator, a, n, accumulator);
    }
    out = accumulator;
}

int main() {
#ifndef TRANSITION_ONLY
    Word n; load_word(SOURCE_ARMS, n);
    std::uint64_t random_state = 0x87c031992ed04badULL;
    auto random_word = [&]() {
        Word word;
        for (int i = 0; i < W - 10; ++i) {
            random_state ^= random_state << 13;
            random_state ^= random_state >> 7;
            random_state ^= random_state << 17;
            word.v[i] = random_state & 1 ? F : T;
        }
        if (compare(word, n) >= 0) subtract(word, n, word);
        return word;
    };
    std::vector<Word> left, right;
    for (int i = 0; i < 128; ++i) {
        left.push_back(random_word()); right.push_back(random_word());
        if (i == 0) clear(left.back());
        if (i == 1) small(1, left.back());
        if (i == 2) { Word unit; small(1, unit); subtract(n, unit, left.back()); right.back() = left.back(); }
        Word expected, actual;
        parity_mulmod(left.back(), right.back(), n, expected);
        regular_mulmod(left.back(), right.back(), n, actual);
        if (compare(expected, actual)) return 1;
        actual = left.back(); regular_mulmod(actual, right.back(), n, actual);
        if (compare(expected, actual)) return 2;
        actual = right.back(); regular_mulmod(left.back(), actual, n, actual);
        if (compare(expected, actual)) return 3;
        parity_mulmod(left.back(), left.back(), n, expected);
        actual = left.back(); regular_mulmod(actual, actual, n, actual);
        if (compare(expected, actual)) return 4;
    }
    auto measure = [&](auto multiply) {
        const auto start = std::chrono::steady_clock::now();
        Word output;
        for (unsigned i = 0; i < left.size(); ++i) multiply(left[i], right[i], n, output);
        std::cout << "terminal=" << numeral(output.v) << '\n';
        return std::chrono::duration<double>(std::chrono::steady_clock::now()-start).count();
    };
    const auto before = measure(parity_mulmod);
    const auto after = measure(regular_mulmod);
    std::cout << "source_bits=" << W-10 << " exact_cases=512 parity_seconds=" << before
        << " packed_seconds=" << after << " multiplication_speedup=" << before/after << '\n';
#endif
    std::vector<arm> state(64 * 14 * W, T);
    std::vector<int> continuation(64 * 6146, 0);
    std::vector<unsigned long long> receipt(10 + 64 * 10, 0);
    std::copy(std::begin(PREPARED_STATE), std::end(PREPARED_STATE), state.begin());
    std::copy(std::begin(PREPARED_CONTINUATION), std::end(PREPARED_CONTINUATION), continuation.begin());
    std::copy(std::begin(PREPARED_RECEIPT), std::end(PREPARED_RECEIPT), receipt.begin());
    const auto transition_start = std::chrono::steady_clock::now();
    idempotent_membrane(state.data(), continuation.data(), receipt.data(), 1, 0);
    const auto transition_seconds = std::chrono::duration<double>(std::chrono::steady_clock::now()-transition_start).count();
    auto hash = [](const auto &bank) {
        std::uint64_t value = 14695981039346656037ULL;
        for (auto cell : bank) { value ^= std::uint64_t(cell); value *= 1099511628211ULL; }
        return value;
    };
    std::cout << "transition_state=" << hash(state) << " continuation=" << hash(continuation)
        << " receipt=" << hash(receipt) << " transition_seconds=" << transition_seconds << '\n';
}
