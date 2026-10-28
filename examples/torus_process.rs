//! Native carrier bridge. JSON envelope is retained; only geometry is transformed.
extern crate alloc;
use g_momonados::tokens;
#[macro_export]
macro_rules! sprint { ($($arg:tt)*) => { print!($($arg)*) }; }
#[macro_export]
macro_rules! sprintln { ($($arg:tt)*) => { println!($($arg)*) }; }
#[allow(unused_imports)]
#[path = "../src/kernel_torus.rs"]
mod kernel_torus;
use kernel_torus::{TorusFrame,TorusMap,agent_loop_program};
use serde_json::{json,Value};
use std::io::{self,Read};

fn phase_error(a:f64,b:f64) -> f64 { let d=a-b; d.sin().atan2(d.cos()).abs() }

fn main() -> Result<(),Box<dyn std::error::Error>> {
    let mut input=String::new();io::stdin().read_to_string(&mut input)?;
    let mut state: Value=serde_json::from_str(&input)?;
    let major=state["R"].as_f64().ok_or("R required")?;
    let minor=state["r"].as_f64().ok_or("r required")?;
    let matrix: [[i64;2];2]=serde_json::from_value(state["matrix"].clone())?;
    let mut map=TorusMap::new(&agent_loop_program());
    let tokens: Vec<_>=map.stations.iter().map(|s|(s.index,s.token as usize,s.lift)).collect();
    map.set_frame(major,minor)?;map.wind(matrix)?;
    for (s,(index,token,_)) in map.stations.iter().zip(tokens.iter()) {
        assert_eq!((s.index,s.token as usize),(*index,*token));
        let read=map.frame.readout(s.station,s.normal);
        for (actual,expected) in read.into_iter().zip(s.lift) {
            assert!(phase_error(actual,expected)<1e-9);
        }
    }
    let inverse=[[matrix[1][1],-matrix[0][1]],[-matrix[1][0],matrix[0][0]]];
    map.wind(inverse)?;
    for (s,(_,_,lift)) in map.stations.iter().zip(tokens.iter()) {
        assert!((s.lift[0]-lift[0]).abs()<1e-9 && (s.lift[1]-lift[1]).abs()<1e-9);
    }
    let frame=TorusFrame{major,minor};
    let angles: Vec<[f64;2]>=serde_json::from_value(state["angles"].clone())?;
    let velocities: Vec<[f64;2]>=serde_json::from_value(state["velocities"].clone())?;
    let transformed: Vec<_>=angles.iter().map(|&p|TorusFrame::wind_lift(p,matrix)).collect();
    let velocity: Vec<_>=velocities.iter().map(|&p|TorusFrame::wind_lift(p,matrix)).collect();
    let mut residual: f64=0.0;
    let mut paired_count=0;
    for &lift in angles.iter().chain(transformed.iter()) {
        let read=frame.readout(frame.point(lift,false),TorusFrame::normal(lift));
        for (a,b) in read.into_iter().zip(lift) { residual=residual.max(phase_error(a,b)); }
        if let Some(pair) = frame.paired_readout(frame.point(lift,false),frame.point(lift,true)) {
            for (a,b) in pair.into_iter().zip(lift) {
                assert!(phase_error(a,b)<1e-8);
            }
            paired_count+=1;
        }
    }
    state["angles"]=json!(transformed);state["velocities"]=json!(velocity);
    state["paired_readout_count"]=json!(paired_count);
    state["native_readout_error"]=json!(residual);
    state["token_order_verified"]=json!(true);
    state["token_inverse_verified"]=json!(true);
    println!("{}",state);
    Ok(())
}
