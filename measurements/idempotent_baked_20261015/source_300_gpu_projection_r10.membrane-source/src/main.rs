use cudarc::driver::{CudaContext,LaunchConfig,PushKernelArg};
use cudarc::nvrtc::Ptx;
use std::time::Instant;
const PTX:&str=include_str!("../membrane.ptx");
const WIDTH:usize=302;
const SOURCE:&str="⊢≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣";

// Formatting only: arithmetic, including quotient and product, stays on GPU.
fn word(arms:&[u8])->Result<String,String>{
    if arms.iter().any(|&a|a!=8&&a!=9){return Err("invalid device parity cell".into());}
    let end=arms.iter().rposition(|&a|a==9).map_or(0,|i|i+1);
    let mut out=String::from("⊢");
    for &a in &arms[..end]{out.push_str(if a==9{"≻⋈∈⊥∋"}else{"≻⋈∈⊤∋"});}
    out.push_str("⊙⊡⊣");Ok(out)
}
fn main()->Result<(),String>{
    if std::env::args_os().len()!=1{return Err("baked membrane accepts no instance arguments".into());}
    let trigger=Instant::now();
    // Physical GPU 1 in the measured WSL inventory. Windows CUDA ordinals
    // differ, so identity is bound by UUID rather than a host-local ordinal.
    let wanted=[0x9b,0xa0,0xee,0xd8,0xe1,0x8b,0x02,0xa5,0x7c,0x80,0xca,0x49,0x71,0x2e,0x25,0xd1];
    let mut selected=None;
    for ordinal in 0..CudaContext::device_count().map_err(|e|e.to_string())? as usize{
        let ctx=CudaContext::new(ordinal).map_err(|e|format!("CUDA context: {e}"))?;
        let uuid=ctx.uuid().map_err(|e|e.to_string())?;
        if uuid.bytes.iter().map(|&x|x as u8).eq(wanted){selected=Some(ctx);break;}
    }
    let ctx=selected.ok_or("physical GPU 1 UUID not found")?;
    let name=ctx.name().map_err(|e|e.to_string())?;
    if !name.contains("4070"){return Err(format!("GPU 1 must be RTX 4070, found {name}"));}
    let stream=ctx.default_stream();
    let module=ctx.load_module(Ptx::from_src(PTX)).map_err(|e|format!("CUDA embedded PTX: {e}"))?;
    let kernel=module.load_function("idempotent_membrane").map_err(|e|e.to_string())?;
    let mut state=stream.clone_htod(&vec![8u8;11*WIDTH]).map_err(|e|e.to_string())?;
    let mut receipt=stream.clone_htod(&[0u64;8]).map_err(|e|e.to_string())?;
    println!("device=1 native_cuda_ordinal={} name={name} uuid=GPU-9ba0eed8-e18b-02a5-7c80-ca49712e25d1\nsource={SOURCE}\ninstance_inputs=N-only baked\nproducer=GPU-source-powered-idempotent\noperator=s=2e-1; s'=s^k mod N; e'=(s'+1)/2 mod N\nY_reducer=GPU powerset_bind=GPU idempotent_arithmetic=GPU factor_extraction=GPU quotient=GPU exact_product=GPU",ctx.ordinal());
    let mut initialize=1i32;
    let mut last_report=Instant::now();
    loop{
        let mut launch=stream.launch_builder(&kernel);
        launch.arg(&mut state).arg(&mut receipt).arg(&32u32).arg(&initialize);
        unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(32,1,1),shared_mem_bytes:0})}
            .map_err(|e|e.to_string())?;
        let observed=stream.clone_dtoh(&receipt).map_err(|e|e.to_string())?;
        if initialize!=0||last_report.elapsed().as_secs()>=1||observed[3]!=0{
            println!("readout_seconds={:.9} Y_contractions={} Y_calls={} powerset_binds={} held={} restarts={} projecting={} singularities={} status={}",
                trigger.elapsed().as_secs_f64(),observed[0],observed[1],observed[2],observed[4],observed[5],observed[6],observed[7],observed[3]);
            last_report=Instant::now();
        }
        if observed[3]!=0{
            let returned=stream.clone_dtoh(&state).map_err(|e|e.to_string())?;
            for (index,label) in ["idempotent","residual","complement","divisor","quotient","exponent","initial_candidate","last_trivial_candidate","last_trivial_residual","source_power","singular_projection"].iter().enumerate(){
                println!("{label}={}",word(&returned[index*WIDTH..(index+1)*WIDTH])?);
            }
            if observed[3]!=1{return Err(format!("device returned status {}",observed[3]));}
            println!("GPU_idempotency=true GPU_orthogonality=true GPU_complementary_sum=true GPU_proper_divisor=true GPU_remainder=0 GPU_exact_word_product=true GPU_paired_boundary=true\ntrigger_to_verified_factor_seconds={:.9}",trigger.elapsed().as_secs_f64());
            return Ok(());
        }
        initialize=0;
    }
}
