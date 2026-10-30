//! Host-side controller. The circuit's prepare and activation functions execute
//! in the resident IMASM machine; this controller only supplies gates, measures,
//! reads signals and checks them against a direct transform.
use std::time::Instant;
use vox::imasm_vm::{Machine, Stop};

fn stop(error: Stop) -> String {
    match error { Stop::Halt(s) => s, Stop::SysExit(n) => format!("unexpected process exit {n}") }
}
fn invoke(machine: &mut Machine, address: u64, args: &[i64]) -> Result<(), String> {
    let code = machine.call(address, args, 10_000_000).map_err(stop)?;
    if code == 0 { Ok(()) } else { Err(format!("circuit function returned {code}")) }
}
fn info(machine: &mut Machine, address: u64, field: i64) -> Result<u64, String> {
    machine.call(address, &[field], 100_000).map_err(stop)?;
    Ok(machine.reg("rax") as u64)
}
fn direct(input: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let ports = input.len();
    (0..ports).map(|k| {
        let mut sum = (0.0, 0.0);
        for (j, &(re, im)) in input.iter().enumerate() {
            let angle = 2.0*core::f64::consts::PI*(j*k) as f64/ports as f64;
            sum.0 += re*angle.cos()-im*angle.sin();
            sum.1 += re*angle.sin()+im*angle.cos();
        }
        (sum.0/(ports as f64).sqrt(), sum.1/(ports as f64).sqrt())
    }).collect()
}
fn read_output(machine: &Machine, address: u64, ports: usize) -> Vec<(f64, f64)> {
    let bytes = machine.peek(address, (ports*16) as u64);
    (0..ports).map(|k| (
        f64::from_le_bytes(bytes[k*16..k*16+8].try_into().unwrap()),
        f64::from_le_bytes(bytes[k*16+8..k*16+16].try_into().unwrap()))).collect()
}
fn check(got: &[(f64,f64)], expected: &[(f64,f64)]) -> Result<f64, String> {
    if got.len() != expected.len() { return Err("output port count differs from the control".into()); }
    let mut maximum: f64 = 0.0;
    for (k, (&a, &b)) in got.iter().zip(expected).enumerate() {
        let error = (a.0-b.0).abs().max((a.1-b.1).abs());
        if !a.0.is_finite() || !a.1.is_finite() || error > 1e-10 {
            return Err(format!("port {k}: got {a:?}, direct transform {b:?}"));
        }
        maximum = maximum.max(error);
    }
    Ok(maximum)
}

fn parse_gate(case: &str) -> Result<(u64, bool), String> {
    let (mask, feedback) = case.trim().split_once(':').unwrap_or((case.trim(), "0"));
    let gates = u64::from_str_radix(mask.trim_start_matches("0x"), 16).map_err(|_| format!("invalid gate mask {mask}"))?;
    let feedback = match feedback { "0" => false, "1" => true, _ => return Err("feedback must be 0 or 1".into()) };
    Ok((gates, feedback))
}

