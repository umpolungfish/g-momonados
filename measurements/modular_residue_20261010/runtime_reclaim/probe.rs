// Exercise static musl allocation, growth, and release across live neighbors.
fn main() {
    let guard: Vec<u64> = (0..4097u64).map(|i| i ^ 0x51a7).collect();
    let guard_sum: u64 = guard.iter().sum();
    let mut result = 0u64;
    for round in 0..4u64 {
        let mut work = Vec::with_capacity(17);
        for i in 0..8193u64 { work.push(i * 3 + round); }
        let neighbor: Vec<u64> = (0..4097u64).map(|i| i + 11 * round).collect();
        result += work.iter().sum::<u64>();
        drop(work);
        let fresh = vec![round + 1; 8193];
        result += neighbor.iter().sum::<u64>() + fresh.iter().sum::<u64>();
        assert_eq!(guard.iter().sum::<u64>(), guard_sum);
        assert_eq!(neighbor[4096], 4096 + 11 * round);
    }
    println!("allocation_checksum={result}");
    println!("live_guard_checksum={guard_sum}");
}
