/*
 * Baked Gödel-word phase extractor. This source is compiled to an ELF and
 * lifted by V⊙x into a self-contained executable IMASM module. All numeral
 * inputs enter through the encoded words below; no decimal operand is read
 * by the resulting membrane.
 */

#include <stddef.h>
#include <stdint.h>

#ifndef LIMBS
#error LIMBS must be derived from the baked modulus width
#endif
#ifndef MODULUS_BITS
#error MODULUS_BITS must be derived from the baked modulus width
#endif

#ifndef SINGLE_BASE
#define SINGLE_BASE 0
#endif

#ifndef SUPPORT_EVERY_PHASE
#define SUPPORT_EVERY_PHASE 0
#endif

static const char baked_n[] = "@@N_WORD@@";
static const char baked_base_word[] = "@@BASE_WORD@@";

typedef struct {
    uint64_t limb[LIMBS];
} Number;

typedef struct State State;
struct State {
    uint64_t hash;
    Number previous;
    Number current;
    State *next;
};

typedef struct {
    State **bucket;
    size_t capacity;
    size_t count;
} StateMap;

static Number modulus;
static Number mont_one;
static uint64_t n0_inverse;

static long syscall6(long number, long a0, long a1, long a2,
                     long a3, long a4, long a5) {
    register long r10 __asm__("r10") = a3;
    register long r8 __asm__("r8") = a4;
    register long r9 __asm__("r9") = a5;
    register long rax __asm__("rax") = number;
    __asm__ volatile("syscall"
                     : "+a"(rax)
                     : "D"(a0), "S"(a1), "d"(a2), "r"(r10), "r"(r8), "r"(r9)
                     : "rcx", "r11", "memory");
    return rax;
}

static void *map_memory(size_t bytes) {
    long result = syscall6(9, 0, (long)bytes, 3, 0x22, -1, 0);
    return result < 0 ? (void *)0 : (void *)result;
}

static void byte_copy(void *to, const void *from, size_t bytes) {
    unsigned char *d = (unsigned char *)to;
    const unsigned char *s = (const unsigned char *)from;
    for (size_t i = 0; i < bytes; ++i) d[i] = s[i];
}

static void byte_zero(void *to, size_t bytes) {
    unsigned char *d = (unsigned char *)to;
    for (size_t i = 0; i < bytes; ++i) d[i] = 0;
}

static size_t utf8_cell(const char *at) {
    static const unsigned char bot[3] = {0xe2, 0x8a, 0xa5};
    static const unsigned char top[3] = {0xe2, 0x8a, 0xa4};
    const unsigned char *p = (const unsigned char *)at;
    if (p[0] == bot[0] && p[1] == bot[1] && p[2] == bot[2]) return 1;
    if (p[0] == top[0] && p[1] == top[1] && p[2] == top[2]) return 0;
    return 2;
}

static int decode_godel_word(const char *word, Number *out) {
    byte_zero(out, sizeof(*out));
    size_t bit = 0;
    for (size_t i = 0; word[i] != 0; ++i) {
        if ((unsigned char)word[i] != 0xe2) continue;
        size_t value = utf8_cell(word + i);
        if (value == 2) continue;
        if (bit >= LIMBS * 64u) return 0;
        if (value != 0) out->limb[bit / 64u] |= (uint64_t)1u << (bit % 64u);
        ++bit;
        i += 2;
    }
    return bit != 0;
}

static int compare(const Number *a, const Number *b) {
    for (size_t i = LIMBS; i != 0; --i) {
        uint64_t x = a->limb[i - 1];
        uint64_t y = b->limb[i - 1];
        if (x < y) return -1;
        if (x > y) return 1;
    }
    return 0;
}

static int is_zero(const Number *a) {
    uint64_t bits = 0;
    for (size_t i = 0; i < LIMBS; ++i) bits |= a->limb[i];
    return bits == 0;
}

static int is_one(const Number *a) {
    if (a->limb[0] != 1) return 0;
    for (size_t i = 1; i < LIMBS; ++i) if (a->limb[i] != 0) return 0;
    return 1;
}

static int equal(const Number *a, const Number *b) {
    return compare(a, b) == 0;
}

static uint32_t subtract(Number *out, const Number *a, const Number *b) {
    unsigned __int128 borrow = 0;
    for (size_t i = 0; i < LIMBS; ++i) {
        unsigned __int128 x = a->limb[i];
        unsigned __int128 y = (unsigned __int128)b->limb[i] + borrow;
        out->limb[i] = (uint64_t)(x - y);
        borrow = x < y;
    }
    return (uint32_t)borrow;
}