pub fn run(args: &[String]) -> Result<(), String> {
    let file = args.first().ok_or("vox circuit <module.imasm> [--stdin | hex-mask[:feedback-0-or-1] ...]")?;
    let streaming = args.get(1).map(String::as_str) == Some("--stdin");
    if streaming && args.len() != 2 { return Err("--stdin takes gate changes one per line, ending at EOF".into()); }
    let mut cases = Vec::new();
    if !streaming {
        for case in &args[1..] { cases.push(parse_gate(case)?); }
    }
    if cases.is_empty() && !streaming {
        cases.extend([(0, false), (1, false), (3, false), (0x0123456789abcdef, false),
            (u64::MAX, true), (0x5555555555555555, true), (0, false), (1 << 63, false), (1, false)]);
    }
    let start = Instant::now();
    let module = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let module = if file.ends_with(".glyphs") || module.starts_with(vox::glyph_module::PREFIX) {
        vox::glyph_module::decode(&module)?
    } else { module };
    let mut machine = Machine::new(&module);
    let resolve = |name: &str| machine.resolve(name).ok_or_else(|| format!("missing circuit ABI symbol {name}"));
    let prepare = resolve("membrane_prepare")?;
    let activate = resolve("membrane_activate")?;
    let metadata = resolve("membrane_info")?;
    let ports = info(&mut machine, metadata, 0)? as usize;
    if !ports.is_power_of_two() || ports > 4096 { return Err("direct-transform control supports power-of-two port counts through 4096".into()); }
    let output = info(&mut machine, metadata, 1)?;
    let loaded = start.elapsed();
    let start = Instant::now();
    invoke(&mut machine, prepare, &[])?;
    println!("ports={ports} levels={} load={loaded:?} prepare={:?} prepare_steps={}", ports.trailing_zeros(), start.elapsed(), machine.steps);
    let mut previous = vec![(0.0, 0.0); ports];
    let mut times = Vec::new();
    let inputs: Box<dyn Iterator<Item=Result<(u64,bool),String>>> = if streaming {
        Box::new(std::io::stdin().lines().map(|line| line.map_err(|e| e.to_string()).and_then(|s| parse_gate(&s))))
    } else { Box::new(cases.into_iter().map(Ok)) };
    for (index, case) in inputs.enumerate() {
        let (gates, feedback) = case?;
        let input: Vec<_> = (0..ports).map(|j| if gates >> (j % 64) & 1 == 0 { (0.0, 0.0) }
            else if feedback { previous[j] } else { (1.0, 0.0) }).collect();
        let expected = direct(&input);
        let start = Instant::now();
        invoke(&mut machine, activate, &[gates as i64, feedback as i64])?;
        let elapsed = start.elapsed();
        times.push(elapsed);
        let steps = machine.steps;
        let got = read_output(&machine, output, ports);
        let error = check(&got, &expected)?;
        if info(&mut machine, metadata, 2)? != 1 || info(&mut machine, metadata, 3)? != index as u64+1 {
            return Err("resident preparation/activation counters changed unexpectedly".into());
        }
        println!("gates={gates:016x} feedback={} activate={elapsed:?} steps={steps} max_error={error:.3e}", feedback as u8);
        previous = expected;
    }
    times.sort();
    if let Some(median) = times.get(times.len()/2) {
        println!("resident: preparations=1 activations={} median_activation={median:?}", times.len());
    } else { println!("resident: preparations=1 activations=0"); }
    if streaming { return Ok(()); }
    // Same module, same feedforward gates: measure one cold construction and
    // activation beside repeated activation on the already prepared machine.
    let gates = 0x0123456789abcdefu64;
    let input: Vec<_> = (0..ports).map(|j| if gates >> (j % 64) & 1 == 0 {(0.0,0.0)} else {(1.0,0.0)}).collect();
    let expected = direct(&input);
    let mut cold_times = Vec::new();
    let mut warm_times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let mut cold = Machine::new(&module);
        invoke(&mut cold, prepare, &[])?;
        invoke(&mut cold, activate, &[gates as i64, 0])?;
        cold_times.push(start.elapsed());
        check(&read_output(&cold, output, ports), &expected)?;
        let start = Instant::now();
        invoke(&mut machine, activate, &[gates as i64, 0])?;
        warm_times.push(start.elapsed());
        check(&read_output(&machine, output, ports), &expected)?;
    }
    cold_times.sort(); warm_times.sort();
    println!("same_gates_control: cold_parse_prepare_activate={:?} resident_activate={:?} ratio={:.2}x samples=5",
        cold_times[2], warm_times[2], cold_times[2].as_secs_f64()/warm_times[2].as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gate_syntax_and_controls_reject_invalid_readings() {
        assert_eq!(parse_gate("0xffffffffffffffff:1"), Ok((u64::MAX, true)));
        assert_eq!(parse_gate("0"), Ok((0, false)));
        for bad in ["", "xyz", "1:2", "10000000000000000", "1:1:1"] { assert!(parse_gate(bad).is_err()); }
        assert!(check(&[(f64::NAN, 0.0)], &[(0.0, 0.0)]).is_err());
        assert!(check(&[(1.0, 0.0)], &[(0.0, 0.0)]).is_err());
        assert!(check(&[], &[(0.0, 0.0)]).is_err());
        assert_eq!(check(&[(1.0, 0.0)], &[(1.0, 0.0)]), Ok(0.0));
    }
}
