// Proper nesting per task spec:
// Payload P = ≻⊙∈⊤≺⊥⊞⋈∋⊙⊡ (frobenius_braider core without endpoints)
// Carrier C = ≻⋈∈⊥∋ (numeral fragment)
// Pattern: ⊢∈⋈∈C{⊢∈P∋⊣} ∋⋈∈C{⊢∈P∋⊣} ∋⋈∈C{⊢∈P∋⊣} ∋⋈∋⊙⊡⊣
// Collapsed: ⊢∈⋈∈C⊢∈P∋⋈∈C⊢∈P∋⋈∈C⊢∈P∋⋈∋⊙⊡⊣

fn main() { 
    // C = ≻⋈∈⊥∋, P = ≻⊙∈⊤≺⊥⊞⋈∋⊙⊡
    // ⊢∈⋈∈≻⋈∈⊥∋⊢∈≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈∈≻⋈∈⊥∋⊢∈≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈∈≻⋈∈⊥∋⊢∈≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈∋⊙⊡⊣
    membranes::main_membrane("proper_nested", 
        "⊢∈⋈∈≻⋈∈⊥∋⊢∈≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈∈≻⋈∈⊥∋⊢∈≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈∈≻⋈∈⊥∋⊢∈≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈∋⊙⊡⊣"); 
}