static void shift_right(Number *a) {
    uint64_t carry = 0;
    for (size_t i = LIMBS; i != 0; --i) {
        uint64_t next = a->limb[i - 1] & 1u;
        a->limb[i - 1] = (a->limb[i - 1] >> 1) | (carry << 63);
        carry = next;
    }
}

static void shift_left_one(Number *a) {
    uint64_t carry = 0;
    for (size_t i = 0; i < LIMBS; ++i) {
        uint64_t next = a->limb[i] >> 63;
        a->limb[i] = (a->limb[i] << 1) | carry;
        carry = next;
    }
}

static void add_mod(Number *out, const Number *a, const Number *b) {
    unsigned __int128 carry = 0;
    for (size_t i = 0; i < LIMBS; ++i) {
        unsigned __int128 sum = (unsigned __int128)a->limb[i] + b->limb[i] + carry;
        out->limb[i] = (uint64_t)sum;
        carry = sum >> 64;
    }
    if (carry != 0 || compare(out, &modulus) >= 0) {
        Number reduced;
        (void)subtract(&reduced, out, &modulus);
        byte_copy(out, &reduced, sizeof(*out));
    }
}

static void montgomery_multiply(Number *out, const Number *a, const Number *b) {
    uint64_t product[2u * LIMBS + 2u];
    byte_zero(product, sizeof(product));
    for (size_t i = 0; i < LIMBS; ++i) {
        unsigned __int128 carry = 0;
        for (size_t j = 0; j < LIMBS; ++j) {
            unsigned __int128 value = (unsigned __int128)a->limb[i] * b->limb[j]
                                    + product[i + j] + carry;
            product[i + j] = (uint64_t)value;
            carry = value >> 64;
        }
        size_t k = i + LIMBS;
        while (carry != 0) {
            unsigned __int128 value = (unsigned __int128)product[k] + carry;
            product[k] = (uint64_t)value;
            carry = value >> 64;
            ++k;
        }
    }
    for (size_t i = 0; i < LIMBS; ++i) {
        uint64_t factor = product[i] * n0_inverse;
        unsigned __int128 carry = 0;
        for (size_t j = 0; j < LIMBS; ++j) {
            unsigned __int128 value = (unsigned __int128)factor * modulus.limb[j]
                                    + product[i + j] + carry;
            product[i + j] = (uint64_t)value;
            carry = value >> 64;
        }
        size_t k = i + LIMBS;
        while (carry != 0) {
            unsigned __int128 value = (unsigned __int128)product[k] + carry;
            product[k] = (uint64_t)value;
            carry = value >> 64;
            ++k;
        }
    }
    for (size_t i = 0; i < LIMBS; ++i) out->limb[i] = product[LIMBS + i];
    if (product[2u * LIMBS] != 0 || compare(out, &modulus) >= 0) {
        Number reduced;
        (void)subtract(&reduced, out, &modulus);
        byte_copy(out, &reduced, sizeof(*out));
    }
}

static void modular_double(Number *a) {
    Number copy;
    byte_copy(&copy, a, sizeof(copy));
    add_mod(a, &copy, &copy);
}

static void reduce_mod(Number *out, const Number *input) {
    byte_zero(out, sizeof(*out));
    Number one;
    byte_zero(&one, sizeof(one));
    one.limb[0] = 1u;
    for (size_t bit = LIMBS * 64u; bit != 0; --bit) {
        modular_double(out);
        size_t position = bit - 1u;
        if (((input->limb[position / 64u] >> (position % 64u)) & 1u) != 0) {
            Number sum;
            add_mod(&sum, out, &one);
            byte_copy(out, &sum, sizeof(*out));
        }
    }
}

static void to_montgomery(Number *out, const Number *input) {
    reduce_mod(out, input);
    for (size_t i = 0; i < LIMBS * 64u; ++i) modular_double(out);
}

static uint64_t hash_number(const Number *a) {
    uint64_t hash = 0xcbf29ce484222325ull;
    for (size_t i = 0; i < LIMBS; ++i) {
        uint64_t word = a->limb[i];
        for (unsigned byte = 0; byte < 8; ++byte) {
            hash ^= (word >> (byte * 8u)) & 255u;
            hash *= 0x100000001b3ull;
        }
    }
    return hash;
}

