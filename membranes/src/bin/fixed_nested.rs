// Fixed nested membrane: ensure all ∈ frames have matching ∋ fuses
// The issue: ∈⋈∈C{P}∋ pattern leaves ∈ at depth 2 unfused
// Fix: use ⊢∈⋈C{P}∋⋈C{P}∋⋈C{P}∋⊣ pattern where each C{P} is self-contained

fn main() { 
    // Each unit: ⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋ (carrier + payload + fuse)
    // Three units chained with ⋈, wrapped in ⊢∈...⊣
    membranes::main_membrane("fixed_nested", 
        "⊢∈⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⋈⊢≻⋈∈⊥∋⊢≻⊙∈⊤≺⊥⊞⋈∋⊙⊡∋⊣"); 
}
