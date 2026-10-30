fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    membranes::order_cycle::run("quantum_phase", &args, 500_000);
}
