// Fixed-word membrane with one factor vessel nested at each IMSCRIB site.
// Each added level contributes one FSPLIT and one FFUSE, keeping pair
// production flat while the operation descends through the previous vessel.

const FACTOR_PREFIX: &str = "⊢∈≻⊤≺⊥⋈";
const FACTOR_SUFFIX: &str = "⊞∋⊡⋈⊙⊣";

fn enfold(depth: usize) -> String {
    let mut word = format!("{FACTOR_PREFIX}⊙{FACTOR_SUFFIX}");
    for _ in 1..depth {
        word = format!("{FACTOR_PREFIX}{word}{FACTOR_SUFFIX}");
    }
    word
}

fn main() {
    membranes::main_membrane("fixed_nested", &enfold(3));
}
