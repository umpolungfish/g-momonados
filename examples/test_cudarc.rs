use cudarc::driver::CudaContext;

fn main() {
    let ctx = CudaContext::new(0).expect("context");
    println!("Success: {:?}", ctx.name());
}
