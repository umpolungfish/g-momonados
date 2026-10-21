use g_momonados::{ququart_decision::DecisionArena,phase_unbraid::{FixedComplex,FixedPointFormat}};
use num_bigint::{BigInt,BigUint};

fn main(){
 let text=std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
 let n:BigUint=text.trim().parse().unwrap();assert!(n.bits()>=200);
 let width=n.bits() as usize;let start=std::time::Instant::now();
 let e:Vec<_>=(0..width).map(|i|2*i+2).collect();
 let r:Vec<_>=(0..2*width+1).map(|i|2*i+3).collect();
 let mut arena=DecisionArena::new(4*width+5);
 let format=FixedPointFormat::for_modulus_bits(n.bits()).unwrap();
 let points=vec![BigUint::from(2u8),&n-1u8,&n>>1usize,&n/3u8,&n/5u8];
 let address=|point:&BigUint,residual:&BigUint,label:usize|{
  let mut value=BigUint::from(label);
  for (i,&wire) in e.iter().enumerate(){value.set_bit(wire as u64,point.bit(i as u64));}
  for (i,&wire) in r.iter().enumerate(){value.set_bit(wire as u64,residual.bit(i as u64));}
  value
 };
 let make=|a:&mut DecisionArena,reduced:Option<bool>|{
  let mut root=a.zero();
  for (j,point) in points.iter().enumerate(){for label in 0..4{
   let residual=match reduced{None=>BigUint::from(0u8),Some(false)=>point*point-point,Some(true)=>(point*point-point)%&n};
   let leaf=a.basis(&address(point,&residual,label),BigInt::from(1));
   let leaf=a.scale(leaf,&FixedComplex{re:BigInt::from(1+j+label)*format.scale(),
       im:-BigInt::from(2+3*j+label)*format.scale()},&format);
   root=a.sum(root,leaf);
  }}root
 };
 let mut root=make(&mut arena,None);let prepared=root;arena.fold_roots(&[prepared]);
 let mass=arena.mass(root);
 let powers:Vec<_>=(0..2*width).map(|i|BigUint::from(1u8)<<i).collect();
 for i in 0..width{
  root=arena.integer_affine_add(root,&r,&e[i+1..],2*i+2,&(&powers[2*i]-&powers[i]),&[(e[i],true)]).unwrap();
  arena.fold_roots(&[prepared,root]);
 }
 let integer=root;let expected_integer=make(&mut arena,Some(false));
 assert_eq!(integer,expected_integer,"integer square-minus-self changed a complete complex amplitude");
 assert_eq!(arena.mass(integer),mass);arena.fold_roots(&[prepared,integer]);
 root=arena.reduce_register(root,&r,&n).unwrap();
 let expected_modular=make(&mut arena,Some(true));
 assert_eq!(root,expected_modular,"single source reduction differs from independent integer arithmetic");
 assert_eq!(arena.mass(root),mass);
 arena.fold_roots(&[prepared,integer,root]);
 assert_eq!(make(&mut arena,None),prepared,"prepared bank lost its signed complex coefficients");
 println!("source_bits={} exact_integer_root=true exact_modular_root=true complete_complex_amplitudes=true mass_preserved=true prepared_bank_retained=true rows={} reductions=1 elapsed_seconds={:.9}",n.bits(),width,start.elapsed().as_secs_f64());
}
