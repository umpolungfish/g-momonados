//! Batched CUDA FFT verification of WH overlaps, without displaced-state tables.
use super::frame::Complex;
use super::wh::{OverlapField, WhSic};
use super::{error, SicError};
use cudarc::driver::{CudaContext, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::compile_ptx;
use std::f64::consts::PI;
const KERNEL: &str = r#"
extern "C" __global__ void reverse_bits(const double* a,double* b,unsigned int n,unsigned int bits,unsigned int rows) {
 unsigned long long i=(unsigned long long)blockIdx.x*blockDim.x+threadIdx.x;
 if(i>=(unsigned long long)n*rows)return;
 unsigned int k=i%n,rev=0; for(unsigned int j=0;j<bits;j++){rev=(rev<<1)|(k&1);k>>=1;}
 unsigned long long out=(i/n)*n+rev;b[2*out]=a[2*i];b[2*out+1]=a[2*i+1];
}
extern "C" __global__ void butterfly(const double* a,double* b,unsigned int n,unsigned int span,unsigned int rows,double sign,double scale) {
 unsigned long long pair=(unsigned long long)blockIdx.x*blockDim.x+threadIdx.x;
 if(pair>=(unsigned long long)(n/2)*rows)return;
 unsigned int half=span/2;unsigned long long row=pair/(n/2),local=pair%(n/2),base=(local/half)*span,j=local%half;
 unsigned long long x=row*n+base+j,y=x+half;double angle=sign*6.2831853071795864769*(double)j/span;
 double c=cos(angle),s=sin(angle),vr=a[2*y]*c-a[2*y+1]*s,vi=a[2*y]*s+a[2*y+1]*c;
 b[2*x]=(a[2*x]+vr)*scale;b[2*x+1]=(a[2*x+1]+vi)*scale;
 b[2*y]=(a[2*x]-vr)*scale;b[2*y+1]=(a[2*x+1]-vi)*scale;
}
"#;
fn batch_fft(
    input: &[Complex],
    rows: usize,
    width: usize,
    positive: bool,
    normalize: bool,
) -> Result<Vec<Complex>, SicError> {
    if !width.is_power_of_two() || width < 2 || rows.checked_mul(width) != Some(input.len()) {
        return Err(error("invalid GPU FFT batch"));
    }
    let n = u32::try_from(width).map_err(|_| error("GPU FFT width overflow"))?;
    let r = u32::try_from(rows).map_err(|_| error("GPU FFT row overflow"))?;
    let ctx = CudaContext::new(0).map_err(|e| SicError(format!("SIC CUDA context: {e}")))?;
    let stream = ctx.default_stream();
    let module = ctx
        .load_module(compile_ptx(KERNEL).map_err(|e| SicError(format!("SIC NVRTC: {e}")))?)
        .map_err(|e| SicError(e.to_string()))?;
    let reverse = module
        .load_function("reverse_bits")
        .map_err(|e| SicError(e.to_string()))?;
    let butterfly = module
        .load_function("butterfly")
        .map_err(|e| SicError(e.to_string()))?;
    let host: Vec<f64> = input.iter().flat_map(|z| [z.re, z.im]).collect();
    let mut a = stream
        .clone_htod(&host)
        .map_err(|e| SicError(e.to_string()))?;
    let mut b = stream
        .alloc_zeros::<f64>(host.len())
        .map_err(|e| SicError(e.to_string()))?;
    let threads = u32::try_from(input.len()).map_err(|_| error("GPU FFT grid overflow"))?;
    let bits = n.trailing_zeros();
    let cfg = |count: u32| LaunchConfig {
        grid_dim: (count.div_ceil(256), 1, 1),
        block_dim: (256, 1, 1),
        shared_mem_bytes: 0,
    };
    let mut launch = stream.launch_builder(&reverse);
    launch.arg(&a).arg(&mut b).arg(&n).arg(&bits).arg(&r);
    unsafe { launch.launch(cfg(threads)) }.map_err(|e| SicError(e.to_string()))?;
    std::mem::swap(&mut a, &mut b);
    let sign = if positive { 1.0f64 } else { -1.0 };
    let mut span = 2u32;
    while span <= n {
        let scale = if normalize && span == n {
            1.0 / n as f64
        } else {
            1.0
        };
        let mut launch = stream.launch_builder(&butterfly);
        launch
            .arg(&a)
            .arg(&mut b)
            .arg(&n)
            .arg(&span)
            .arg(&r)
            .arg(&sign)
            .arg(&scale);
        unsafe { launch.launch(cfg(threads / 2)) }.map_err(|e| SicError(e.to_string()))?;
        std::mem::swap(&mut a, &mut b);
        if span == n {
            break;
        }
        span *= 2;
    }
    let values = stream.clone_dtoh(&a).map_err(|e| SicError(e.to_string()))?;
    Ok(values
        .chunks_exact(2)
        .map(|z| Complex::new(z[0], z[1]))
        .collect())
}
pub fn wh_overlaps(wh: &WhSic) -> Result<OverlapField, SicError> {
    let d = super::frame::Sic::dimension(wh);
    let psi = wh.fiducial();
    let width = if d.is_power_of_two() {
        d
    } else {
        d.checked_mul(2)
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| n.checked_next_power_of_two())
            .ok_or_else(|| error("GPU Bluestein width overflow"))?
    };
    let mut a = vec![Complex::default(); d * width];
    let mut b = a.clone();
    for p in 0..d {
        for n in 0..d {
            let z = psi[n].conj() * psi[(n + d - p) % d];
            if d == width {
                a[p * width + n] = z;
            } else {
                let chirp = Complex::phase(PI * (n as f64) * (n as f64) / d as f64);
                a[p * width + n] = z * chirp;
                b[p * width + n] = chirp.conj();
                if n != 0 {
                    b[p * width + width - n] = chirp.conj();
                }
            }
        }
    }
    let output = if d == width {
        batch_fft(&a, d, width, true, false)?
    } else {
        let mut af = batch_fft(&a, d, width, false, false)?;
        let bf = batch_fft(&b, d, width, false, false)?;
        for (a, b) in af.iter_mut().zip(bf) {
            *a = *a * b;
        }
        batch_fft(&af, d, width, true, true)?
    };
    let mut values = Vec::with_capacity(d * d);
    for p in 0..d {
        for q in 0..d {
            let mut z = output[p * width + q];
            if d != width {
                z = z * Complex::phase(PI * (q as f64) * (q as f64) / d as f64);
            }
            z = z * Complex::phase(-(PI + PI / d as f64) * (p as f64) * (q as f64));
            values.push(z);
        }
    }
    Ok(OverlapField {
        dimension: d,
        values,
    })
}
