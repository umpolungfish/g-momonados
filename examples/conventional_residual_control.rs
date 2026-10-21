fn main()->Result<(),String> {
    let args:Vec<_>=std::env::args().collect();
    if args.len()!=2{return Err("source control accepts N alone".into());}
    let n=args[1].parse().map_err(|_|"invalid N")?;
    let result=g_momonados::conventional_spectrum::source_residual_control(n)?;
    println!("{}",serde_json::to_string_pretty(&result).map_err(|e|e.to_string())?);
    Ok(())
}
