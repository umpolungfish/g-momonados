use g_momonados::{ququart_decision::DecisionArena,phase_unbraid::{FixedComplex,FixedPointFormat}};
use num_bigint::{BigInt,BigUint};
fn main(){
 let text=std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
 let n:BigUint=text.trim().parse().unwrap();assert!(n.bits()>=200);
 let width=n.bits() as usize;let start=std::time::Instant::now();
 let source:Vec<_>=(0..width).map(|i|2*i).collect();
 let register:Vec<_>=(0..2*width).map(|i|2*i+1).collect();let control=4*width;
 let states=vec![(n.clone()-1u8,n.clone()-2u8,true,3i64,2i64),
  (&n>>1usize,&n>>3usize,true,-5,7),(&n>>2usize,&n+5u8,true,11,-1),
  (&n-2u8,&n-1u8,false,2,-9),(&n>>3usize,BigUint::from(0u8),true,-7,-4)];
 let address=|e:&BigUint,r:&BigUint,enabled:bool|{
  let mut value=BigUint::from(0u8);
  for (i,&wire) in source.iter().enumerate(){if e.bit(i as u64){value|=BigUint::from(1u8)<<wire;}}
  for (i,&wire) in register.iter().enumerate(){if r.bit(i as u64){value|=BigUint::from(1u8)<<wire;}}
  if enabled{value|=BigUint::from(1u8)<<control;}value
 };
 for shift in [0usize,1,7,width-1]{
  let mut arena=DecisionArena::new(control+1);let mut root=arena.zero();
  let format=FixedPointFormat::for_modulus_bits(n.bits()).unwrap();
  for (e,r,enabled,re,im) in &states {
   let value=arena.basis(&address(e,r,*enabled),BigInt::from(1));
   let value=arena.scale(value,&FixedComplex{re:BigInt::from(*re)*format.scale(),im:BigInt::from(*im)*format.scale()},&format);
   root=arena.sum(root,value);
  }
  arena.fold_roots(&[root]);let mass=arena.mass(root);let c=&n-3u8;
  let changed=arena.modular_affine_add(root,&register,&source,shift,&c,&n,&[(control,true)]).unwrap();
  arena.fold_roots(&[root,changed]);assert_eq!(arena.mass(changed),mass);
  let mut expected=arena.zero();
  for (e,r,enabled,re,im) in &states {
   let output=if *enabled && r<&n {(r+(e<<shift)+&c)%&n}else{r.clone()};
   let point=arena.basis(&address(e,&output,*enabled),BigInt::from(1));
   let point=arena.scale(point,&FixedComplex{re:BigInt::from(*re)*format.scale(),im:BigInt::from(*im)*format.scale()},&format);
   expected=arena.sum(expected,point);
  }
  assert_eq!(changed,expected,"exact joint complex coefficient diagram differs from independent integer oracle");
  if shift==1 {
   use g_momonados::reentry_powerset::{Held,unit,bind,multiply};
   let held:Held<_>=[root,changed].into_iter().collect();
   let f=|a:&mut DecisionArena,x|->Result<Held<usize>,String>{
    let y=a.modular_affine_add(x,&register,&source,shift,&c,&n,&[(control,true)])?;
    Ok([x,y].into_iter().collect())
   };
   let g=|a:&mut DecisionArena,x|->Result<Held<usize>,String>{
    a.modular_affine_add(x,&register,&source,shift,&BigUint::from(1u8),&n,&[(control,true)]).map(unit)
   };
   assert_eq!(bind(unit(root),|x|f(&mut arena,x)).unwrap(),f(&mut arena,root).unwrap());
   assert_eq!(bind(held.clone(),|x|Ok::<_,String>(unit(x))).unwrap(),held);
   let fx=bind(held.clone(),|x|f(&mut arena,x)).unwrap();
   let left=bind(fx,|x|g(&mut arena,x)).unwrap();
   let right=bind(held.clone(),|x|{let fx=f(&mut arena,x)?;bind(fx,|y|g(&mut arena,y))}).unwrap();
   assert_eq!(left,right);
   let grouped=unit(held.clone());let separated:Held<_>=held.iter().copied().map(unit).collect();
   assert_ne!(grouped,separated);assert_eq!(multiply(grouped),multiply(separated));
   assert!(bind(Held::<usize>::new(),|x|g(&mut arena,x)).unwrap().is_empty());
   assert_ne!(Held::<usize>::new(),unit(root));
   println!("monad_left_unit=true right_unit=true associativity=true nested_distinction=true empty_distinction=true");
  }
  let mut reversed=changed;
  for &wire in &source{reversed=arena.flip(reversed,wire);}
  arena.fold_roots(&[root,changed,reversed]);
  let total=(&c+(((BigUint::from(1u8)<<width)-1u8)<<shift))%&n;
  let inverse=(&n-total)%&n;
  reversed=arena.modular_affine_add(reversed,&register,&source,shift,&inverse,&n,&[(control,true)]).unwrap();
  for &wire in &source{reversed=arena.flip(reversed,wire);}
  assert_eq!(reversed,root,"full complex affine inverse did not recover its bank");
  println!("shift={shift} exact_complex_support=true mass_preserved=true disabled_arm=true invalid_arm=true inverse_bank_recovery=true");
 }
 println!("source_bits={} elapsed_seconds={:.9}",n.bits(),start.elapsed().as_secs_f64());
}
