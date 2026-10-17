//! Shared GLUT multiplier/carry relation executed on CUDA. Host preparation
//! exports the existing nested construction; all branching executes on device.
use cudarc::driver::{CudaContext,LaunchConfig,PushKernelArg};
use cudarc::nvrtc::compile_ptx;
use num_bigint::BigUint;
use std::time::Instant;

/// Compile the existing Quantum port's gates from N alone. Targets equal to
/// u32::MAX reverse the relative sign; other targets are controlled X gates.
#[derive(Default)]
struct FactorPhaseProgram {
 offsets:Vec<u32>, controls:Vec<u32>, values:Vec<u32>, targets:Vec<u32>,
 width:u32, cells:usize,
}
impl FactorPhaseProgram {
 fn prepare(bits:&[char])->Result<Self,String> {
   use vox_core::fixed_point_quantum_phase::factor_oracle::FactorPhaseOracle;
   let oracle=FactorPhaseOracle::from_n(bits).map_err(str::to_owned)?;
   let mut program=Self {offsets:vec![0],width:oracle.layout().p.len() as u32,
     cells:oracle.layout().cells,..Self::default()};
   oracle.lower(&mut program).map_err(str::to_owned)?;
   Ok(program)
 }
 fn gate(&mut self,controls:&[vox_core::fixed_point_quantum_phase::factor_oracle::Control],target:u32)
   ->Result<(),&'static str> {
   for control in controls {
     if control.cell>=self.cells||!matches!(control.value,'⊤'|'⊥') {
       return Err("CUDA factor-phase control outside canonical layout");
     }
     self.controls.push(u32::try_from(control.cell).map_err(|_|"CUDA phase cell overflow")?);
     self.values.push(u32::from(control.value=='⊥'));
   }
   self.offsets.push(u32::try_from(self.controls.len()).map_err(|_|"CUDA phase control extent overflow")?);
   self.targets.push(target);Ok(())
 }
}
impl vox_core::fixed_point_quantum_phase::factor_oracle::GateSink for FactorPhaseProgram {
 fn toggle(&mut self,controls:&[vox_core::fixed_point_quantum_phase::factor_oracle::Control],target:usize)
   ->Result<(),&'static str> {
   if target>=self.cells||controls.iter().any(|c|c.cell==target) {
     return Err("CUDA factor-phase target outside canonical layout");
   }
   self.gate(controls,u32::try_from(target).map_err(|_|"CUDA phase target overflow")?)
 }
 fn phase_flip(&mut self,controls:&[vox_core::fixed_point_quantum_phase::factor_oracle::Control])
   ->Result<(),&'static str> {self.gate(controls,u32::MAX)}
}

fn read_checkpoint(path:&str)->Result<Vec<Vec<u32>>,String> {
 use std::io::Read;
 let mut file=std::io::BufReader::new(std::fs::File::open(path).map_err(|e|format!("GLUT resume open: {e}"))?);
 let mut magic=[0u8;8];file.read_exact(&mut magic).map_err(|e|format!("GLUT resume header: {e}"))?;
 let fields=match &magic {b"GLUTGPU2"=>15,b"GLUTGPU3"=>19,b"GLUTGPU4"=>23,
   _=>return Err("GLUT resume requires a parity-enabled checkpoint format".into())};
 let mut vectors=Vec::new();
 for _ in 0..fields {
   let mut bytes=[0u8;8];file.read_exact(&mut bytes).map_err(|e|format!("GLUT resume length: {e}"))?;
   let len=usize::try_from(u64::from_le_bytes(bytes)).map_err(|_|"GLUT resume extent overflow")?;
   let bytes_len=len.checked_mul(4).ok_or("GLUT resume byte extent overflow")?;
   let mut words=Vec::new();
   words.try_reserve_exact(len).map_err(|e|format!("GLUT resume allocation: {e}"))?;
   let mut raw=vec![0u8;bytes_len];file.read_exact(&mut raw).map_err(|e|format!("GLUT resume vector: {e}"))?;
   for word in raw.chunks_exact(4){words.push(u32::from_le_bytes(word.try_into().unwrap()));}
   vectors.push(words);
 }
 let mut extra=[0u8;1];if file.read(&mut extra).map_err(|e|format!("GLUT resume tail: {e}"))?!=0 {
   return Err("GLUT resume has trailing fields".into());
 }
 Ok(vectors)
}

fn write_device_state(path:&std::path::Path,stream:&std::sync::Arc<cudarc::driver::CudaStream>,
 source:&[&[u32]],decisions:&[&cudarc::driver::CudaSlice<u32>],word:&[u32],
 observations:&[&cudarc::driver::CudaSlice<u32>],terminal:Option<&[&cudarc::driver::CudaSlice<u32>]>)
 ->Result<(),String> {
 use std::io::Write;
 fn vector(file:&mut std::fs::File,words:&[u32])->Result<(),String> {
   file.write_all(&(words.len() as u64).to_le_bytes()).map_err(|e|format!("GLUT state extent: {e}"))?;
   let bytes:Vec<u8>=words.iter().flat_map(|v|v.to_le_bytes()).collect();
   file.write_all(&bytes).map_err(|e|format!("GLUT state cells: {e}"))
 }
 if source.len()!=10||decisions.len()!=5||observations.len()!=3||terminal.is_some_and(|v|v.len()!=4) {
   return Err("GLUT state serialization shape mismatch".into());
 }
 if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()) {
   std::fs::create_dir_all(parent).map_err(|e|format!("GLUT state directory: {e}"))?;
 }
 let mut file=std::fs::File::create(path).map_err(|e|format!("GLUT state create: {e}"))?;
 file.write_all(if terminal.is_some(){b"GLUTGPU4"}else{b"GLUTGPU3"}).map_err(|e|format!("GLUT state header: {e}"))?;
 for words in source {vector(&mut file,words)?;}
 for buffer in decisions {vector(&mut file,&stream.clone_dtoh(*buffer).map_err(|e|format!("GLUT state decision read: {e}"))?)?;}
 vector(&mut file,word)?;
 for buffer in observations {vector(&mut file,&stream.clone_dtoh(*buffer).map_err(|e|format!("GLUT state observation read: {e}"))?)?;}
 if let Some(buffers)=terminal {
   for buffer in buffers {vector(&mut file,&stream.clone_dtoh(*buffer).map_err(|e|format!("GLUT state terminal read: {e}"))?)?;}
 }
 file.sync_all().map_err(|e|format!("GLUT state sync: {e}"))
}

fn validate_enclosing_checkpoint(v:&[Vec<u32>],ops:&[u32])->Result<bool,String> {
 if v.len()==15{return Ok(false);}
 if v.len()!=19{return Err("GLUT resume checkpoint field count mismatch".into());}
 if v[15]!=ops||v[16].len()!=4*ops.len()||v[17].len()!=4*ops.len()
   ||v[18].len()!=10||v[18][0]>15||v[18][1]!=1||v[18][2]!=0||v[18][3]!=0||v[18][7]!=0 {
   return Err("GLUT resume enclosing word or held-frame mismatch".into());
 }
 for i in 0..v[16].len() {
   let lanes=match ops[i/4] {5=>1,9=>2,10=>12,_=>0};
   if v[16][i]>v[17][i]||v[17][i]>1||(v[17][i]!=0&&lanes&(1u32<<(i%4))==0) {
     return Err("GLUT resume enclosing deposit ancestry mismatch".into());
   }
 }
 Ok(true)
}

