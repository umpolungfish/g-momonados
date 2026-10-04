use g_momonados::sic::certificate::{EvidencePolicy, SicCertificate};
use g_momonados::sic::frame::Complex;
use g_momonados::sic::wh::WhSic;
use g_momonados::sic::{BlochVector, QubitState, Sic, SicError, TetraSic};

fn run(args: &[String]) -> Result<(), SicError> {
    let policy = EvidencePolicy::new(1e-12, 1e-8)?;
    match args.first().map(String::as_str).unwrap_or("qubit") {
        "qubit" => {
            let tetra = TetraSic::new();
            let mut certificate = SicCertificate::measure(&tetra.frame)?;
            certificate.exact = Some(g_momonados::sic::exact::certify_tetrahedron());
            print!("{}", certificate.report(policy));
            if args.len() > 1 {
                if args.len() != 4 {
                    return Err(SicError("usage: sic-tool qubit [rx ry rz]".into()));
                }
                let values = args[1..]
                    .iter()
                    .map(|x| {
                        x.parse::<f64>()
                            .map_err(|_| SicError("invalid Bloch coordinate".into()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let state = QubitState::new(BlochVector::new(values[0], values[1], values[2]))?;
                let p = tetra.split(&state);
                println!(
                    "SIC coordinates      : {:?}",
                    p.distribution().probabilities()
                );
                println!("SIC reconstruction   : {:?}", p.reconstruct()?.bloch());
            }
        }
        "wh" | "wh-gpu" => {
            let path = args
                .get(1)
                .ok_or_else(|| SicError("usage: sic-tool wh|wh-gpu <fiducial.json>".into()))?;
            let text = std::fs::read_to_string(path).map_err(|e| SicError(e.to_string()))?;
            let entries: Vec<[f64; 2]> =
                serde_json::from_str(&text).map_err(|e| SicError(e.to_string()))?;
            let wh = WhSic::new(
                entries
                    .into_iter()
                    .map(|z| Complex::new(z[0], z[1]))
                    .collect(),
            )?;
            let field = if args[0] == "wh-gpu" {
                g_momonados::sic::gpu::wh_overlaps(&wh)?
            } else {
                wh.overlaps()?
            };
            let d = wh.dimension();
            let target = 1.0 / (d + 1) as f64;
            let equi = field
                .values
                .iter()
                .skip(1)
                .map(|z| (z.abs2() - target).abs())
                .fold(0.0, f64::max);
            println!("SIC dimension        : {d}\nSIC outcomes         : {}\nWH normalization     : {:.6e}\nWH overlap max error : {equi:.6e}",d*d,(field.values[0].re-1.0).abs());
            if d <= 8 {
                let mut certificate = SicCertificate::measure(&wh)?;
                certificate.wh_overlap = Some(equi);
                print!("{}", certificate.report(policy));
            } else {
                println!("SIC operator closure : not evaluated (overlap-only verification)");
            }
        }
        _ => {
            return Err(SicError(
                "usage: sic-tool qubit [rx ry rz] | wh|wh-gpu <fiducial.json>".into(),
            ))
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("sic-tool: {error}");
        std::process::exit(2);
    }
}
