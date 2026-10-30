//! Shared-anchor SIC recovery of one arbitrary-length complex carrier.
//! The overlapping ququart frames belong to the same ray; a common anchor
//! recovers every cross-frame coherence without a quadratic global Gram.
use alloc::{vec, vec::Vec};
use super::{error, SicError};
use super::frame::{Complex, Operator, Sic, SicCoordinates, SicFrame};

pub struct RayFrame {
    pub indices: Vec<usize>,
    pub masses: Vec<Complex>,
    pub signed_gram_residuals: Vec<Complex>,
}
pub struct RayReturn {
    pub recovered: Vec<Complex>,
    pub frames: Vec<RayFrame>,
    pub anchor: usize,
    pub gauge: Complex,
    pub signed_amplitude_residuals: Vec<Complex>,
    pub local_residual: f64,
    pub anchor_residual: f64,
    pub full_gram_bound: f64,
    pub tolerance: f64,
}

pub fn recover_ray(frame:&SicFrame,source:&[Complex],anchor:usize,
    retained_masses:Option<&[Vec<Complex>]>)->Result<RayReturn,SicError>{
    if source.is_empty()||source.iter().any(|z|!z.finite())||anchor>=source.len()||source[anchor].abs2()==0.0{
        return Err(error("invalid shared-anchor complex carrier"));
    }
    let remaining:Vec<_>=(0..source.len()).filter(|&i|i!=anchor).collect();
    let groups:Vec<_>=if remaining.is_empty(){vec![vec![]]}else{remaining.chunks(3).map(|c|c.to_vec()).collect()};
    if retained_masses.is_some_and(|m|m.len()!=groups.len()){
        return Err(error("retained SIC frames differ from the carrier cover"));
    }
    let mut recovered=vec![Complex::default();source.len()];let mut frames=Vec::new();
    let mut reference=None;let mut local_residual=0.0f64;let mut anchor_residual=0.0f64;
    for (number,group) in groups.into_iter().enumerate(){
        let mut ray=[Complex::default();4];ray[0]=source[anchor];
        for (slot,&index) in group.iter().enumerate(){ray[slot+1]=source[index];}
        let gram=Operator::projector(&ray)?;
        let masses=if let Some(held)=retained_masses{
            SicCoordinates{dimension:4,values:held[number].clone()}
        }else{frame.split(&gram)?};
        let returned=frame.fuse(&masses)?;
        local_residual=local_residual.max(returned.distance(&gram)?);
        let population=returned.get(0,0).re;
        if population<=0.0{return Err(error("SIC return lost the shared enclosing anchor"));}
        let amplitude=libm::sqrt(population);let common=*reference.get_or_insert(amplitude);
        anchor_residual=anchor_residual.max((amplitude-common).abs());
        recovered[anchor]=Complex::new(common,0.0);
        for (slot,&index) in group.iter().enumerate(){recovered[index]=returned.get(slot+1,0).scale(1.0/common);}
        frames.push(RayFrame{indices:group,masses:masses.values,
            signed_gram_residuals:returned.values().iter().zip(gram.values()).map(|(a,b)|*a-*b).collect()});
    }
    let gauge=source[anchor].conj().scale(1.0/libm::sqrt(source[anchor].abs2()));
    let errors:Vec<_>=source.iter().zip(&recovered).map(|(v,w)|*w-*v*gauge).collect();
    let maximum=source.iter().map(|z|libm::sqrt(z.abs2())).fold(0.0,f64::max);
    let returned_max= recovered.iter().map(|z|libm::sqrt(z.abs2())).fold(0.0,f64::max);
    let error_max=errors.iter().map(|z|libm::sqrt(z.abs2())).fold(0.0,f64::max);
    let gamma=32.0*f64::EPSILON/(1.0-32.0*f64::EPSILON);
    let scale=maximum.max(returned_max);
    let full_gram_bound=2.0*maximum*error_max+error_max*error_max+gamma*scale*scale;
    let tolerance=f64::EPSILON*256.0*(source.len()+1) as f64*maximum.max(1.0)*maximum.max(1.0);
    if full_gram_bound>tolerance||local_residual>tolerance||anchor_residual>tolerance{
        return Err(error("SIC shared-carrier reconstruction exceeds source-scaled roundoff bound"));
    }
    Ok(RayReturn{recovered,frames,anchor,gauge,signed_amplitude_residuals:errors,
        local_residual,anchor_residual,full_gram_bound,tolerance})
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn shared_cover_recovers_every_cross_frame_complex_coordinate(){
        let frame=super::super::wh::ququart_frame();
        for length in [1,2,4,5,13,37]{
            let source:Vec<_>=(0..length).map(|i|Complex::new((i+1) as f64/7.0,(i%5) as f64/3.0)).collect();
            let out=recover_ray(&frame,&source,0,None).unwrap();
            for i in 0..length{for j in 0..length{
                let error=out.recovered[i]*out.recovered[j].conj()-source[i]*source[j].conj();
                assert!(libm::sqrt(error.abs2())<=out.full_gram_bound);
            }}
            let mut changed=source.clone();
            if length>1{
                changed[length-1]=changed[length-1]*Complex::new(0.0,1.0);
                assert!(recover_ray(&frame,&changed,0,None).is_ok());
                let held:Vec<_>=out.frames.into_iter().map(|f|f.masses).collect();
                assert!(recover_ray(&frame,&changed,0,Some(&held)).is_err());
            }
        }
    }
    #[test]
    fn canonical_frame_returns_each_complex_matrix_unit(){
        let frame=super::super::wh::ququart_frame();
        for i in 0..16{for z in [Complex::new(1.0,0.0),Complex::new(0.0,1.0)]{
            let mut matrix=Operator::zero(4);matrix.set(i/4,i%4,z);
            let recovered=frame.fuse(&frame.split(&matrix).unwrap()).unwrap();
            assert!(recovered.distance(&matrix).unwrap()<1e-12);
        }}
    }
}
