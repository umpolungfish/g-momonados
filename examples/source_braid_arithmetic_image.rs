//! Exercise the full-source arithmetic representation and exact address readback.
//! This is not a factor producer or a collision-trace evaluator.
use g_momonados::{source_braid_state::arithmetic_image::SquareImage,
    ququart_decision::DecisionArena,word_tape::WordTape};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::json;

fn main()->Result<(),Box<dyn std::error::Error>> {
    let mut args=std::env::args().skip(1);
    let source_path=args.next().ok_or("N-only source path")?;
    let output=args.next().ok_or("receipt path")?;
    let n=BigUint::parse_bytes(std::fs::read_to_string(source_path)?.trim().as_bytes(),10)
        .ok_or("decimal N-only source")?;
    let started=std::time::Instant::now();
    let source=WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&n)).unwrap();
    let width=n.bits() as usize;
    let wires:Vec<_>=(0..width).collect();
    let flag=width;
    let mut arena=DecisionArena::new(width+1);
    let mut root=arena.basis(&BigUint::zero(),BigInt::one());
    for &wire in &wires {let high=arena.flip(root,wire);root=arena.sum(root,high);}
    root=arena.compare_constant(root,&wires,&n,&[],flag);
    let zero=arena.zero();root=arena.conditional_literals(zero,root,&[(flag,true)]);
    root=arena.flip(root,flag);
    arena.fold_roots(&[root]);
    assert_eq!(arena.mass(root),n);
    let image=SquareImage::new(source.clone(),root)?;
    let projected=image.project_proper_axis_family();
    let preparation_seconds=started.elapsed().as_secs_f64();
    let mut witnesses=Vec::new();
    for address in [BigUint::zero(),BigUint::one(),&n-1u8,&n>>1,&n/3u8] {
        let native=WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&address)).unwrap();
        assert!(projected.amplitude_at(&native,&WordTape::zero())?.is_zero());
        assert!(projected.amplitude_at(&WordTape::zero(),&native)?.is_zero());
        assert_eq!(projected.fibre_extent_at(&native)?,if native.is_zero(){WordTape::zero()}
            else{native.gcd(&source).sub(&WordTape::one()).unwrap()});
        let returned=image.return_at(&native)?;
        let expected=WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&(&address*&address))).unwrap();
        assert_eq!(returned.integer,expected);
        assert_eq!(returned.quotient.mul(&source).add(&returned.modular),expected);
        assert!(image.inverse_at(&returned)?.is_zero());
        let opposite=if address.is_zero(){BigUint::zero()}else{&n-&address};
        let opposite=WordTape::from_canonical_word(&vox_core::combined_factor::native_word(&opposite)).unwrap();
        assert!(image.collision_at(&native,&opposite)?);
        let (a,b)=image.equality_chart(&native,&opposite)?;
        let (x,y)=image.inverse_chart(&a,&b)?;
        assert_eq!(x,native);assert_eq!(y,opposite);
        assert!(a.mul(&b).divmod(&source).unwrap().1.is_zero());
        let fibre=image.contract_zero_product_fibre(&a)?;
        let first=fibre.member(&WordTape::zero())?;
        let last=fibre.member(&fibre.count.sub(&WordTape::one()).unwrap())?;
        let axis=image.paired_axis_readout(&a,&WordTape::zero())?;
        assert_eq!(axis.divisor,fibre.count);
        assert_eq!(axis.left.modular,axis.right.modular);
        witnesses.push(json!({"address":address.to_string(),"integer_word":returned.integer.as_word(),
            "modular_word":returned.modular.as_word(),"quotient_word":returned.quotient.as_word(),
            "inverse_zero":true,"opposite_collision":true,"chart_inverse":true,
            "zero_product_fibre":{"a_word":a.as_word(),"step_word":fibre.step.as_word(),
                "count_word":fibre.count.as_word(),"first_word":first.as_word(),"last_word":last.as_word()},
            "axis_readout":{"source_left_word":axis.left.address.as_word(),
                "source_right_word":axis.right.address.as_word(),"gcd_word":axis.divisor.as_word(),
                "proper_product":axis.factors.is_some(),"source_point_selected_by_producer":false}}));
    }
    let result=json!({"image":image.record(),"proper_family_projection":projected.record(),"source_mass":n.to_string(),
        "retained_source_nodes":arena.retained_nodes(),"preparation_seconds":preparation_seconds,
        "total_seconds":started.elapsed().as_secs_f64(),"readback":witnesses,
        "collision_trace_evaluated":false,"factors":null});
    std::fs::write(output,serde_json::to_vec_pretty(&result)?)?;
    println!("source_bits={width} source_nodes={} arithmetic_edges=4 preparation_seconds={preparation_seconds}",arena.retained_nodes());
    Ok(())
}
