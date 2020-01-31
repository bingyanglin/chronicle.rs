pub mod node;
mod supervisor;
use tokio::time::delay_for;
use std::error::Error;
use tokio;
use tokio::runtime::Runtime;
use std::time::Duration;
 use node::stage::supervisor as stage_supervisor;

pub async fn node() {
    // create the node supervisor
}

#[test]
pub fn test() -> Result<(), Box<dyn Error>> {
    // Open a TCP stream to the socket address.
    let mut rt = Runtime::new()?;
    
    rt.block_on(async {
    let args = stage_supervisor::Args{reporters_num: 2, address: "172.17.0.2:9042".to_string(), shard: 0};
    stage_supervisor::supervisor(args).await;
    });



    Ok(())
}
