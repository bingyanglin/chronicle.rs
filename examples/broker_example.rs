// example WIP
use tokio::time::delay_for;
use chronicle::engine::cluster::node::stage::reporter;
use chronicle::engine::cluster::node;
use chronicle::engine::cluster::node::stage::reporter::{
    BrokerEvent, BrokerId, SendStatus, Error, Sender as ReporterSender, Event, Id};
use tokio::sync::mpsc;

const PREPARE_PAYLOAD: [u8; 200] = [0;200]; // imagine this is an encoded prepare payload

type Sender = mpsc::UnboundedSender<BrokerEvent>;
type Receiver = mpsc::UnboundedReceiver<BrokerEvent>;

pub async fn broker_example(reporter_tx: ReporterSender) {
    // create broker
    let (tx, mut rx) = mpsc::unbounded_channel::<BrokerEvent>();
    // create query_reference with query_id(1) and prepare_payload
    let qf = reporter::QueryRef::new(1, &PREPARE_PAYLOAD);
    // create broker_id which is a BrokerID struct
    let broker_id = BrokerId::new(tx, qf);
    // create request (note: payload is encoded cql frame)
    let request = Event::Request{id: Id::Broker(broker_id), payload: [4,0,0,0,1,0,0,0,0].to_vec()};
    // imagine we know which stage's reporter(s) should handle this reuqest,
    // for sake of simplicty we are using a random reporter_tx.
    reporter_tx.send(request);
    // broker_example event loop
    while let Some(event) = rx.recv().await {
        match event {
            BrokerEvent::SendStatus{send_status, query_reference, my_tx} => {
                println!("broker_example received the send_status: {:?}, for his request with query_id {:?}, and might claimed his tx {:?}",send_status, query_reference.query_id, my_tx)
            }
            BrokerEvent::Response{giveload, query_reference, my_tx} => {
                println!("broker_example received the response: {:?}, for his request with query_id {}, and might claimed his tx {:?}",giveload, query_reference.query_id, my_tx);
            }
            BrokerEvent::Error{kind: error, query_reference: query_reference, my_tx} => {
                println!("broker_example received an internal error {:?}, for his request with query_id {}, and claimed his tx {:?}", error, query_reference.query_id, my_tx);
            }
        }
    }

}


fn main() {
    let (mut rt, registry_read) = chronicle::engine::run("172.17.0.2:9042", 1).unwrap();
    let address = "172.17.0.2:9042".to_string();

    // this temp for testing only 
    loop {
        unsafe{
            if node::supervisor::READY == true {
                break;
            }
        }
    }

    let x = registry_read.get_and(&1, |r| {
        rt.block_on(broker_example(r.first().unwrap().reporter.clone()));
    });


}