pub fn run(n:&BigUint)->Result<String,String> {
 let started=Instant::now();
 if n.bits()==0 || n.bits()>1024 {return Err("GPU GLUT source must be nonzero and fit the 1024-bit arithmetic carrier".into());}
 let bits:Vec<char>=(0..n.bits()).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
 let relation=vox_core::glut_system::prepare_device_correlation(&bits);
 let factor_phase=FactorPhaseProgram::prepare(&bits)?;
 let admission:Vec<u32>=relation.admission.iter().flat_map(|&(p,q,cell)|[p as u32,q as u32,cell as u32]).collect();
 let admission_rows=relation.admission.len() as u32;
 let midpoint_cells=relation.midpoint.len();let gap_cells=relation.gap.len();
 if relation.empty{return Err("prepared GLUT relation is empty".into());}
 let mut offsets=vec![0u32];let mut cells=Vec::new();let mut supports=Vec::new();let mut all=Vec::new();
 for gate in &relation.gates {
   if gate.cells.len()!=gate.supports.len(){return Err("GLUT support shape mismatch".into());}
   cells.extend(gate.cells.iter().map(|&i|i as u32));
   for pair in &gate.supports {supports.extend(pair.iter().map(|&s|s as u32));}
   offsets.push(cells.len() as u32);all.push(gate.all as u32);
 }
 let count=u32::try_from(relation.masks.len()).map_err(|_|"GLUT cell index overflow")?;
 let gates=all.len() as u32;
 let mut parity_offsets=vec![0u32];let mut parity_cells=Vec::new();let mut parity_marks=Vec::new();
 for (row,mark) in &relation.parity {
   parity_cells.extend(row.iter().map(|&i|i as u32));parity_offsets.push(parity_cells.len() as u32);parity_marks.push(u32::from(*mark));
 }
 let parity_rows=parity_marks.len() as u32;
 let p:Vec<u32>=relation.p.iter().map(|&i|i as u32).collect();
 let q:Vec<u32>=relation.q.iter().map(|&i|i as u32).collect();
 let plen=p.len() as u32;let qlen=q.len() as u32;
 let nwords=n.to_u32_digits();
 let resume=match std::env::var("GLUT_RESUME") {
   Ok(path)=>{
     let vectors=read_checkpoint(&path)?;
     for (saved,current) in vectors[..10].iter().zip([&nwords,&offsets,&cells,&supports,&all,&p,&q,&parity_offsets,&parity_cells,&parity_marks]) {
       if saved!=current{return Err("GLUT resume source or compiled relation mismatch".into());}
     }
     let extent=(count as usize).checked_mul((plen+qlen) as usize).ok_or("GLUT resume bank overflow")?;
     if vectors[10].len()!=count as usize || vectors[11].len()!=extent
       || vectors[12].len()!=(plen+qlen) as usize || vectors[13].len()!=(plen+qlen) as usize
       || vectors[14].len()!=4 || vectors[14][0]>plen+qlen || vectors[14][1]!=0 {
       return Err("GLUT resume carried-state shape mismatch".into());
     }
     if vectors[10].iter().zip(&relation.masks).any(|(&a,&root)|a==0 || a>3 || a&root!=a) {
       return Err("GLUT resume lost a source-root restriction".into());
     }
     for depth in 0..vectors[14][0] as usize {
       if vectors[13][depth]>=count || vectors[12][depth]>1 {
         return Err("GLUT resume decision ancestry mismatch".into());
       }
     }
     eprintln!("glut_resume path={path} source_equal=true relation_equal=true retained_depth={} decisions={} conflicts={}",vectors[14][0],vectors[14][2],vectors[14][3]);
     Some(vectors)
   },Err(_)=>None,
 };
 let mut selected=None;
 let devices=match std::env::var("GLUT_GPU_DEVICE") {
   Ok(value)=>{let device=value.parse::<usize>().map_err(|_|"invalid GLUT_GPU_DEVICE")?;
     if device>1{return Err("GLUT_GPU_DEVICE must be 0 or 1".into());}vec![device]},
   Err(_)=>vec![1usize,0],
 };
 for device in devices {if let Ok(ctx)=CudaContext::new(device){selected=Some((ctx,device));break;}}
 let (ctx,device)=selected.ok_or("GPU GLUT requires CUDA device 0 or 1")?;
 let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
 let module=ctx.load_module(compile_ptx(src).map_err(|e|format!("GLUT NVRTC: {e}"))?).map_err(|e|format!("GLUT load: {e}"))?;
 let propagate=module.load_function("glut_propagate").map_err(|e|format!("GLUT propagate: {e}"))?;
 let control=module.load_function("glut_control").map_err(|e|format!("GLUT control: {e}"))?;
 let parity=module.load_function("glut_parity").map_err(|e|format!("GLUT parity: {e}"))?;
 let interval=module.load_function("glut_interval").map_err(|e|format!("GLUT interval: {e}"))?;
 let holonomy=module.load_function("glut_holonomy").map_err(|e|format!("GLUT holonomy: {e}"))?;
 let quantum=module.load_function("glut_factor_phase").map_err(|e|format!("GLUT Quantum port: {e}"))?;
 let masks_port=module.load_function("glut_mask_carry").map_err(|e|format!("GLUT Masks port: {e}"))?;
 let mask_intervals=module.load_function("glut_mask_intervals").map_err(|e|format!("GLUT Masks interval port: {e}"))?;
 let mask_envelope=module.load_function("glut_mask_envelope").map_err(|e|format!("GLUT Masks envelope port: {e}"))?;
 let admission_read=module.load_function("glut_admission_read").map_err(|e|format!("GLUT Admission return: {e}"))?;
 let verify_port=module.load_function("glut_verify_carries").map_err(|e|format!("GLUT Verify port: {e}"))?;
 let transport=module.load_function("glut_carrier_transport").map_err(|e|format!("GLUT enclosing transport: {e}"))?;
 let prepare=module.load_function("membrane_prepare").map_err(|e|format!("GLUT source preparation: {e}"))?;
 let stream=ctx.default_stream();
 macro_rules! upload {($v:expr)=>{stream.clone_htod($v).map_err(|e|format!("GLUT upload: {e}"))?};}
 macro_rules! zeros {($t:ty,$len:expr)=>{stream.alloc_zeros::<$t>($len).map_err(|e|format!("GLUT allocation: {e}"))?};}
 let d_offsets=upload!(&offsets);let d_cells=upload!(&cells);let d_supports=upload!(&supports);let d_all=upload!(&all);
 let d_admission=upload!(&admission);let mut d_admission_receipt=zeros!(u32,2);
 let d_phase_offsets=upload!(&factor_phase.offsets);let d_phase_controls=upload!(&factor_phase.controls);
 let d_phase_values=upload!(&factor_phase.values);let d_phase_targets=upload!(&factor_phase.targets);
 let phase_gates=factor_phase.targets.len() as u32;let phase_cells=factor_phase.cells as u32;
 let mut d_phase_workspace=zeros!(u32,factor_phase.cells);let mut d_phase_receipt=zeros!(u32,4);
 let d_parity_offsets=upload!(&parity_offsets);let d_parity_cells=upload!(&parity_cells);let d_parity_marks=upload!(&parity_marks);
 let mut d_masks=upload!(resume.as_ref().map_or(&relation.masks,|v|&v[10]));let d_p=upload!(&p);let d_q=upload!(&q);
 let limbs=nwords.len() as u32;
 let d_expected=upload!(&nwords);let mut d_n=zeros!(u32,limbs as usize);
 let payload=crate::native_numeral::encode(&n.to_string());
 let payload_ops=crate::membrane_family::word_ops(&payload);let payload_len=payload_ops.len() as u32;
 let d_payload=upload!(&payload_ops);let mut d_root=zeros!(u32,limbs as usize);
 let mut d_hi=zeros!(u32,limbs as usize);let mut d_binding=zeros!(u32,3);
 let single=LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0};
 let signed_limbs=limbs as i32;
 let mut launch=stream.launch_builder(&prepare);
 launch.arg(&d_expected).arg(&d_payload).arg(&payload_len).arg(&signed_limbs)
  .arg(&mut d_n).arg(&mut d_root).arg(&mut d_hi).arg(&mut d_binding);
 unsafe{launch.launch(single)}.map_err(|e|format!("GLUT source preparation: {e}"))?;
 let binding=stream.clone_dtoh(&d_binding).map_err(|e|format!("GLUT source read: {e}"))?;
 if binding!=[n.bits() as u32,1,1]{return Err(format!("GLUT source binding refused: {binding:?}"));}
 eprintln!("membrane_source_binding device={device} cells={} payload_nodes={payload_len} shape_valid=true source_equal=true",binding[0]);
 let bank_len=(count as usize).checked_mul((plen+qlen) as usize).ok_or("GLUT bank extent overflow")?;
 let mut d_banks=if let Some(v)=&resume {upload!(&v[11])}else{zeros!(u32,bank_len)};
 let mut d_choices=if let Some(v)=&resume {upload!(&v[12])}else{zeros!(u32,(plen+qlen) as usize)};
 let mut d_stack=if let Some(v)=&resume {upload!(&v[13])}else{zeros!(u32,(plen+qlen) as usize)};
 let mut d_state=if let Some(v)=&resume {upload!(&v[14])}else{zeros!(u32,4)};
 let mut d_parents=zeros!(u32,count as usize);let mut d_turns=zeros!(u32,count as usize);
 let mut d_holonomy_stats=zeros!(u32,3);
 let mut d_masks_stats=zeros!(u32,2);let source_bits=n.bits() as u32;
 let mut d_interval_stats=zeros!(u32,2);
 let mut d_envelope_stats=zeros!(u32,2);
 let mut d_verify_carries=zeros!(u32,source_bits as usize+1);let mut d_verify_receipt=zeros!(u32,2);
 let carrier=vox_core::glut_system::GlutComponentProgram::compile()?;
 let carrier_word:String=carrier.collapsed.iter().collect();
 if crate::membrane_family::compile_dissolved_word(&carrier_word)?!=carrier_word {
   return Err("GLUT enclosing graph admission changed its compiled word".into());
 }
 if !carrier_word.starts_with("⊢∈")||!carrier_word.ends_with("∋⊡⊣")
   ||carrier.collapsed.iter().filter(|&&c|c=='∈').count()!=1 {
   return Err("GLUT device transport requires its compiled enclosing frame".into());
 }
 let transport_ops=crate::membrane_family::word_ops(&carrier_word);let transport_len=transport_ops.len() as u32;
 let body_begin=2u32;let body_end=transport_len-3;let prefix_begin=0u32;
 let d_transport_word=upload!(&transport_ops);
 let held=match resume.as_ref(){Some(v) if validate_enclosing_checkpoint(v,&transport_ops)?=>Some(v),_=>None};
 let mut d_transport_live=if let Some(v)=held {upload!(&v[16])}else{zeros!(u32,4*transport_ops.len())};
 let mut d_transport_bank=if let Some(v)=held {upload!(&v[17])}else{zeros!(u32,4*transport_ops.len())};
 let mut d_transport_state=if let Some(v)=held {upload!(&v[18])}else{zeros!(u32,10)};
 let mut d_transport_identity=zeros!(u64,2);
 let mut d_outp=zeros!(u32,limbs as usize);let mut d_outq=zeros!(u32,limbs as usize);
 eprintln!("glut_prepared device={device} bits={} cells={count} gates={gates} parity_rows={parity_rows} banks={} preparation_seconds={:.9} operations=shared_product,carry,source_constraints,admission_selectors device_bindings=propagate,dqi_syndrome,admission,mask_intervals,mask_carry,mask_envelope,phase_holonomy,interval_reject,bank,branch,restore,verify_carries,quantum_factor_phase,exact_product",n.bits(),plen+qlen,started.elapsed().as_secs_f64());
 eprintln!("glut_admission_binding device={device} source_bits={} geometry_rows={admission_rows} selector_cells=resident banked_with=product_carry",n.bits());
 eprintln!("glut_square_binding device={device} midpoint_cells={midpoint_cells} gap_cells={gap_cells} relation=a_squared_equals_N_plus_b_squared factor_frames=p_plus_b_equals_a,a_plus_b_equals_q shared_register=true");
 let execution=Instant::now();let mut report=Instant::now();let mut rounds=0u64;
 let prefix_end=if held.is_some(){0u32}else{body_begin};
 let mut launch=stream.launch_builder(&transport);
 launch.arg(&d_transport_word).arg(&transport_len).arg(&prefix_begin).arg(&prefix_end)
   .arg(&d_n).arg(&d_masks).arg(&mut d_transport_live).arg(&mut d_transport_bank)
   .arg(&mut d_transport_state).arg(&mut d_transport_identity);
 unsafe{launch.launch(single)}.map_err(|e|format!("GLUT enclosing frame open: {e}"))?;
 let seconds=std::env::var("MEMBRANE_RUN_SECONDS").ok().and_then(|s|s.parse::<u64>().ok()).unwrap_or(300);
 let mut checkpoint_saved=false;
 let cfg=LaunchConfig{grid_dim:(gates.div_ceil(128),1,1),block_dim:(128,1,1),shared_mem_bytes:0};
 loop {
   let mut flags=zeros!(u32,2);
   let mut launch=stream.launch_builder(&propagate);
   launch.arg(&d_offsets).arg(&d_cells).arg(&d_supports).arg(&d_all).arg(&gates).arg(&mut d_masks).arg(&mut flags);
   unsafe{launch.launch(cfg)}.map_err(|e|format!("GLUT propagation: {e}"))?;
   if parity_rows>0 {
     let mut launch=stream.launch_builder(&parity);
     launch.arg(&d_parity_offsets).arg(&d_parity_cells).arg(&d_parity_marks).arg(&parity_rows).arg(&mut d_masks).arg(&mut flags);
     let cfg=LaunchConfig{grid_dim:(parity_rows.div_ceil(128),1,1),block_dim:(128,1,1),shared_mem_bytes:0};
     unsafe{launch.launch(cfg)}.map_err(|e|format!("GLUT DQI syndrome: {e}"))?;
   }
   let observed=stream.clone_dtoh(&flags).map_err(|e|format!("GLUT flags: {e}"))?;
   rounds+=1;
   if observed[1]==0 && observed[0]==0 {
     let mut launch=stream.launch_builder(&admission_read);
     launch.arg(&d_admission).arg(&admission_rows).arg(&d_masks).arg(&mut flags).arg(&mut d_admission_receipt);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT Admission readout: {e}"))?;
     let mut launch=stream.launch_builder(&mask_intervals);
     launch.arg(&mut d_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_n).arg(&d_root).arg(&limbs).arg(&mut flags).arg(&mut d_interval_stats);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT Masks interval transport: {e}"))?;
     let mut launch=stream.launch_builder(&masks_port);
     launch.arg(&mut d_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_n).arg(&source_bits).arg(&mut flags).arg(&mut d_masks_stats);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT Masks carry: {e}"))?;
     let restricted=stream.clone_dtoh(&flags).map_err(|e|format!("GLUT Masks transport flags: {e}"))?;
     if restricted==[0,0] {
       let mut launch=stream.launch_builder(&mask_envelope);
       launch.arg(&mut d_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
         .arg(&d_n).arg(&source_bits).arg(&mut flags).arg(&mut d_envelope_stats);
       unsafe{launch.launch(single)}.map_err(|e|format!("GLUT Masks complete carry envelope: {e}"))?;
     }
     if parity_rows>0 {
       let mut launch=stream.launch_builder(&holonomy);
       launch.arg(&d_parity_offsets).arg(&d_parity_cells).arg(&d_parity_marks).arg(&parity_rows)
        .arg(&d_masks).arg(&count).arg(&mut d_parents).arg(&mut d_turns)
        .arg(&mut flags).arg(&mut d_holonomy_stats);
       unsafe{launch.launch(single)}.map_err(|e|format!("GLUT phase holonomy: {e}"))?;
     }
     let mut launch=stream.launch_builder(&interval);
     launch.arg(&d_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
      .arg(&d_n).arg(&d_root).arg(&limbs).arg(&mut flags);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT interval rejection: {e}"))?;
   }
   let after_masks=stream.clone_dtoh(&flags).map_err(|e|format!("GLUT Masks flags: {e}"))?;
   if after_masks[1]!=0 || after_masks[0]==0 {
     let mut launch=stream.launch_builder(&control);
     launch.arg(&mut d_masks).arg(&count).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
      .arg(&mut d_banks).arg(&mut d_choices).arg(&mut d_stack).arg(&mut d_state)
      .arg(&flags).arg(&d_n).arg(&limbs).arg(&mut d_outp).arg(&mut d_outq);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT control: {e}"))?;
   }
   let mut launch=stream.launch_builder(&transport);
   launch.arg(&d_transport_word).arg(&transport_len).arg(&body_begin).arg(&body_end)
     .arg(&d_n).arg(&d_masks).arg(&mut d_transport_live).arg(&mut d_transport_bank)
     .arg(&mut d_transport_state).arg(&mut d_transport_identity);
   unsafe{launch.launch(single)}.map_err(|e|format!("GLUT enclosing body transport: {e}"))?;
   let state=stream.clone_dtoh(&d_state).map_err(|e|format!("GLUT state: {e}"))?;
   if state[1]==1 {
     let mut launch=stream.launch_builder(&verify_port);
     launch.arg(&d_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen).arg(&d_n)
       .arg(&source_bits).arg(&mut d_verify_carries).arg(&mut d_verify_receipt);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT Verify carries: {e}"))?;
     let verified=stream.clone_dtoh(&d_verify_receipt).map_err(|e|format!("GLUT Verify readout: {e}"))?;
     if verified!=[1,source_bits]{return Err(format!("GLUT Verify ancestry refused: {verified:?}"));}
     eprintln!("glut_verify_return device={device} source_bits={source_bits} carry_checkpoints={} source_equal=true terminal_carry=0",source_bits+1);
     // Correlation's candidate stays in its resident masks. Quantum acts there
     // before any factor words cross the terminal boundary.
     let mut launch=stream.launch_builder(&quantum);
     launch.arg(&d_phase_offsets).arg(&d_phase_controls).arg(&d_phase_values).arg(&d_phase_targets)
       .arg(&phase_gates).arg(&factor_phase.width).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_masks).arg(&mut d_phase_workspace).arg(&phase_cells).arg(&mut d_phase_receipt);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT Quantum execution: {e}"))?;
     let phase=stream.clone_dtoh(&d_phase_receipt).map_err(|e|format!("GLUT Quantum readout: {e}"))?;
     if phase!=[1,phase_gates,0,1]{return Err(format!("GLUT Quantum return refused: {phase:?}"));}
     eprintln!("glut_quantum_return device={device} gates={} relative_sign=-1 workspace_clear=true factor_cells_preserved=true",phase[1]);
     let a=BigUint::new(stream.clone_dtoh(&d_outp).map_err(|e|format!("GLUT factor: {e}"))?);
     let b=BigUint::new(stream.clone_dtoh(&d_outq).map_err(|e|format!("GLUT factor: {e}"))?);
     let readout_seconds=execution.elapsed().as_secs_f64();
     if &a*&b!=*n{return Err("GLUT host product closure refused".into());}
     if a<=BigUint::from(1u32)||a>=*n||n%&a!=BigUint::from(0u32)||n/&a!=b {
       return Err("GLUT divisor/derived quotient boundary refused".into());
     }
     let mut launch=stream.launch_builder(&transport);
     launch.arg(&d_transport_word).arg(&transport_len).arg(&body_end).arg(&transport_len)
       .arg(&d_n).arg(&d_masks).arg(&mut d_transport_live).arg(&mut d_transport_bank)
       .arg(&mut d_transport_state).arg(&mut d_transport_identity);
     unsafe{launch.launch(single)}.map_err(|e|format!("GLUT enclosing return: {e}"))?;
     let returned=stream.clone_dtoh(&d_transport_state).map_err(|e|format!("GLUT enclosing readout: {e}"))?;
     if returned[1]!=0||returned[2]!=1||returned[3]!=0||returned[7]!=1||returned[4]==0 {
       return Err(format!("GLUT enclosing bank return refused: {returned:?}"));
     }
     eprintln!("glut_enclosing_return device={device} register={} restored={} exposed=0 resident_identity=preserved fixed=true",returned[0],returned[4]);
     let execution_seconds=execution.elapsed().as_secs_f64();let saving=Instant::now();
     eprintln!("glut_execution_clocks device={device} trigger_to_readout_seconds={readout_seconds:.9} trigger_to_verified_factor_seconds={execution_seconds:.9}");
     let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
       .map_err(|e|format!("GLUT return clock: {e}"))?.as_nanos();
     let return_path=std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
       .join("measurements/membrane_rewiring_20261014/returns")
       .join(format!("{stamp}_{}.glut",std::process::id()));
     write_device_state(&return_path,&stream,
       &[&nwords,&offsets,&cells,&supports,&all,&p,&q,&parity_offsets,&parity_cells,&parity_marks],
       &[&d_masks,&d_banks,&d_choices,&d_stack,&d_state],&transport_ops,
       &[&d_transport_live,&d_transport_bank,&d_transport_state],
       Some(&[&d_phase_receipt,&d_phase_workspace,&d_verify_receipt,&d_verify_carries]))?;
     eprintln!("glut_return path={} source_factor_carry_banks=retained enclosing_observations=retained quantum_workspace=retained return_storage_seconds={:.9}",return_path.display(),saving.elapsed().as_secs_f64());
     return Ok(format!("CUDA device={device}; GLUT shared multiplier; rounds={rounds}; decisions={}; conflicts={}; execution_seconds={execution_seconds:.9}\n{n} = {a} x {b} (GPU product closure verified)",state[2],state[3]));
   }
   if state[1]!=0{return Err(format!("GLUT terminal refusal state={state:?}; rounds={rounds}"));}
   if report.elapsed().as_secs()>=1 {
     let phase=stream.clone_dtoh(&d_holonomy_stats).map_err(|e|format!("GLUT phase counters: {e}"))?;
     let masks=stream.clone_dtoh(&d_masks_stats).map_err(|e|format!("GLUT Masks counters: {e}"))?;
     let ranges=stream.clone_dtoh(&d_interval_stats).map_err(|e|format!("GLUT Masks interval counters: {e}"))?;
     let envelope=stream.clone_dtoh(&d_envelope_stats).map_err(|e|format!("GLUT Masks envelope counters: {e}"))?;
     let admission=stream.clone_dtoh(&d_admission_receipt).map_err(|e|format!("GLUT Admission counters: {e}"))?;
     eprintln!("glut_progress device={device} rounds={rounds} depth={} decisions={} conflicts={} phase_edges={} odd_loop_conflicts={} mask_diagonals={} mask_forced={} range_transports={} range_forced={} envelope_diagonals={} envelope_forced={} admitted_geometries={} selected_geometries={} execution_seconds={:.9}",state[0],state[2],state[3],phase[0],phase[1],masks[0],masks[1],ranges[0],ranges[1],envelope[0],envelope[1],admission[0],admission[1],execution.elapsed().as_secs_f64());report=Instant::now();
   }
   if !checkpoint_saved && execution.elapsed().as_secs()>=seconds {
     let execution_seconds=execution.elapsed().as_secs_f64();
     let checkpoint_started=Instant::now();
     if let Ok(path)=std::env::var("GLUT_CHECKPOINT") {
       write_device_state(std::path::Path::new(&path),&stream,
         &[&nwords,&offsets,&cells,&supports,&all,&p,&q,&parity_offsets,&parity_cells,&parity_marks],
         &[&d_masks,&d_banks,&d_choices,&d_stack,&d_state],&transport_ops,
         &[&d_transport_live,&d_transport_bank,&d_transport_state],None)?;
       eprintln!("glut_checkpoint path={path} correlated_banks=retained checkpoint_seconds={:.9}",checkpoint_started.elapsed().as_secs_f64());
     }
     eprintln!("glut_continuing device={device} rounds={rounds} state={state:?} execution_seconds={execution_seconds:.9}");
     checkpoint_saved=true;
   }
 }
}

