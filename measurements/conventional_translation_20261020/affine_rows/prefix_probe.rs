fn main(){
 let n=std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
 let receipt=g_momonados::conventional_spectrum::source_affine_prefix_control(n.trim().parse().unwrap(),2).unwrap();
 println!("{receipt}");
}
