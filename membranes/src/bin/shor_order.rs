fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    membranes::order_cycle::run("shor_order", &args, 500_000);
}
