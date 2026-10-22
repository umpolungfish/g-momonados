//! Complete N-only GPU-residual -> SIC masses -> nonunit observable -> product.
use g_momonados::factor_observable;
fn main()->Result<(),String>{
 let args:Vec<_>=std::env::args().skip(1).collect();let raw=args.first().ok_or("factor_observable N [B1 B2 curves sigma device seconds]")?;
 let get=|i:usize,default:u64|->Result<u64,String>{args.get(i).map(|s|s.parse().map_err(|_|"invalid numeric option".into())).unwrap_or(Ok(default))};
 let(b1,b2,curves,sigma,device,seconds)=(get(1,500)?,get(2,5000)?,get(3,16)?,get(4,6)?,get(5,1)?,get(6,300)?);
 let result=factor_observable::run(raw,b1,b2,u32::try_from(curves).map_err(|_|"curves overflow")?,sigma,usize::try_from(device).map_err(|_|"device overflow")?,seconds)?;
 println!("{result}");Ok(())
}