static State *new_state(const Number *current, const Number *previous,
                        uint64_t hash, State *next) {
    State *state = (State *)map_memory(sizeof(State));
    if (state == 0) return 0;
    state->hash = hash;
    byte_copy(&state->current, current, sizeof(*current));
    byte_copy(&state->previous, previous, sizeof(*previous));
    state->next = next;
    return state;
}

static int map_init(StateMap *map) {
    map->capacity = 1024;
    map->count = 0;
    map->bucket = (State **)map_memory(map->capacity * sizeof(State *));
    if (map->bucket == 0) return 0;
    byte_zero(map->bucket, map->capacity * sizeof(State *));
    return 1;
}

static State *map_find(StateMap *map, const Number *current, uint64_t hash) {
    size_t slot = (size_t)(hash % map->capacity);
    for (State *state = map->bucket[slot]; state != 0; state = state->next)
        if (state->hash == hash && equal(&state->current, current)) return state;
    return 0;
}

static int map_grow(StateMap *map) {
    size_t capacity = map->capacity * 2u;
    if (capacity < map->capacity) return 0;
    State **bucket = (State **)map_memory(capacity * sizeof(State *));
    if (bucket == 0) return 0;
    byte_zero(bucket, capacity * sizeof(State *));
    for (size_t i = 0; i < map->capacity; ++i) {
        State *state = map->bucket[i];
        while (state != 0) {
            State *next = state->next;
            size_t slot = (size_t)(state->hash % capacity);
            state->next = bucket[slot];
            bucket[slot] = state;
            state = next;
        }
    }
    map->bucket = bucket;
    map->capacity = capacity;
    return 1;
}

static int map_insert(StateMap *map, const Number *current,
                      const Number *previous, uint64_t hash) {
    if (map->count >= map->capacity * 2u && !map_grow(map)) return 0;
    size_t slot = (size_t)(hash % map->capacity);
    State *state = new_state(current, previous, hash, map->bucket[slot]);
    if (state == 0) return 0;
    map->bucket[slot] = state;
    ++map->count;
    return 1;
}

static void binary_gcd(Number *out, const Number *left, const Number *right) {
    Number a, b;
    byte_copy(&a, left, sizeof(a));
    byte_copy(&b, right, sizeof(b));
    if (is_zero(&a)) { byte_copy(out, &b, sizeof(*out)); return; }
    if (is_zero(&b)) { byte_copy(out, &a, sizeof(*out)); return; }
    size_t common = 0;
    while ((a.limb[0] & 1u) == 0 && (b.limb[0] & 1u) == 0) {
        shift_right(&a); shift_right(&b); ++common;
    }
    while ((a.limb[0] & 1u) == 0) shift_right(&a);
    do {
        while ((b.limb[0] & 1u) == 0) shift_right(&b);
        if (compare(&a, &b) > 0) {
            Number swap = a; a = b; b = swap;
        }
        Number difference;
        (void)subtract(&difference, &b, &a);
        byte_copy(&b, &difference, sizeof(b));
    } while (!is_zero(&b));
    byte_copy(out, &a, sizeof(*out));
    for (size_t i = 0; i < common; ++i) shift_left_one(out);
}

static void exact_complement(Number *quotient, const Number *factor) {
    Number residual = modulus;
    byte_zero(quotient, sizeof(*quotient));
    for (size_t bit = 0; bit < LIMBS * 64u; ++bit) {
        if ((residual.limb[0] & 1u) != 0) {
            Number difference;
            if (subtract(&difference, &residual, factor) != 0) {
                byte_zero(quotient, sizeof(*quotient));
                return;
            }
            byte_copy(&residual, &difference, sizeof(residual));
            quotient->limb[bit / 64u] |= (uint64_t)1u << (bit % 64u);
        }
        shift_right(&residual);
    }
    if (!is_zero(&residual)) byte_zero(quotient, sizeof(*quotient));
}

static void emit_bytes(const char *bytes, size_t length) {
    (void)syscall6(1, 1, (long)bytes, (long)length, 0, 0, 0);
}

static uint64_t clock_nanoseconds(void) {
    struct {
        int64_t seconds;
        int64_t nanoseconds;
    } reading;
    if (syscall6(228, 1, (long)&reading, 0, 0, 0, 0) != 0) return 0;
    return (uint64_t)reading.seconds * 1000000000ull + (uint64_t)reading.nanoseconds;
}

