//! Audit the emitted physical braid presentation throughout the register stack.
use g_momonados::braid_protocol::audit_braid_frames;
use num_bigint::BigUint;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n = BigUint::parse_bytes(args[1].as_bytes(),10).unwrap();
    assert!(n.bits() >= 200);
    let data = std::fs::read_to_string(&args[2]).unwrap();
    let word: Vec<i32> = data.split_whitespace().map(|g|g.parse().unwrap()).collect();
    let heights: Vec<i32> = (1..=(g_momonados::reversible_modular::ModularMultiply::new(&n).unwrap().elementary_qubits()+1) as i32).collect();
    let reports = audit_braid_frames(&word,6,&heights).unwrap();
    assert_eq!(reports.len(),heights.len());
    for report in &reports {
        assert_eq!(report.returned_depth,report.height);
        assert_eq!(report.crossings,word.len());
    }
    println!("source_bits={} physical_generators={} audited_heights={} lowest_height={} highest_height={} transformed_frames_closed=true presentation_recovered=true",n.bits(),word.len(),reports.len(),reports.first().unwrap().height,reports.last().unwrap().height);
}
