fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    membranes::order_cycle::run("squaring_cycle", &args, 200_000);
}
