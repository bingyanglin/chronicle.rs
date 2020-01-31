pub mod stage;
pub mod supervisor;

pub async fn node(address: String, reporters_num: u8) {
    // create node supervisor args
    let args = supervisor::Args{address, reporters_num};
    // now await on node supervisor
    supervisor::supervisor(args).await;
}


#[test]
pub fn test() -> () {
    use std::error::Error;
    use tokio;
    use tokio::runtime::Runtime;
    let mut rt = Runtime::new();
    rt.unwrap().block_on(async {
        let address: String = String::from("172.17.0.2:9042");
        let reporters_num: u8 = 1;
        node(address, reporters_num).await;
    });

    ()
}
