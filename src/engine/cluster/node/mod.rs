pub mod stage;
pub mod supervisor;
use crate::engine::cluster::supervisor::Address;
use super::node::stage::supervisor::ReporterNum;

pub async fn node(address: Address, reporters_num: ReporterNum) {
    // create node supervisor args
    let args = supervisor::Args{address, reporters_num};
    // now await on node supervisor
    supervisor::supervisor(args).await;
}

#[test]
pub fn test() -> () {
    use tokio;
    let rt = tokio::runtime::Runtime::new();
    rt.unwrap().block_on(async move {
        let address: Address = String::from("172.17.0.2:9042");
        let reporters_num: u8 = 1;
        node(address, reporters_num).await;
    });

    ()
}