static size_t encode_word(const Number *number, char *output) {
    static const char prefix[] = "⊢";
    static const char cell_prefix[] = "≻⋈∈";
    static const char zero[] = "⊤";
    static const char one[] = "⊥";
    static const char cell_suffix[] = "∋";
    static const char suffix[] = "⊙⊡⊣";
    size_t bit_length = LIMBS * 64u;
    while (bit_length > 1u) {
        size_t bit = bit_length - 1u;
        if (((number->limb[bit / 64u] >> (bit % 64u)) & 1u) != 0) break;
        --bit_length;
    }
    size_t at = 0;
    for (size_t i = 0; prefix[i] != 0; ++i) output[at++] = prefix[i];
    for (size_t bit = 0; bit < bit_length; ++bit) {
        for (size_t i = 0; cell_prefix[i] != 0; ++i) output[at++] = cell_prefix[i];
        const char *symbol = ((number->limb[bit / 64u] >> (bit % 64u)) & 1u) ? one : zero;
        for (size_t i = 0; symbol[i] != 0; ++i) output[at++] = symbol[i];
        for (size_t i = 0; cell_suffix[i] != 0; ++i) output[at++] = cell_suffix[i];
    }
    for (size_t i = 0; suffix[i] != 0; ++i) output[at++] = suffix[i];
    return at;
}

static void report_pair(const Number *p, const Number *q, uint64_t started) {
    size_t capacity = (size_t)LIMBS * 64u * 18u + 128u;
    char *buffer = (char *)map_memory(capacity);
    if (buffer == 0) return;
    static const char p_label[] = "P = ";
    static const char q_label[] = "\nQ = ";
    static const char closure[] = "\nclosure = ⊤\n";
    static const char elapsed_label[] = "elapsed_ns=";
    size_t at = 0;
    for (size_t i = 0; p_label[i] != 0; ++i) buffer[at++] = p_label[i];
    at += encode_word(p, buffer + at);
    for (size_t i = 0; q_label[i] != 0; ++i) buffer[at++] = q_label[i];
    at += encode_word(q, buffer + at);
    for (size_t i = 0; closure[i] != 0; ++i) buffer[at++] = closure[i];
    for (size_t i = 0; elapsed_label[i] != 0; ++i) buffer[at++] = elapsed_label[i];
    uint64_t elapsed = clock_nanoseconds() - started;
    char digits[24];
    size_t digit_count = 0;
    do {
        digits[digit_count++] = (char)('0' + elapsed % 10u);
        elapsed /= 10u;
    } while (elapsed != 0);
    while (digit_count != 0) buffer[at++] = digits[--digit_count];
    buffer[at++] = '\n';
    emit_bytes(buffer, at);
}

static int support_candidate(Number *candidate, const Number *x) {
    Number powers[8];
    byte_copy(&powers[0], &mont_one, sizeof(Number));
    for (size_t i = 1; i < 8u; ++i)
        montgomery_multiply(&powers[i], &powers[i - 1u], x);
    Number x8;
    montgomery_multiply(&x8, &powers[7], x);
    Number accumulator;
    byte_zero(&accumulator, sizeof(accumulator));
    size_t bits = MODULUS_BITS;
    size_t frames = (bits + 7u) / 8u;
    for (size_t frame = frames; frame != 0; --frame) {
        Number product;
        montgomery_multiply(&product, &accumulator, &x8);
        byte_copy(&accumulator, &product, sizeof(accumulator));
        size_t low = (frame - 1u) * 8u;
        for (size_t j = 0; j < 8u && low + j < bits; ++j) {
            if (((modulus.limb[(low + j) / 64u] >> ((low + j) % 64u)) & 1u) == 0) continue;
            Number sum;
            add_mod(&sum, &accumulator, &powers[j]);
            byte_copy(&accumulator, &sum, sizeof(accumulator));
        }
    }
    binary_gcd(candidate, &accumulator, &modulus);
    return !is_one(candidate) && !is_zero(candidate) && !equal(candidate, &modulus);
}

static int is_power_of_two(const Number *number) {
    size_t set_bits = 0;
    for (size_t i = 0; i < LIMBS; ++i) {
        uint64_t limb = number->limb[i];
        while (limb != 0) {
            limb &= limb - 1u;
            if (++set_bits > 1u) return 0;
        }
    }
    return set_bits == 1u;
}

