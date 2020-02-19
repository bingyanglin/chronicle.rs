pub mod cluster;
use crate::engine::cluster::node::stage::reporter::SenderAPI;
use tokio;
use evmap;


pub fn run(address: &'static str, reporters_num: u8) -> std::result::Result<(tokio::runtime::Runtime, evmap::ReadHandle<u8, SenderAPI>), std::io::Error> {
    // create registry (ex evmap)
    let (registry_read, registry_write) = evmap::new();
    let rt = tokio::runtime::Runtime::new()?;
    rt.spawn(async move {
        let address: String = String::from(address);
        cluster::node::node(address, reporters_num, registry_write).await;
    });
    Ok((rt, registry_read))
}
