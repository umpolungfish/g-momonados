//! Diagnostic carrier: compilation prepares tensors; triggering still contracts them.
#[cfg(not(target_feature="crt-static"))]
compile_error!("point_carrier_probe requires a self-contained static image");
use std::time::Instant;
include!(concat!(env!("OUT_DIR"),"/point_prepared.rs"));
fn main()->Result<(),String>{
    let mode=std::env::args().nth(1);
    let dump=mode.as_deref()==Some("--dump-prepared");
    let inspect=mode.as_deref()==Some("--inspect-prepared")||dump;
    let hold=mode.as_deref()==Some("--hold");
    if std::env::args_os().len()!=if hold{4}else if inspect{2}else{1}{return Err("compiled carrier accepts no source inputs; --hold takes steps and a return path".into());}
    let loading=Instant::now();let switch=compiled_point_switch()?;
    eprintln!("prepared-state-restored seconds={:.9}",loading.elapsed().as_secs_f64());
    if dump{println!("{}",switch.prepared_record()?);return Ok(());}
    if inspect{println!("{}",switch.loading_record());return Ok(());}
    if hold {
        let steps=std::env::args().nth(2).unwrap().parse::<u64>().map_err(|e|e.to_string())?;
        let path=std::env::args().nth(3).unwrap();
        println!("{}",switch.hold(steps,std::path::Path::new(&path))?);return Ok(());
    }
    let trigger=Instant::now();let returned=switch.trigger()?;
    eprintln!("switch-to-return seconds={:.9}",trigger.elapsed().as_secs_f64());
    println!("{returned}");Ok(())
}