#[cfg(test)]
mod tests {
 use super::*;

 #[test]
 #[ignore="requires CUDA; 128-bit enclosing carrier return and payload identity control"]
 fn enclosing_carrier_return_matches_native_observation_transport() {
   let n=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only")
     .trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);
   let bits:Vec<_>=(0..128).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
   let relation=vox_core::glut_system::prepare_device_correlation(&bits);assert!(!relation.empty);
   let program=vox_core::glut_system::GlutComponentProgram::compile().unwrap();
   let word:String=program.collapsed.iter().collect();let expected=vox_core::phase_word::execute(&word,&()).unwrap();
   assert_eq!(crate::membrane_family::compile_dissolved_word(&word).unwrap(),word);
   assert!(expected.exposed==0&&expected.restored>0);
   let ops=crate::membrane_family::word_ops(&word);let length=ops.len() as u32;
   let ctx=CudaContext::new(1).unwrap();let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_carrier_transport").unwrap();
   let d_word=stream.clone_htod(&ops).unwrap();let source=stream.clone_htod(&n.to_u32_digits()).unwrap();
   let payload=stream.clone_htod(&relation.masks).unwrap();let replacement=stream.clone_htod(&relation.masks).unwrap();
   let nwords=n.to_u32_digits();let mut offsets=vec![0u32];let mut cells=Vec::new();let mut supports=Vec::new();let mut rows=Vec::new();
   for gate in &relation.gates {
     cells.extend(gate.cells.iter().map(|&v|v as u32));
     for pair in &gate.supports {supports.extend(pair.iter().map(|&v|v as u32));}
     offsets.push(cells.len() as u32);rows.push(gate.all as u32);
   }
   let p:Vec<u32>=relation.p.iter().map(|&v|v as u32).collect();let q:Vec<u32>=relation.q.iter().map(|&v|v as u32).collect();
   let mut parity_offsets=vec![0u32];let mut parity_cells=Vec::new();let mut parity_marks=Vec::new();
   for (ids,mark) in &relation.parity {parity_cells.extend(ids.iter().map(|&v|v as u32));parity_offsets.push(parity_cells.len() as u32);parity_marks.push(u32::from(*mark));}
   let source_fields:[&[u32];10]=[&nwords,&offsets,&cells,&supports,&rows,&p,&q,&parity_offsets,&parity_cells,&parity_marks];
   let decision_banks=stream.alloc_zeros::<u32>(relation.masks.len()*(p.len()+q.len())).unwrap();
   let choices=stream.alloc_zeros::<u32>(p.len()+q.len()).unwrap();let stack=stream.alloc_zeros::<u32>(p.len()+q.len()).unwrap();
   let decision_state=stream.alloc_zeros::<u32>(4).unwrap();
   for substitute in [false,true] {
     let mut live=stream.alloc_zeros::<u32>(4*ops.len()).unwrap();let mut bank=stream.alloc_zeros::<u32>(4*ops.len()).unwrap();
     let mut state=stream.alloc_zeros::<u32>(10).unwrap();let mut identity=stream.alloc_zeros::<u64>(2).unwrap();
     for (begin,end) in [(0u32,2u32),(2,length-3),(length-3,length)] {
       let selected=if substitute&&end==length {&replacement}else{&payload};
       let mut launch=stream.launch_builder(&kernel);
       launch.arg(&d_word).arg(&length).arg(&begin).arg(&end).arg(&source).arg(selected)
         .arg(&mut live).arg(&mut bank).arg(&mut state).arg(&mut identity);
       unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
       if end==length-3&&!substitute {
         let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
         let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
           .join("measurements/membrane_rewiring_20261014/held_returns")
           .join(format!("transport_128_{stamp}.glut"));
         write_device_state(&path,&stream,&source_fields,
           &[&payload,&decision_banks,&choices,&stack,&decision_state],&ops,&[&live,&bank,&state],None).unwrap();
         let saved=read_checkpoint(path.to_str().unwrap()).unwrap();
         for (index,expected) in source_fields.iter().enumerate(){assert_eq!(saved[index],*expected);}
         assert_eq!(saved[10],relation.masks);
         assert!(validate_enclosing_checkpoint(&saved,&ops).unwrap());
         println!("enclosing_held_state path={} format=GLUTGPU3 fields=19 source_equal=true serialized_observations_equal=true",path.display());
         let mut malformed=saved.clone();malformed[17][0]=1;
         assert!(validate_enclosing_checkpoint(&malformed,&ops).is_err());
         let mut resumed_live=stream.clone_htod(&saved[16]).unwrap();let mut resumed_bank=stream.clone_htod(&saved[17]).unwrap();
         let mut resumed_state=stream.clone_htod(&saved[18]).unwrap();let mut rebound=stream.alloc_zeros::<u64>(2).unwrap();
         for (start,stop) in [(0u32,0u32),(length-3,length)] {
           let mut launch=stream.launch_builder(&kernel);
           launch.arg(&d_word).arg(&length).arg(&start).arg(&stop).arg(&source).arg(&replacement)
             .arg(&mut resumed_live).arg(&mut resumed_bank).arg(&mut resumed_state).arg(&mut rebound);
           unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
         }
         let resumed=stream.clone_dtoh(&resumed_state).unwrap();
         assert_eq!(resumed[0],expected.register as u32);assert_eq!(resumed[3],0);assert_eq!(resumed[7],1);
         assert_eq!(resumed[4],expected.restored as u32);
         println!("enclosing_reentry_stage bits=128 device=1 held_observations=retained source_payload_handles=rebound malformed_deposit=rejected restored={}",resumed[4]);
       }
     }
     let state=stream.clone_dtoh(&state).unwrap();
     if substitute {assert!(state[3]>0);assert_eq!(state[7],0);}else {
       assert_eq!(state[0],expected.register as u32);assert_eq!(state[1],0);assert_eq!(state[2],1);
       assert_eq!(state[3],expected.exposed as u32);assert_eq!(state[4],expected.restored as u32);
       assert_eq!(state[5],expected.deposits as u32);assert_eq!(state[6],expected.inert as u32);
       assert_eq!(state[7],1);assert_eq!(state[8],expected.cleared as u32);
       let live=stream.clone_dtoh(&live).unwrap();
       assert_eq!(live.iter().filter(|&&v|v!=0).count(),expected.surviving.len());
     }
     assert_eq!(stream.clone_dtoh(&payload).unwrap(),relation.masks);
     assert_eq!(BigUint::new(stream.clone_dtoh(&source).unwrap()),n);
     println!("enclosing_return_stage bits=128 device=1 substitute={substitute} register={} restored={} exposed={} released={} source_and_payload_preserved=true",state[0],state[4],state[3],state[7]);
   }
 }

 #[test]
 #[ignore="requires CUDA; 128-bit shared midpoint-square relation wiring control"]
 fn square_coordinates_transform_the_same_resident_factor_relation() {
   let prepared=Instant::now();
   let n=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only")
     .trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);
   let bits:Vec<_>=(0..128).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
   let relation=vox_core::glut_system::prepare_device_correlation(&bits);assert!(!relation.empty);
   assert_eq!(relation.midpoint.len(),64);assert_eq!(relation.gap.len(),64);
   assert!(relation.midpoint.iter().chain(&relation.gap).all(|&cell|cell<relation.masks.len()));
   let selected=relation.midpoint.iter().enumerate().find(|&(_, &cell)|relation.masks[cell]==3)
     .map(|(bit,&cell)|(bit,cell)).expect("source has a retained square-coordinate alternative");
   let ctx=CudaContext::new(1).unwrap();let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_propagate").unwrap();
   let mut offsets=vec![0u32];let mut cells=Vec::new();let mut supports=Vec::new();let mut all=Vec::new();
   for gate in &relation.gates {
     cells.extend(gate.cells.iter().map(|&cell|cell as u32));
     for pair in &gate.supports {supports.extend(pair.iter().map(|&value|value as u32));}
     offsets.push(cells.len() as u32);all.push(gate.all as u32);
   }
   let offsets=stream.clone_htod(&offsets).unwrap();let cells=stream.clone_htod(&cells).unwrap();
   let supports=stream.clone_htod(&supports).unwrap();let all=stream.clone_htod(&all).unwrap();
   let gates=relation.gates.len() as u32;let mut held=0;let mut changed=0;
   let preparation_seconds=prepared.elapsed().as_secs_f64();let execution=Instant::now();
   for bit in [0u32,1] {
     let mut input=relation.masks.clone();input[selected.1]=1u32<<bit;
     let mut masks=stream.clone_htod(&input).unwrap();let mut settled=false;
     for _ in 0..=2*input.len() {
       let mut flags=stream.alloc_zeros::<u32>(2).unwrap();
       let mut launch=stream.launch_builder(&kernel);
       launch.arg(&offsets).arg(&cells).arg(&supports).arg(&all).arg(&gates).arg(&mut masks).arg(&mut flags);
       unsafe{launch.launch(LaunchConfig{grid_dim:(gates.div_ceil(128),1,1),block_dim:(128,1,1),shared_mem_bytes:0})}.unwrap();
       let flags=stream.clone_dtoh(&flags).unwrap();
       if flags[1]!=0 {settled=true;break;}
       if flags[0]==0 {
         held+=1;let returned=stream.clone_dtoh(&masks).unwrap();
         let changes=returned.iter().zip(&input).filter(|(a,b)|a!=b).count();
         assert!(changes>0,"square-coordinate restriction never left its selected cell");changed+=changes;
         assert!(returned.iter().zip(&relation.masks).all(|(&a,&root)|a!=0&&a&root==a));
         println!("square_stage bits=128 device=1 midpoint_bit={} value={bit} shared_gates={gates} changed_cells={changes} source_root_preserved=true",selected.0);
         settled=true;break;
       }
     }
     assert!(settled,"square transport exceeded its information-height bound");
   }
   assert!(held>0&&changed>0);
   println!("square_stage_timing preparation_seconds={preparation_seconds:.9} control_execution_seconds={:.9} producing_arm=none",execution.elapsed().as_secs_f64());
 }

 #[test]
 #[ignore="requires CUDA; 128-bit source-derived Admission family wiring control"]
 fn admission_selectors_execute_and_return_in_the_same_device_cells() {
   let n=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only")
     .trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);
   let bits:Vec<_>=(0..128).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
   let relation=vox_core::glut_system::prepare_device_admission(&bits);assert!(!relation.empty);
   let expected_labels=vox_core::glut_system::source_factor_geometries(128);
   assert_eq!(relation.admission.iter().map(|&(p,q,_)|(p,q)).collect::<Vec<_>>(),expected_labels);
   let ctx=CudaContext::new(1).unwrap();let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_propagate").unwrap();
   let mut offsets=vec![0u32];let mut cells=Vec::new();let mut supports=Vec::new();let mut all=Vec::new();
   for gate in &relation.gates {
     cells.extend(gate.cells.iter().map(|&cell|cell as u32));
     for pair in &gate.supports {supports.extend(pair.iter().map(|&value|value as u32));}
     offsets.push(cells.len() as u32);all.push(gate.all as u32);
   }
   let d_offsets=stream.clone_htod(&offsets).unwrap();let d_cells=stream.clone_htod(&cells).unwrap();
   let d_supports=stream.clone_htod(&supports).unwrap();let d_all=stream.clone_htod(&all).unwrap();
   let gates=all.len() as u32;
   for (pw,qw,admitted) in [(64usize,64usize,true),(64,65,true),(63,65,true),(2,65,false)] {
     let basis=|width:usize,shift:usize|((&n>>shift)&((BigUint::from(1u32)<<width)-1u32))
       |(BigUint::from(1u32)<<(width-1))|BigUint::from(1u32);
     let a=basis(pw,0);let b=basis(qw,31);
     let mut masks=relation.masks.clone();
     for (ids,value) in [(&relation.p,&a),(&relation.q,&b)] {
       for (i,&cell) in ids.iter().enumerate(){masks[cell]&=1u32<<u32::from(value.bit(i as u64));}
     }
     let mut device_masks=stream.clone_htod(&masks).unwrap();let mut finished=false;
     // Each gate intersection removes a bit. This bound is the finite carrier's
     // information height, with no factor decisions or candidate enumeration.
     for _ in 0..=2*masks.len() {
       let mut flags=stream.alloc_zeros::<u32>(2).unwrap();
       let mut launch=stream.launch_builder(&kernel);
       launch.arg(&d_offsets).arg(&d_cells).arg(&d_supports).arg(&d_all).arg(&gates)
         .arg(&mut device_masks).arg(&mut flags);
       unsafe{launch.launch(LaunchConfig{grid_dim:(gates.div_ceil(128),1,1),block_dim:(128,1,1),shared_mem_bytes:0})}.unwrap();
       let flags=stream.clone_dtoh(&flags).unwrap();
       if flags[1]!=0 {assert!(!admitted);finished=true;break;}
       if flags[0]==0 {
         assert!(admitted);let returned=stream.clone_dtoh(&device_masks).unwrap();
         for &(p,q,selector) in &relation.admission {
           assert_eq!(returned[selector],if (p,q)==(pw,qw){2}else{1},"label=({p},{q}) basis=({pw},{qw})");
         }
         for &cell in relation.p.iter().chain(&relation.q){assert_eq!(returned[cell],masks[cell]);}
         finished=true;break;
       }
     }
     assert!(finished,"Admission exceeded its information-height bound");
     println!("admission_stage bits=128 device=1 p_width={pw} q_width={qw} admitted={admitted} selector_labels_preserved=true source_boundary=N-only");
   }
   // Exercise bank/clear/re-entry on the COMPLETE executing relation. Width
   // selectors and all internal carries must return with their original IDs.
   let resident=vox_core::glut_system::prepare_device_correlation(&bits);assert!(!resident.empty);
   assert_eq!(resident.admission.iter().map(|&(p,q,_)|(p,q)).collect::<Vec<_>>(),expected_labels);
   let original=resident.masks;let count=original.len() as u32;
   let p:Vec<u32>=resident.p.iter().map(|&cell|cell as u32).collect();
   let q:Vec<u32>=resident.q.iter().map(|&cell|cell as u32).collect();
   let plen=p.len() as u32;let qlen=q.len() as u32;let limbs=4u32;
   let d_p=stream.clone_htod(&p).unwrap();let d_q=stream.clone_htod(&q).unwrap();
   let d_n=stream.clone_htod(&n.to_u32_digits()).unwrap();
   let mut masks=stream.clone_htod(&original).unwrap();
   let mut banks=stream.alloc_zeros::<u32>(original.len()).unwrap();
   let mut choices=stream.alloc_zeros::<u32>(1).unwrap();let mut stack=stream.alloc_zeros::<u32>(1).unwrap();
   let mut state=stream.alloc_zeros::<u32>(4).unwrap();
   let mut outp=stream.alloc_zeros::<u32>(4).unwrap();let mut outq=stream.alloc_zeros::<u32>(4).unwrap();
   let control=module.load_function("glut_control").unwrap();
   let mut selected=None;
   for conflict in [false,true] {
     if conflict {stream.memset_zeros(&mut masks).unwrap();}
     let flags=stream.clone_htod(&[0u32,u32::from(conflict)]).unwrap();
     let mut launch=stream.launch_builder(&control);
     launch.arg(&mut masks).arg(&count).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&mut banks).arg(&mut choices).arg(&mut stack).arg(&mut state)
       .arg(&flags).arg(&d_n).arg(&limbs).arg(&mut outp).arg(&mut outq);
     unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
     assert_eq!(stream.clone_dtoh(&banks).unwrap(),original);
     let cell=stream.clone_dtoh(&stack).unwrap()[0] as usize;
     if let Some(previous)=selected {assert_eq!(cell,previous);}else{selected=Some(cell);}
     let returned=stream.clone_dtoh(&masks).unwrap();
     for i in 0..original.len(){assert_eq!(returned[i],if i==cell {if conflict {1}else{2}}else{original[i]});}
     for &(_,_,selector) in &resident.admission {assert_eq!(returned[selector],original[selector],"Admission selector overlapped a factor decision");}
     let state=stream.clone_dtoh(&state).unwrap();assert_eq!(state[0],1);assert_eq!(state[1],0);
   }
   println!("admission_banked_return bits=128 device=1 selector_cells=retained source_carry_cells=retained cleared_live_masks=recovered proper_divisor_production=false");
 }

 #[test]
 #[ignore="requires CUDA; 128-bit complete carry-envelope wiring control"]
 fn masks_complete_envelope_matches_canonical_terminal_carry() {
   let n=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only")
     .trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);
   let bits:Vec<_>=(0..128).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
   let ctx=CudaContext::new(1).unwrap();let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_mask_envelope").unwrap();
   let p:Vec<u32>=(0..64).collect();let q:Vec<u32>=(64..192).collect();
   let d_p=stream.clone_htod(&p).unwrap();let d_q=stream.clone_htod(&q).unwrap();
   let d_n=stream.clone_htod(&n.to_u32_digits()).unwrap();
   let plen=64u32;let qlen=128u32;let width=128u32;
   let mut changed=0;let mut rejected=0;
   for sample in 0..16u32 {
     let mut masks=vec![3u32;192];masks[0]=2;masks[64]=2;masks[63]=2;
     for i in 1..=4 {masks[i]=1+u32::from(n.bit((i+sample as usize) as u64));}
     if sample&1!=0 {masks[191]=2;}
     let expected=vox_core::glut_system::device_carry_envelope_step(&bits,&masks[..64],&masks[64..]);
     let mut device_masks=stream.clone_htod(&masks).unwrap();
     let mut flags=stream.alloc_zeros::<u32>(2).unwrap();let mut stats=stream.alloc_zeros::<u32>(2).unwrap();
     let mut launch=stream.launch_builder(&kernel);
     launch.arg(&mut device_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_n).arg(&width).arg(&mut flags).arg(&mut stats);
     unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
     let flags=stream.clone_dtoh(&flags).unwrap();
     match expected {
       None=>{assert_eq!(flags[1],1,"sample={sample}");rejected+=1;},
       Some((left,right))=>{
         assert_eq!(flags[1],0,"sample={sample}");
         let expected:Vec<_>=left.into_iter().chain(right).collect();
         assert_eq!(stream.clone_dtoh(&device_masks).unwrap(),expected,"sample={sample}");
         if expected!=masks {assert_eq!(flags[0],1);changed+=1;}
       }
     }
   }
   assert!(changed>0&&rejected>0);
   println!("envelope_stage bits=128 device=1 samples=16 changed={changed} rejected={rejected} canonical_masks_equal=true terminal_carry=0 reference_factor_inputs=0");
 }

 #[test]
 #[ignore="requires CUDA; 128-bit interval transport compared with the resident Masks operation"]
 fn masks_interval_transport_matches_canonical_source_bound_extrema() {
   let n=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only")
     .trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);
   let bits:Vec<_>=(0..128).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
   let root=vox_core::morphism_factor::tape_to_limbs(&vox_core::morphism_factor::isqrt(&bits),4);
   let ctx=CudaContext::new(1).unwrap();let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_mask_intervals").unwrap();
   let p:Vec<u32>=(0..64).collect();let q:Vec<u32>=(64..192).collect();
   let d_p=stream.clone_htod(&p).unwrap();let d_q=stream.clone_htod(&q).unwrap();
   let d_n=stream.clone_htod(&n.to_u32_digits()).unwrap();let d_root=stream.clone_htod(&root).unwrap();
   let plen=64u32;let qlen=128u32;let limbs=4u32;
   let mut changed=0;let mut rejected=0;
   for sample in 0..16u32 {
     let mut masks=vec![3u32;192];masks[0]=2;masks[64]=2;
     if sample&1!=0 {masks[63]=2;}
     for i in 1..=4 {
       masks[i]=1+u32::from(n.bit((i+sample as usize) as u64));
     }
     // A high cofactor pin deliberately tests empty interval reconvergence.
     if sample&2!=0 {masks[191]=2;}
     let expected=vox_core::glut_system::device_mask_interval_step(&bits,&masks[..64],&masks[64..]);
     let mut device_masks=stream.clone_htod(&masks).unwrap();
     let mut flags=stream.alloc_zeros::<u32>(2).unwrap();let mut stats=stream.alloc_zeros::<u32>(2).unwrap();
     let mut launch=stream.launch_builder(&kernel);
     launch.arg(&mut device_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_n).arg(&d_root).arg(&limbs).arg(&mut flags).arg(&mut stats);
     unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
     let flags=stream.clone_dtoh(&flags).unwrap();
     match expected {
       None=>{assert_eq!(flags[1],1,"sample={sample}");rejected+=1;},
       Some((left,right))=>{
         assert_eq!(flags[1],0,"sample={sample}");
         let expected:Vec<_>=left.into_iter().chain(right).collect();
         assert_eq!(stream.clone_dtoh(&device_masks).unwrap(),expected,"sample={sample}");
         if expected!=masks {assert_eq!(flags[0],1);changed+=1;}
       }
     }
   }
   assert!(changed>0&&rejected>0);
   println!("interval_stage bits=128 device=1 samples=16 changed={changed} rejected={rejected} canonical_masks_equal=true reference_factor_inputs=0");
 }

 #[test]
 #[ignore="requires CUDA; 128-bit Masks port rewiring control"]
 fn masks_port_forces_one_corner_and_retains_unresolved_diagonal() {
   let n=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only")
     .trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);assert!(n.bit(0));
   let ctx=CudaContext::new(1).unwrap();let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_mask_carry").unwrap();
   let p:Vec<u32>=(0..64).collect();let q:Vec<u32>=(64..192).collect();
   let d_p=stream.clone_htod(&p).unwrap();let d_q=stream.clone_htod(&q).unwrap();
   let d_n=stream.clone_htod(&n.to_u32_digits()).unwrap();
   let plen=64u32;let qlen=128u32;let bits=128u32;
   for p_bit in [0u32,1] {
     let mut masks=vec![3u32;192];masks[0]=2;masks[64]=2;masks[1]=1+p_bit;
     let mut device_masks=stream.clone_htod(&masks).unwrap();
     let mut flags=stream.alloc_zeros::<u32>(2).unwrap();
     let mut stats=stream.alloc_zeros::<u32>(2).unwrap();
     let mut launch=stream.launch_builder(&kernel);
     launch.arg(&mut device_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_n).arg(&bits).arg(&mut flags).arg(&mut stats);
     unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
     masks[65]=1+(u32::from(n.bit(1))^p_bit);
     assert_eq!(stream.clone_dtoh(&device_masks).unwrap(),masks);
     assert_eq!(stream.clone_dtoh(&flags).unwrap(),[1,0]);
     assert_eq!(stream.clone_dtoh(&stats).unwrap(),[2,1]);
     println!("masks_stage bits=128 device=1 source_boundary=N-only p_bit={p_bit} visited_diagonals=2 forced_cells=1 unresolved_diagonal=retained");
   }
   let verify=module.load_function("glut_verify_carries").unwrap();
   // N-only identity-product control checks the source/carry return. Proper
   // divisor admission is checked independently by the Quantum sign port.
   let mut masks=vec![1u32;192];masks[0]=2;
   for i in 0..128 {masks[64+i]=if n.bit(i as u64){2}else{1};}
   for corrupt in [false,true] {
     if corrupt {masks[65]=3-masks[65];}
     let device_masks=stream.clone_htod(&masks).unwrap();
     let mut carries=stream.alloc_zeros::<u32>(129).unwrap();
     let mut receipt=stream.alloc_zeros::<u32>(2).unwrap();
     let mut launch=stream.launch_builder(&verify);
     launch.arg(&device_masks).arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen)
       .arg(&d_n).arg(&bits).arg(&mut carries).arg(&mut receipt);
     unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
     let receipt=stream.clone_dtoh(&receipt).unwrap();
     if corrupt {assert_eq!(receipt[0],0);}else {
       assert_eq!(receipt,[1,128]);assert_eq!(stream.clone_dtoh(&carries).unwrap(),vec![0;129]);
     }
     println!("verify_stage bits=128 device=1 identity_product_control=true corrupt={corrupt} source_carry_equal={}",receipt[0]);
   }
 }

 #[test]
 #[ignore="requires CUDA; N-only 128-bit Quantum port rewiring control"]
 fn quantum_port_executes_canonical_gates_on_retained_factor_cells() {
   let source=include_str!("../measurements/membrane_rewiring_20261014/inputs/balanced_128.n-only");
   let n=source.trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),128);
   let bits:Vec<_>=(0..n.bits()).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
   let program=FactorPhaseProgram::prepare(&bits).unwrap();
   let ctx=CudaContext::new(1).expect("CUDA device 1");let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_factor_phase").unwrap();
   let offsets=stream.clone_htod(&program.offsets).unwrap();
   let controls=stream.clone_htod(&program.controls).unwrap();
   let values=stream.clone_htod(&program.values).unwrap();
   let targets=stream.clone_htod(&program.targets).unwrap();
   let width=program.width;let cells=program.cells as u32;let gates=program.targets.len() as u32;
   let plen=width;let qlen=width;
   let p:Vec<_>=(0..width).collect();let q:Vec<_>=(width..2*width).collect();
   let d_p=stream.clone_htod(&p).unwrap();let d_q=stream.clone_htod(&q).unwrap();
   // These are N-derived basis addresses, never factors supplied to production.
   // The independent product predicate checks the oracle's sign action.
   for (a,b) in [(BigUint::from(1u32),n.clone()),
     (BigUint::from(2u32),&n>>1usize),
     (&n&((BigUint::from(1u32)<<64usize)-1u32),&n>>64usize)] {
     let masks:Vec<u32>=(0..width).map(|i|if a.bit(i as u64){2}else{1})
       .chain((0..width).map(|i|if b.bit(i as u64){2}else{1})).collect();
     let d_masks=stream.clone_htod(&masks).unwrap();
     let mut workspace=stream.alloc_zeros::<u32>(program.cells).unwrap();
     let mut receipt=stream.alloc_zeros::<u32>(4).unwrap();
     let mut launch=stream.launch_builder(&kernel);
     launch.arg(&offsets).arg(&controls).arg(&values).arg(&targets).arg(&gates).arg(&width)
       .arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen).arg(&d_masks).arg(&mut workspace)
       .arg(&cells).arg(&mut receipt);
     unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
     let marked=&a*&b==n&&a>BigUint::from(1u32)&&b>BigUint::from(1u32);
     let receipt=stream.clone_dtoh(&receipt).unwrap();
     assert_eq!(receipt,[u32::from(marked),gates,0,1]);
     assert_eq!(stream.clone_dtoh(&d_masks).unwrap(),masks);
     println!("quantum_stage bits=128 device=1 gates={gates} phase={} workspace_clear=true factor_cells_preserved=true source_boundary=N-only",receipt[0]);
   }
   // A signed control on the same source-sized workspace checks that the
   // device really applies a phase flip as well as controlled toggles.
   let offsets=stream.clone_htod(&[0u32,0,1,1]).unwrap();
   let controls=stream.clone_htod(&[0u32]).unwrap();let values=stream.clone_htod(&[1u32]).unwrap();
   let targets=stream.clone_htod(&[0u32,u32::MAX,0]).unwrap();let three=3u32;
   let d_masks=stream.clone_htod(&vec![1u32;(2*width) as usize]).unwrap();
   let mut workspace=stream.alloc_zeros::<u32>(program.cells).unwrap();
   let mut receipt=stream.alloc_zeros::<u32>(4).unwrap();
   let mut launch=stream.launch_builder(&kernel);
   launch.arg(&offsets).arg(&controls).arg(&values).arg(&targets).arg(&three).arg(&width)
     .arg(&d_p).arg(&plen).arg(&d_q).arg(&qlen).arg(&d_masks).arg(&mut workspace)
     .arg(&cells).arg(&mut receipt);
   unsafe{launch.launch(LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0})}.unwrap();
   assert_eq!(stream.clone_dtoh(&receipt).unwrap(),[1,3,0,1]);
 }

 #[test]
 #[ignore = "requires CUDA; phase controls are embedded in the three 300-bit prepared sources"]
 fn device_holonomy_rejects_odd_loops_and_preserves_even_loops() {
   let ctx=CudaContext::new(0).expect("CUDA device 0");let stream=ctx.default_stream();
   let src=format!("{}\n{}",include_str!("membrane_family_gpu.cuh"),include_str!("gpu_glut.cuh"));
   let module=ctx.load_module(compile_ptx(src).unwrap()).unwrap();
   let kernel=module.load_function("glut_holonomy").unwrap();
   for input in [include_str!("../measurements/membrane_gpu_wiring_20261013/300bit/case_1.n-only"),
     include_str!("../measurements/membrane_gpu_wiring_20261013/300bit/case_2.n-only"),
     include_str!("../measurements/membrane_gpu_wiring_20261013/300bit/case_3.n-only")] {
     let n=input.trim().parse::<BigUint>().unwrap();assert_eq!(n.bits(),300);
     let bits:Vec<_>=(0..n.bits()).map(|i|if n.bit(i){'⊥'}else{'⊤'}).collect();
     let relation=vox_core::glut_system::prepare_device_correlation(&bits);
     let free:Vec<u32>=relation.masks.iter().enumerate().filter(|(_,v)|**v==3)
       .map(|(i,_)|i as u32).take(64).collect();assert_eq!(free.len(),64);
     let offsets:Vec<u32>=(0..=64).map(|i|2*i).collect();
     let cells:Vec<u32>=(0..64).flat_map(|i|[free[i],free[(i+1)%64]]).collect();
     let d_offsets=stream.clone_htod(&offsets).unwrap();let d_cells=stream.clone_htod(&cells).unwrap();
     let d_masks=stream.clone_htod(&relation.masks).unwrap();let count=relation.masks.len() as u32;
     for odd in [false,true] {
       let mut marks=vec![0u32;64];marks[63]=u32::from(odd);
       let d_marks=stream.clone_htod(&marks).unwrap();let rows=64u32;
       let mut parents=stream.alloc_zeros::<u32>(count as usize).unwrap();
       let mut turns=stream.alloc_zeros::<u32>(count as usize).unwrap();
       let mut flags=stream.alloc_zeros::<u32>(2).unwrap();let mut stats=stream.alloc_zeros::<u32>(3).unwrap();
       let mut launch=stream.launch_builder(&kernel);
       launch.arg(&d_offsets).arg(&d_cells).arg(&d_marks).arg(&rows).arg(&d_masks).arg(&count)
         .arg(&mut parents).arg(&mut turns).arg(&mut flags).arg(&mut stats);
       let cfg=LaunchConfig{grid_dim:(1,1,1),block_dim:(1,1,1),shared_mem_bytes:0};
       unsafe{launch.launch(cfg)}.unwrap();
       assert_eq!(stream.clone_dtoh(&flags).unwrap()[1],u32::from(odd));
       let stats=stream.clone_dtoh(&stats).unwrap();assert_eq!(stats[0],64);assert_eq!(stats[1],u32::from(odd));
       assert_eq!(stream.clone_dtoh(&d_masks).unwrap(),relation.masks);
     }
   }
 }
}
