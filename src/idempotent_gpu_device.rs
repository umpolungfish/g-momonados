use cudarc::driver::CudaContext;
use std::sync::Arc;

/// Physical RTX 4070, measured as WSL GPU 1. Windows uses ordinal 0.
pub fn select() -> Result<Arc<CudaContext>,String> {
    let wanted=[0x9b,0xa0,0xee,0xd8,0xe1,0x8b,0x02,0xa5,0x7c,0x80,0xca,0x49,0x71,0x2e,0x25,0xd1];
    for ordinal in 0..CudaContext::device_count().map_err(|e|e.to_string())? as usize {
        let ctx=CudaContext::new(ordinal).map_err(|e|format!("CUDA context: {e}"))?;
        let uuid=ctx.uuid().map_err(|e|e.to_string())?;
        if uuid.bytes.iter().map(|&x|x as u8).eq(wanted) {
            let name=ctx.name().map_err(|e|e.to_string())?;
            if !name.contains("4070"){return Err(format!("GPU UUID/name mismatch: {name}"));}
            return Ok(ctx);
        }
    }
    Err("physical GPU 1 UUID not found".into())
}
