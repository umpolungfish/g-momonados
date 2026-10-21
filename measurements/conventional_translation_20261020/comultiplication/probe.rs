fn main(){
 let args:Vec<_>=std::env::args().collect();
 let text=std::fs::read_to_string(&args[1]).unwrap();
 let n=text.trim().parse().unwrap();
 let result=match args[2].as_str(){
  "complex"=>g_momonados::conventional_spectrum::source_complex_protocol_control(n),
  "protocol"=>g_momonados::conventional_spectrum::source_protocol_control(n),
  "wells"=>g_momonados::reentry_wells::source_control(n),
  _=>panic!("unknown instrument")
 };
 match result{Ok(v)=>println!("{}",v),Err(e)=>{eprintln!("{}",e);std::process::exit(1)}}
}
