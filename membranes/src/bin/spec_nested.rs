// Test the exact nesting pattern from spec:
// Payload P = ⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣ (full frobenius_braider - healthy, self-contained)
// Carrier C = ≻⋈∈⊥∋ (numeral fragment)
// Pattern: ⊢∈⋈∈C{P}∋⋈∈C{P}∋⋈∈C{P}∋⋈∋⊙⊡⊣
// Collapsed: ⊢∈⋈∈CP∋⋈∈CP∋⋈∈CP∋⋈∋⊙⊡⊣

fn main() { 
    // C = ≻⋈∈⊥∋, P = ⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣
    // ⊢∈⋈∈≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣∋⋈∈≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣∋⋈∈≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣∋⋈∋⊙⊡⊣
    membranes::main_membrane("spec_nested", 
        "⊢∈⋈∈≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣∋⋈∈≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣∋⋈∈≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡⊣∋⋈∋⊙⊡⊣"); 
}
