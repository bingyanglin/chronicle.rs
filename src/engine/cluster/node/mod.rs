pub mod stage;
pub mod supervisor;
use crate::engine::cluster::node::stage::reporter::SenderAPI;
use crate::engine::cluster::supervisor::Address;
use super::node::stage::supervisor::ReporterNum;
use evmap;

pub async fn node(address: Address, reporters_num: ReporterNum, registry: evmap::WriteHandle<u8, SenderAPI>) {
    // create node supervisor args
    let args = supervisor::Args{address, reporters_num, registry};
    // now await on node supervisor
    supervisor::supervisor(args).await;
}
