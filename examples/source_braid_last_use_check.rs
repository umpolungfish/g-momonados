//! Exact coherent last-use equivalence on a source-bound sparse register.
//! This checks the operation on a >=200-bit source, not full-source factoring.
use g_momonados::{ququart_decision::DecisionArena,word_tape::WordTape,
    source_braid_state::last_use::FunctionalBank};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::json;

fn main()->Result<(),Box<dyn std::error::Error>> {
    let path=std::env::args().nth(1).ok_or("expected N-only source path")?;
    let raw=std::fs::read_to_string(path)?;
    let n=BigUint::parse_bytes(raw.trim().as_bytes(),10).ok_or("decimal source")?;
    let width=n.bits() as usize;
    assert!(width>=200);
    let source=WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&n)).unwrap();
    let x:Vec<_>=(0..width).map(|i|2*i).collect();
    let r:Vec<_>=(0..2*width+1).map(|i|2*i+1).collect();
    let mut arena=DecisionArena::new(4*width+3);
    let zero=arena.zero();
    let addresses=[BigUint::zero(),BigUint::one(),&n-1u8,&n>>1];
    let weights=[(2i32,0i32),(0,1),(-1,0),(1,-2)];
    let packed=|address:&BigUint,returned:&BigUint| {
        let mut value=BigUint::zero();
        for (i,&wire) in x.iter().enumerate(){if address.bit(i as u64){value|=BigUint::one()<<wire;}}
        for (i,&wire) in r.iter().enumerate(){if returned.bit(i as u64){value|=BigUint::one()<<wire;}}
        value
    };
    let mut prepared=zero;
    for (address,&(re,im)) in addresses.iter().zip(&weights) {
        let root=arena.basis(&packed(address,&BigUint::zero()),BigInt::one());
        let root=arena.scale_complex_numerator(root,&(BigInt::from(re),BigInt::from(im)));
        prepared=arena.sum(prepared,root);
    }
    let mut late=prepared;
    let mut early=prepared;
    arena.fold_roots(&[prepared,late,early]);
    let mut bank=FunctionalBank{source:source.clone(),applied_rows:0};
    let started=std::time::Instant::now();
    for i in 0..width {
        late=arena.integer_affine_add(late,&r,&x[i+1..],2*i+2,
            &(BigUint::one()<<(2*i)),&[(x[i],true)])?;
        early=arena.integer_affine_add(early,&r,&x[i+1..],2*i+2,
            &(BigUint::one()<<(2*i)),&[(x[i],true)])?;
        early=arena.contract_internal_wire(early,x[i]);
        early=arena.conditional_literals(zero,early,&[(x[i],false)]);
        bank.applied_rows+=1;
        for (address,&(re,im)) in addresses.iter().zip(&weights) {
            let native=WordTape::from_canonical_word(&vox_core::combined_factor::native_word(address)).unwrap();
            let returned_word=bank.integer_return(&native)?;
            let returned=BigUint::parse_bytes(vox_core::morphism_factor::dec_of(
                &returned_word.to_imasm_tape()).as_bytes(),10).ok_or("native return decimal")?;
            let high=address>>(i+1);
            assert_eq!(returned,address*address-((&high*&high)<<(2*(i+1))));
            assert_eq!(arena.amplitude(late,&packed(address,&returned)),(BigInt::from(re),BigInt::from(im)));
            let modular=bank.modular_return(&native)?;
            assert!(bank.recover_zero_bank(&native,&modular)?.is_zero());
        }
        let mut reference=late;
        for &wire in &x[..=i] {
            reference=arena.contract_internal_wire(reference,wire);
            reference=arena.conditional_literals(zero,reference,&[(wire,false)]);
        }
        assert_eq!(reference,early,"last-use commutation at row {i}");
        arena.fold_roots(&[prepared,late,early]);
    }
    late=arena.reduce_register(late,&r,&n)?;
    arena.fold_roots(&[prepared,late,early]);
    early=arena.reduce_register(early,&r,&n)?;
    arena.fold_roots(&[prepared,late,early]);
    for &wire in &x {
        late=arena.contract_internal_wire(late,wire);
        late=arena.conditional_literals(zero,late,&[(wire,false)]);
    }
    assert_eq!(late,early,"modular return and final coherent observation");
    let gram=arena.control_gram(&[prepared,late,early,zero]);
    println!("{}",json!({"source_bits":width,"rows_checked":width,
        "address_witnesses":addresses.map(|x|x.to_string()),"complex_weights":weights,
        "last_use_equals_delayed_contraction":true,"functional_bank_inverse":true,
        "modular_observation_equal":true,"retained_nodes":arena.retained_nodes(),
        "seconds":started.elapsed().as_secs_f64(),"gram":gram.map(|(r,i)|[r.to_string(),i.to_string()]),
        "scope":"four coherent source addresses, not full-source factor production"}));
    Ok(())
}
