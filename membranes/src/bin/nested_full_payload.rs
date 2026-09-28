// Proper nested membrane: payload P = ≻⊙∈⊤≺⊥⊞⋈∋⊙⊡ (healthy frobenius core)
// Carrier C = ⊢≻⋈∈⊥∋ (numeral fragment that ends in F)
// Nesting pattern: ⊢∈⋈C{P}⋈C{P}⋈C{P}⋈∋⊙⊡⊣
// The key: P must be the FULL healthy word, not a fragment

fn main() { 
    // P = ⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣ (full frobenius_braider)
    // C = ⊢≻⋈∈⊥∋ (numeral carrier)
    // Nested: ⊢∈⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣⋈∋⊙⊡⊣
    membranes::main_membrane("nested_full_payload", 
        "⊢∈⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣⋈∋⊙⊡⊣"); 
}
