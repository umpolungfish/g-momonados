use g_momonados::{native_numeral::decode,word_tape::WordTape};
fn main()->Result<(),String>{
    let path=std::env::args().nth(1).ok_or("supply the saved device output")?;
    if path=="generate-control"{
        let prime=||->Result<WordTape,String>{
            let output=std::process::Command::new("openssl").args(["prime","-generate","-bits","112"])
                .output().map_err(|e|e.to_string())?;
            if !output.status.success(){return Err("control prime generation failed".into());}
            let decimal=String::from_utf8(output.stdout).map_err(|e|e.to_string())?;
            WordTape::from_canonical_word(&g_momonados::native_numeral::encode(decimal.trim()))
                .ok_or("invalid generated prime".into())
        };
        let p=prime()?;let mut q=prime()?;while p==q{q=prime()?;}
        let n=p.mul(&q);
        println!("{}",decode(n.as_word()).ok_or("generated source decode")?);
        return Ok(());
    }
    let text=std::fs::read_to_string(path).map_err(|e|e.to_string())?;
    let field=|name:&str|text.lines().find_map(|line|line.strip_prefix(name))
        .ok_or_else(||format!("device return omitted {name}"));
    let read=|name:&str|->Result<WordTape,String>{WordTape::from_canonical_word(field(name)?)
        .ok_or_else(||format!("device returned a noncanonical word for {name}"))};
    let n=read("source=")?;let e=read("idempotent=")?;let r=read("residual=")?;
    let expected=e.mul(&e.sub(&WordTape::one()).ok_or("candidate below one")?)
        .divmod(&n).ok_or("source zero")?.1;
    if r!=expected{return Err("GPU residual differs from exact word e(e-1) mod N".into());}
    let last=text.lines().filter(|line|line.starts_with("readout_seconds=")).last().ok_or("no device readout")?;
    let counter=|name:&str|->Result<u64,String>{last.split_whitespace().find_map(|s|s.strip_prefix(name))
        .ok_or_else(||format!("missing {name}"))?.parse().map_err(|_|format!("invalid {name}"))};
    let calls=counter("Y_calls=")?;let contractions=counter("Y_contractions=")?;
    if contractions!=calls+9||counter("powerset_binds=")?!=calls||counter("held=")?!=1{
        return Err("retained Y or singleton powerset counters differ".into());
    }
    if !text.contains("return=open-held-membrane"){
        return Err("this instrument expects a bounded held return".into());
    }
    println!("{{\"instrument\":\"word-native held-return verifier\",\"source_bits\":{},\"candidate\":\"{}\",\"Y_calls\":{},\"Y_contractions\":{},\"GPU_residual_equals_word_recurrence\":true,\"held_singleton_preserved\":true,\"proper_divisor_emitted\":false}}",
        n.bit_len(),decode(e.as_word()).ok_or("candidate decode")?,calls,contractions);
    Ok(())
}