static void increment(Number *number) {
    for (size_t i = 0; i < LIMBS; ++i) {
        if (++number->limb[i] != 0) return;
    }
}

static int support_due(const Number *phase, const Number *base, const Number *two) {
#if SUPPORT_EVERY_PHASE
    (void)is_power_of_two(phase);
    (void)base; (void)two;
    return 1;
#else
    Number zero, eight;
    byte_zero(&zero, sizeof(zero));
    byte_zero(&eight, sizeof(eight));
    eight.limb[0] = 8u;
    if (is_zero(phase) && !equal(base, two)) return 1;
    return compare(phase, &eight) > 0 && is_power_of_two(phase);
#endif
}

static void add_one_mod(Number *number) {
    Number one;
    byte_zero(&one, sizeof(one));
    one.limb[0] = 1;
    Number sum;
    add_mod(&sum, number, &one);
    byte_copy(number, &sum, sizeof(*number));
}

int main(void) {
    uint64_t started = clock_nanoseconds();
    Number baked_base;
    if (!decode_godel_word(baked_n, &modulus)
        || !decode_godel_word(baked_base_word, &baked_base)) {
        static const char message[] = "encoded input decode failed\n";
        emit_bytes(message, sizeof(message) - 1u);
        return 2;
    }
    if ((modulus.limb[0] & 1u) == 0 || is_zero(&modulus) || is_one(&modulus)) {
        static const char message[] = "modulus must exceed one and be odd\n";
        emit_bytes(message, sizeof(message) - 1u);
        return 2;
    }
    uint64_t inverse = 1u;
    for (unsigned i = 0; i < 6u; ++i)
        inverse *= 2u - modulus.limb[0] * inverse;
    n0_inverse = 0u - inverse;
    Number one;
    byte_zero(&one, sizeof(one));
    one.limb[0] = 1u;
    to_montgomery(&mont_one, &one);
    Number two = one;
    add_one_mod(&two);
    Number base = baked_base;
    for (;;) {
        Number current, previous;
        to_montgomery(&current, &base);
        byte_copy(&previous, &mont_one, sizeof(previous));
        StateMap seen;
        if (!map_init(&seen) || !map_insert(&seen, &mont_one, &mont_one,
                                             hash_number(&mont_one))) {
            static const char message[] = "phase-state allocation failed\n";
            emit_bytes(message, sizeof(message) - 1u);
            return 2;
        }
        Number phase;
        byte_zero(&phase, sizeof(phase));
        for (;;) {
            if (support_due(&phase, &base, &two)) {
                Number candidate, complement;
                if (support_candidate(&candidate, &current)) {
                    exact_complement(&complement, &candidate);
                    if (!is_zero(&complement)) {
                        report_pair(&candidate, &complement, started);
                        return 0;
                    }
                }
            }
            uint64_t hash = hash_number(&current);
            State *prior = map_find(&seen, &current, hash);
            if (prior != 0) {
                Number delta, candidate, complement;
                if (compare(&previous, &prior->previous) >= 0)
                    (void)subtract(&delta, &previous, &prior->previous);
                else
                    (void)subtract(&delta, &prior->previous, &previous);
                binary_gcd(&candidate, &delta, &modulus);
                if (!is_one(&candidate) && !is_zero(&candidate)
                    && !equal(&candidate, &modulus)) {
                    exact_complement(&complement, &candidate);
                    if (!is_zero(&complement)) {
                        report_pair(&candidate, &complement, started);
                        return 0;
                    }
                }
#if SINGLE_BASE
                static const char message[] = "isolated orbit closed without pair\n";
                emit_bytes(message, sizeof(message) - 1u);
                return 1;
#else
                add_one_mod(&base);
                if (compare(&base, &two) < 0) byte_copy(&base, &two, sizeof(base));
                break;
#endif
            }
            if (!map_insert(&seen, &current, &previous, hash)) {
                static const char message[] = "phase-state allocation failed\n";
                emit_bytes(message, sizeof(message) - 1u);
                return 2;
            }
            byte_copy(&previous, &current, sizeof(previous));
            montgomery_multiply(&current, &previous, &previous);
            increment(&phase);
        }
    }
}

__asm__(".global _start\n"
        "_start:\n"
        "xor %ebp, %ebp\n"
        "and $-16, %rsp\n"
        "call main\n"
        "mov %eax, %edi\n"
        "mov $60, %eax\n"
        "syscall\n");
