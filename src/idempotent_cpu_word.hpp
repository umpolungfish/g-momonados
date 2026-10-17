#include <algorithm>
#include <array>
#include <chrono>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
#define __constant__ const
#define __device__
#define __global__
#define __shared__
#define __noinline__ __attribute__((noinline))
#define __forceinline__ inline
#define T 8
#define F 9
#define S W
using arm = unsigned char;
struct CpuIndex { int x = 0; };
static CpuIndex blockIdx, threadIdx;
inline int lane() { return 0; }
inline void __syncwarp() {}
inline bool __all_sync(unsigned, bool value) { return value; }
inline int __ffs(unsigned value) { return __builtin_ffs(value); }
inline int __clz(unsigned value) { return value ? __builtin_clz(value) : 32; }
struct Word {
    arm v[W];
    Word() { std::fill(v, v + W, T); }
};
inline arm digit(const Word &a, int i) { return a.v[i]; }
inline void clear(Word &a) { std::fill(a.v, a.v + W, T); }
inline void copy(const Word &a, Word &b) { b = a; }
inline void small(unsigned long long value, Word &a) {
    for (int i = 0; i < W; ++i) a.v[i] = i < 64 && ((value >> (i & 63)) & 1) ? F : T;
}
inline bool zero(const Word &a) {
    return std::none_of(a.v, a.v + W, [](arm value) { return value == F; });
}
inline bool one(const Word &a) {
    return a.v[0] == F && std::none_of(a.v + 1, a.v + W, [](arm value) { return value == F; });
}
inline int compare(const Word &a, const Word &b) {
    for (int i = W - 1; i >= 0; --i) if (a.v[i] != b.v[i]) return a.v[i] == F ? 1 : -1;
    return 0;
}
inline void add(const Word &a, const Word &b, Word &out) {
    unsigned carry = 0;
    for (int i = 0; i < W; ++i) {
        unsigned sum = unsigned(a.v[i] == F) + unsigned(b.v[i] == F) + carry;
        out.v[i] = sum & 1 ? F : T;
        carry = sum >> 1;
    }
}
inline void subtract(const Word &a, const Word &b, Word &out) {
    int borrow = 0;
    for (int i = 0; i < W; ++i) {
        int difference = int(a.v[i] == F) - int(b.v[i] == F) - borrow;
        out.v[i] = difference & 1 ? F : T;
        borrow = difference < 0;
    }
}
inline void half(Word &a) {
    for (int i = 0; i + 1 < W; ++i) a.v[i] = a.v[i + 1];
    a.v[W - 1] = T;
}
inline void addmod(const Word &a, const Word &b, const Word &n, Word &out) {
    add(a, b, out);
    if (compare(out, n) >= 0) subtract(out, n, out);
}
inline void submod(const Word &a, const Word &b, const Word &n, Word &out) {
    if (compare(a, b) >= 0) subtract(a, b, out);
    else { Word difference; subtract(b, a, difference); subtract(n, difference, out); }
}
__noinline__ void regular_mulmod(const Word &a, const Word &b, const Word &n, Word &out) {
    // The parity banks remain authoritative. Pack only the arithmetic scratch
    // space, then restore every output cell before returning to the carrier.
    constexpr int limbs = (W + 63) / 64;
    using Packed = std::array<std::uint64_t, limbs>;
    Packed left{}, modulus{}, accumulator{};
    for (int i = 0; i < W; ++i) {
        left[i / 64] |= std::uint64_t(a.v[i] == F) << (i % 64);
        modulus[i / 64] |= std::uint64_t(n.v[i] == F) << (i % 64);
    }
    auto accumulate = [&](const Packed &value) {
        std::uint64_t carry = 0;
        for (int j = 0; j < limbs; ++j) {
            const auto old = accumulator[j];
            const auto sum = old + value[j];
            const auto next = sum + carry;
            carry = (sum < old) | (next < sum);
            accumulator[j] = next;
        }
        int relation = 0;
        for (int j = limbs - 1; j >= 0; --j) {
            if (accumulator[j] != modulus[j]) {
                relation = accumulator[j] > modulus[j] ? 1 : -1;
                break;
            }
        }
        if (carry || relation >= 0) {
            std::uint64_t borrow = 0;
            for (int j = 0; j < limbs; ++j) {
                const auto old = accumulator[j];
                const auto difference = old - modulus[j];
                const auto next = difference - borrow;
                borrow = (old < modulus[j]) | (difference < borrow);
                accumulator[j] = next;
            }
        }
    };
    int top = W - 1;
    while (top > 0 && digit(b, top) == T) --top;
    for (int i = top; i >= 0; --i) {
        accumulate(accumulator);
        if (digit(b, i) == F) accumulate(left);
    }
    for (int i = 0; i < W; ++i)
        out.v[i] = ((accumulator[i / 64] >> (i % 64)) & 1) ? F : T;
}
