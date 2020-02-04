mod sender;
mod receiver;
pub mod reporter;
pub mod preparer;
pub mod supervisor;

use crate::engine::cluster::node;


pub async fn stage(supervisor_tx: node::supervisor::Sender,address: String, shard: u8, reporters_num: u8, tx: supervisor::Sender, rx: supervisor::Receiver) {
    // create stage supervisor args
    let args = supervisor::Args{address,shard,reporters_num,tx,rx, supervisor_tx};
    // now await on stage
    supervisor::supervisor(args).await;

}
