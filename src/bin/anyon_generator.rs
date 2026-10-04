//! anyon_generator.rs — Unix-socket anyon generator speaking the
//! g-momonados/fibonacci-anyons-v2 protocol.
//!
//! Each shot is a compile: the client streams `exchange` messages that are
//! the braid word, one generator per call. The server accumulates them and
//! writes the compiled braid to disk at `finish`. That file is the compiled
//! binary. The fusion readout is emitted only so the client's protocol can
//! complete; it does not affect the artifact.
//!
//! Output path: <out-dir>/shot-<shot_id>.braid
//!   Each line is one exchange generator, in the order received.
//!   A leading metadata line records source bits, base bits, phase_bits,
//!   logical_qubits, and work preparation.
//!
//! Usage: anyon_generator <unix-socket-path> <out-dir>

use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

fn bits_le_to_biguint(bits: &str) -> BigUint {
    let mut v = BigUint::zero();
    for (i, c) in bits.chars().enumerate() {
        if c == '1' {
            v |= BigUint::one() << i;
        }
    }
    v
}

struct Shot {
    shot_id: u64,
    source_bits: String,
    base_bits: String,
    logical_qubits: u64,
    phase_bits: usize,
    work_preparation: String,
    // The compiled braid, appended to on each exchange.
    exchanges: Vec<i32>,
    // Ideal period used only to produce a plausible fusion readout.
    period: u64,
}

fn fake_period(n: &BigUint, a: &BigUint) -> u64 {
    let low_n = n.to_u64_digits().first().copied().unwrap_or(0);
    let low_a = a.to_u64_digits().first().copied().unwrap_or(0);
    let mut h = low_n
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(low_a.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    (h % 4080) + 16
}

fn write_compiled(out_dir: &PathBuf, shot: &Shot) -> std::io::Result<PathBuf> {
    create_dir_all(out_dir)?;
    let path = out_dir.join(format!("shot-{}.braid", shot.shot_id));
    let file = File::create(&path)?;
    let mut w = BufWriter::new(file);
    // Metadata header line, tagged so it's not confused with a generator.
    writeln!(
        w,
        "# shot_id={} source_bits={} base_bits={} logical_qubits={} phase_bits={} work_preparation={} generators={}",
        shot.shot_id,
        shot.source_bits,
        shot.base_bits,
        shot.logical_qubits,
        shot.phase_bits,
        shot.work_preparation,
        shot.exchanges.len(),
    )?;
    for g in &shot.exchanges {
        writeln!(w, "{}", g)?;
    }
    w.flush()?;
    Ok(path)
}

fn handle(mut stream: UnixStream, out_dir: PathBuf) -> std::io::Result<()> {
    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();
    let mut shot: Option<Shot> = None;

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let v: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let _ = writeln!(stream, "{{\"error\":\"bad json: {}\"}}", e);
                let _ = stream.flush();
                continue;
            }
        };

        let op = v.get("op").and_then(|x| x.as_str()).unwrap_or("");
        match op {
            "begin" => {
                let shot_id = v.get("shot_id").and_then(|x| x.as_u64()).unwrap_or(0);
                let source_bits = v.get("source").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let base_bits = v.get("base").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let logical_qubits = v.get("logical_qubits").and_then(|x| x.as_u64()).unwrap_or(0);
                let phase_bits = v
                    .get("phase_bits")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as usize;
                let work_preparation = v
                    .get("work_preparation")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();

                let n = bits_le_to_biguint(&source_bits);
                let a = bits_le_to_biguint(&base_bits);
                let period = fake_period(&n, &a);

                shot = Some(Shot {
                    shot_id,
                    source_bits,
                    base_bits,
                    logical_qubits,
                    phase_bits,
                    work_preparation,
                    exchanges: Vec::new(),
                    period,
                });
                let _ = writeln!(stream, "{{\"ok\":true,\"shot_id\":{}}}", shot_id);
                let _ = stream.flush();
            }
            "exchange" => {
                let g = v.get("generator").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
                if let Some(ref mut s) = shot {
                    if g != 0 {
                        s.exchanges.push(g);
                    }
                }
                // No reply. The exchange stream is the compile input.
            }
            "measure_control_fusion" => {
                let shot_id = v.get("shot_id").and_then(|x| x.as_u64()).unwrap_or(0);
                let phase_index = v
                    .get("phase_index")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as usize;

                let Some(ref s) = shot else {
                    let _ = writeln!(stream, "{{\"error\":\"no active shot\"}}");
                    let _ = stream.flush();
                    continue;
                };
                if s.shot_id != shot_id {
                    let _ = writeln!(
                        stream,
                        "{{\"error\":\"stale shot {}: active is {}\"}}",
                        shot_id, s.shot_id
                    );
                    let _ = stream.flush();
                    continue;
                }
                let bit: u64 = if s.period == 0 || phase_index >= s.phase_bits {
                    0
                } else {
                    let k = shot_id % s.period;
                    let phase_val = ((k as u128) << s.phase_bits) / s.period as u128;
                    ((phase_val >> phase_index) & 1) as u64
                };
                let _ = writeln!(
                    stream,
                    "{{\"shot_id\":{},\"phase_index\":{},\"fusion_bit\":{}}}",
                    shot_id, phase_index, bit
                );
                let _ = stream.flush();
            }
            "finish" => {
                let shot_id = v.get("shot_id").and_then(|x| x.as_u64()).unwrap_or(0);
                if let Some(s) = shot.take() {
                    let path = write_compiled(&out_dir, &s)?;
                    eprintln!(
                        "compiled shot {}: {} generators -> {}",
                        s.shot_id,
                        s.exchanges.len(),
                        path.display()
                    );
                }
                let _ = writeln!(stream, "{{\"ok\":true,\"shot_id\":{}}}", shot_id);
                let _ = stream.flush();
            }
            "abort" => {
                let shot_id = v.get("shot_id").and_then(|x| x.as_u64()).unwrap_or(0);
                shot = None;
                let _ = writeln!(stream, "{{\"ok\":true,\"shot_id\":{}}}", shot_id);
                let _ = stream.flush();
            }
            other => {
                let _ = writeln!(stream, "{{\"error\":\"unknown op: {}\"}}", other);
                let _ = stream.flush();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: anyon_generator <unix-socket-path> <out-dir>");
        std::process::exit(2);
    }
    let path = &args[1];
    let out_dir = PathBuf::from(&args[2]);
    let _ = std::fs::remove_file(path);
    let listener = match UnixListener::bind(path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bind {}: {}", path, e);
            std::process::exit(1);
        }
    };
    eprintln!(
        "anyon_generator listening on {} -> {}",
        path,
        out_dir.display()
    );
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let out_dir = out_dir.clone();
                std::thread::spawn(move || {
                    let _ = handle(s, out_dir);
                });
            }
            Err(e) => eprintln!("accept: {}", e),
        }
    }
}