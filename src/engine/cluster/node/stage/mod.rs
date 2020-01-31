mod sender;
mod receiver;
pub mod supervisor;
pub mod reporter;


pub async fn stage(address: String, shard: u8, reporters_num: u8) {
    // create stage supervisor args
    let args = supervisor::Args{address,shard,reporters_num};
    // now await on stage
    supervisor::supervisor(args).await;

}
