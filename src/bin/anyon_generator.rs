//! anyon_generator.rs — Unix-socket anyon generator speaking the
//! g-momonados/fibonacci-anyons-v2 protocol.
//!
//! The client streams `exchange` messages that form the compiled braid.
//! The server saves that braid and its source metadata. Fusion measurement
//! requires an execution backend; this compiler reports its absence instead
//! of supplying synthetic phase bits.
//!
//! Output path: <out-dir>/shot-<shot_id>.braid
//!   Each line is one exchange generator, in the order received.
//!   A leading metadata line records source bits, base bits, phase_bits,
//!   logical_qubits, and work preparation.
//!
//! Usage: anyon_generator <unix-socket-path> <out-dir>

use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

struct Shot {
    shot_id: u64,
    source_bits: String,
    base_bits: String,
    logical_qubits: u64,
    phase_bits: usize,
    work_preparation: String,
    // The compiled braid, appended to on each exchange.
    exchanges: Vec<i32>,
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
            "status" => {
                writeln!(stream, "{}", serde_json::json!({
                    "ok": true,
                    "protocol": "g-momonados/fibonacci-anyons-v2",
                    "backend": "braid_compiler",
                    "fusion_readout_available": false,
                }))?;
                stream.flush()?;
            }
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

                shot = Some(Shot {
                    shot_id,
                    source_bits,
                    base_bits,
                    logical_qubits,
                    phase_bits,
                    work_preparation,
                    exchanges: Vec::new(),
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
                let path = write_compiled(&out_dir, s)?;
                writeln!(stream, "{}", serde_json::json!({
                    "shot_id": shot_id,
                    "phase_index": phase_index,
                    "error": "fusion execution backend is not configured",
                    "compiled_braid": path,
                }))?;
                stream.flush()?;
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
