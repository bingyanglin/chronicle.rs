// node supervisor .. spawn stages
use crate::engine::cluster::node::stage::supervisor::Reporters;
use std::collections::HashMap;
use super::stage;
use tokio::sync::mpsc;
use tokio;

// types
type Stages = HashMap<u8, stage::supervisor::Sender>;
pub type Sender = mpsc::UnboundedSender<Event>;
pub type Receiver = mpsc::UnboundedReceiver<Event>;

// suerpvisor state struct
struct State {
    args: Args,
    spawned: bool,
    tx: Sender,
    rx: Receiver,
    shards_num: u8,
    stages: Stages,
    node: Vec<(String, Reporters)>,
}

// Arguments struct
pub struct Args {
    pub address: String,
    pub reporters_num: u8,
    // pub supervisor_tx:
}


// event Enum
pub enum Event {
    GetShardsNum,
    Shutdown,
    Expose(String, Reporters),
}


pub async fn supervisor(args: Args) -> () {
    let State{mut rx,tx,mut spawned, mut shards_num, mut stages, args, mut node} = init(args).await;
    // send self GetShardsNum
    tx.send(Event::GetShardsNum);
    // event loop
    while let Some(event) = rx.recv().await {
        match event {
            Event::GetShardsNum => {
                // TODO connect to scylla-shard-zero and get_cql_opt to finally get the shards_num
                // for testing purposes, we will manually define it.
                shards_num = 1; // shard(shard-0)
                // ready to spawn stages
                for shard in 0..shards_num {
                    let (stage_tx, stage_rx) = mpsc::unbounded_channel::<stage::supervisor::Event>();
                    let stage = stage::stage(tx.clone(),args.address.clone(), shard, args.reporters_num.clone(), stage_tx.clone(), stage_rx);
                    tokio::spawn(stage);
                    stages.insert(shard, stage_tx);
                }
                spawned = true;
            }
            Event::Shutdown => {
                if spawned {
                    // this mean the stages are still running, therefore we send shutdown events
                    for (_,stage) in stages.drain() {
                        let event = stage::supervisor::Event::Shutdown;
                        stage.send(event);
                    }
                }
                rx.close();
            }
            Event::Expose(key, reporters) => {
                node.push((key, reporters));
                if shards_num == (node.len() as u8) {
                    // now we have all stage's reporters, therefore we expose the node to cluster supervisor

                }
            }
        }
    }


}

async fn init(args: Args) -> State {
    // init the channel
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    let stages: Stages = HashMap::new();
    let node: Vec<(String, Reporters)> = Vec::new();
    // return state
    State {tx,rx,stages,shards_num: 0, spawned: false, args, node}
}
