// Test minimal nesting: just the carrier pattern without complex payload
// Pattern: ⊢∈⋈C⋈C⋈C⋈∋⊙⊡⊣ where C = ⊢≻⋈∈⊥∋ (numeral fragment)

fn main() { 
    membranes::main_membrane("minimal_nested", 
        "⊢∈⋈⊢≻⋈∈⊥∋⋈⊢≻⋈∈⊥∋⋈⊢≻⋈∈⊥∋⋈∋⊙⊡⊣"); 
}
