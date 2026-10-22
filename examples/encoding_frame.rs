//! N-only encoding/coherence extraction experiment, with exact calibrated SIC return.
use g_momonados::{anyon_ququart::FixedQuquartSic, phase_unbraid::FixedPointFormat};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero,Signed};
use serde_json::json;
fn gcd(mut a:BigUint,mut b:BigUint)->BigUint{while !b.is_zero(){let r=&a%&b;a=b;b=r;}a}
fn main()->Result<(),String>{
 let raw=std::env::args().nth(1).ok_or("encoding_frame <decimal|native|hex>")?;
 let value=vox_core::godel_calculus::parse_natural_word(&raw)?;
 let n=BigUint::parse_bytes(value.to_string().as_bytes(),10).ok_or("invalid source")?;
 let native=vox_core::combined_factor::native_word(&n);let hex=vox_core::combined_factor::hex_word(&n);
 let native_bits=vox_core::morphism_factor::parse_numeral(&native)?;
 let hex_bits=vox_core::combined_factor::decode_hex_bits(&hex)?;
 if native_bits.iter().map(|&b|b=='⊥').collect::<Vec<_>>()!=hex_bits{return Err("encoding transport lost a bit".into());}
 // Keep source place values. Coordinates are T,F,I,fork bit planes, not
 // aggregate motif counts, so their weighted sum returns the exact source.
 let mut lanes:[BigInt;4]=core::array::from_fn(|_|BigInt::zero());
 for (bit,&set) in hex_bits.iter().enumerate(){if set{lanes[bit%4]+=BigInt::one()<<(bit-bit%4);}}
 let recovered=(0..4).fold(BigInt::zero(),|s,j|s+(&lanes[j]<<j));
 if recovered!=BigInt::from(n.clone()){return Err("four-plane source return failed".into());}
 // A relative phase makes the imaginary coherences participate in recovery.
 let amplitudes:[(BigInt,BigInt);4]=core::array::from_fn(|i|if i==3{(BigInt::zero(),lanes[i].clone())}else{(lanes[i].clone(),BigInt::zero())});
 let gram=core::array::from_fn(|k|{let (a,b)=&amplitudes[k/4];let(c,d)=&amplitudes[k%4];(a*c+b*d,a*d-b*c)});
 let format=FixedPointFormat::for_modulus_bits(n.bits())?;let sic=FixedQuquartSic::new(&format)?;
 let masses=sic.gram_mass_numerators(&gram)?;
 let analytic=sic.certify_gram_frame_unrounded_masses(&gram,&masses)?;
 let exact=sic.exact_gram_from_masses(&masses)?;
 if exact.numerators.iter().zip(&gram).any(|((a,b),(c,d))|a!=&(c*&exact.denominator)||b!=&(d*&exact.denominator)){return Err("mass-only exact complex recovery failed".into());}
 let mut changed=masses.clone();changed[0]+=BigUint::one();
 let changed_return=sic.exact_gram_from_masses(&changed)?;
 if changed_return.numerators.iter().zip(&gram).all(|((a,b),(c,d))|a==&(c*&changed_return.denominator)&&b==&(d*&changed_return.denominator)){return Err("changed masses returned the unchanged Gram".into());}
 // Undo the retained i phase before reading the three 2x2 flattenings.
 let g=&exact.numerators;let den=&exact.denominator;
 let candidates=[(&g[3].1-&g[6].0)/den,(&g[2].0-&g[7].1)/den,(&g[1].0-&g[11].1)/den];
 let zeta_defect_gcds:Vec<_>=analytic.signed_residual_numerators.iter().flat_map(|(a,b)|[a,b]).filter(|v|!v.is_zero()).map(|v|gcd(n.clone(),v.abs().to_biguint().unwrap()).to_string()).collect();
 let tests:Vec<_>=candidates.iter().map(|candidate|{let d=gcd(n.clone(),candidate.abs().to_biguint().unwrap());let proper=d>BigUint::one()&&d<n;
 json!({"coherence_difference":candidate.to_string(),"gcd":d.to_string(),"factor":if proper{Some(d.to_string())}else{None},"quotient":if proper{Some((&n/&d).to_string())}else{None},"product_closes":proper&&(&n%&d).is_zero()})}).collect();
 println!("{}",json!({"N":n.to_string(),"input_bits":n.bits(),"native_word":native,"hex_word":hex,"plane_order":["T","F","I","fork"],"planes":lanes.map(|a|a.to_string()),"weighted_source_return":true,"relative_phase":["1","1","1","i"],"zeta_minus_one":"-1/12","ideal_dual_diagonal":"5/4","ideal_dual_uniform":"-1/16","analytic_integer_residual":analytic.rounded.maximum_residual.to_string(),"analytic_rational_residual_nonzero":analytic.signed_residual_numerators.iter().any(|(a,b)|!a.is_zero()||!b.is_zero()),"exact_mass_only_denominator":exact.denominator.to_string(),"exact_complex_recovery":true,"changed_mass_changes_recovered_state":true,"zeta_defect_gcds":zeta_defect_gcds,"candidate_tests":tests}));
 Ok(())
}
