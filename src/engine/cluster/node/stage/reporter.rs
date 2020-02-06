// uses
use super::sender;
use super::supervisor;
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::time::SystemTime;

use super::worker;
// types
pub type Sender = mpsc::UnboundedSender<Event>;
type Receiver = mpsc::UnboundedReceiver<Event>;
pub type Giveload = Vec<u8>; // Giveload type is vector<unsigned-integer-8bit>.
pub type StreamId = i16; // stream id type.
pub type StreamIds = Vec<StreamId>; // StreamIds type is vector which should hold u8 from 1 to 32768.
type Worker = Box<dyn WorkerId>; // Worker is how will be presented in the workers_map.
type Workers = HashMap<StreamId,Worker>;

// reporter state struct it holds StreamIds and Workers and the reporter's Sender
struct State {
    stream_ids: StreamIds,
    workers: Workers,
    tx: Sender,
    rx: Receiver,
    sender_tx: Option<sender::Sender>,
    session_id: usize,
    checkpoints: u8,
    supervisor_tx: supervisor::Sender,
    reporter_num: u8,
    address: String,
    shard: u8,
}

// Arguments struct
pub struct Args {
    pub tx: Sender,
    pub rx: Receiver,
    pub sender_tx: sender::Sender,
    pub session_id: usize,
    pub supervisor_tx: supervisor::Sender,
    pub reporter_num: u8,
    pub stream_ids: StreamIds,
    pub address: String,
    pub shard: u8,
}

pub enum Event {
    Query {
        worker: Worker,
        payload: sender::Payload,
    },
    Response{giveload: Giveload, stream_id: StreamId},
    SendStatus(SendStatus),
    Session(Session),
}

pub enum SendStatus {
    Ok(StreamId),
    Err(StreamId),
}

#[derive(Debug)]
pub enum Error {
    Overload,
    Lost,
}

pub enum Session {
    New(usize,sender::Sender),
    CheckPoint(usize),
    Shutdown,
}

#[derive(Clone, Copy)]
pub enum Status {
    New, // the query cycle is New
    Sent, // the query had been sent to the socket.
    Respond, // the query got a response.
    Done, // the query cycle is done.
}

// query status
impl Status {
    pub fn return_sendstatus_ok(self: &mut Status) -> Status {
        match self {
            Status::New => {
                *self = Status::Sent
            }
            _ => {
                *self = Status::Done
            }
        };
        return *self
    }
    pub fn return_error(self: &mut Status) -> Status {
        *self = Status::Done;
        return *self
    }
    pub fn return_response(self: &mut Status) -> Status {
        match self {
            Status::New => {
                *self = Status::Respond
            }
            _ => {
                *self = Status::Done
            }
        };
        return *self
    }
}

// WorkerId trait type which will be implemented by worker in order to send their channeL_tx.
pub trait WorkerId: Send {
    fn send_sendstatus_ok(&mut self, send_status: SendStatus) -> Status ;
    fn send_sendstatus_err(&mut self, send_status: SendStatus) -> Status ;
    fn send_response(&mut self,tx: &Sender, giveload: Giveload) -> Status ;
    fn send_error(&mut self, error: Error) -> Status ;
}


// The main reporter actor
pub async fn reporter(args: Args) -> () {
    // init the actor
    let State {mut stream_ids, mut workers, tx, mut rx, mut sender_tx,
         mut session_id, mut checkpoints, supervisor_tx, reporter_num, shard, address} = init(args).await;
    while let Some(event) = rx.recv().await {
        match event {
            Event::Query{mut worker, payload} => {
                if let Some(stream_id) = stream_ids.pop()   {
                    // assign stream_id to the payload.
                    let payload = assign_stream_id_to_payload(stream_id, payload);
                    // put the payload inside an event of a socket_sender(the sender_tx inside reporter's state)
                    let event = sender::Event::Payload{stream_id: stream_id, payload: payload, reporter: tx.clone()};
                    // send the event
                    match sender_tx {
                        Some(sender) => {
                            sender.send(event);
                            sender_tx = Some(sender);
                            // insert worker into workers map using stream_id as key.
                            workers.insert(stream_id, worker);
                        }
                        None => {
                            // this means the sender_tx had been droped as a result of checkpoint
                            let send_status = SendStatus::Err(0);
                            worker.send_sendstatus_err(send_status);
                        }
                    }
                } else {
                    // send_overload to the worker in-case we don't have anymore stream_ids.
                    worker.send_error(Error::Overload);
                }
            },
            Event::Response{giveload, stream_id} => {

                let worker = workers.get_mut(&stream_id).unwrap();
                if let Status::Done = worker.send_response(&tx,giveload) {
                    // remove the worker from workers.
                    workers.remove(&stream_id);
                    // push the stream_id back to stream_ids vector.
                    stream_ids.push(stream_id);
                };
            }
            Event::SendStatus(send_status) => {
                match send_status {
                    SendStatus::Ok(stream_id) => {
                        // get_mut worker from workers map.
                        let worker = workers.get_mut(&stream_id).unwrap();
                        // tell the worker and mutate its status,
                        if let Status::Done = worker.send_sendstatus_ok(send_status){
                            // remove the worker from workers.
                            workers.remove(&stream_id);
                            // push the stream_id back to stream_ids vector.
                            stream_ids.push(stream_id);
                        };
                    },
                    SendStatus::Err(stream_id) => {
                        // get_mut worker from workers map.
                        let worker = workers.get_mut(&stream_id).unwrap();
                        // tell the worker and mutate worker status,
                        let _status_done = worker.send_sendstatus_err(send_status);
                        // remove the worker from workers.
                        workers.remove(&stream_id);
                        // push the stream_id back to stream_ids vector.
                        stream_ids.push(stream_id);
                    },
                }
            }
            Event::Session(checkpoint) => {
                // drop the sender_tx to prevent any further payloads, and also force sender to gracefully shutdown
                sender_tx = None;
                match checkpoint {
                    Session::New(new_session_id, new_sender_tx) => {
                        session_id = new_session_id;
                        sender_tx = Some(new_sender_tx);
                        println!("address: {}, shard: {}, reporter_num: {}, received new session_id: {:?}",address, shard, reporter_num, session_id);
                    },
                    Session::CheckPoint(old_session_id) => {
                        // check how many checkpoints we have.
                        if checkpoints == 1 {
                            // first we drain workers map from stucked requests, to force_consistency of the old_session requests
                            force_consistency(&mut stream_ids,&mut workers);
                            // reset checkpoints to 0
                            checkpoints = 0;
                            println!("address: {}, shard: {}, reporter_num: {}, closed session_id: {:?}",address, shard, reporter_num, old_session_id);
                            // tell supervisor_tx to reconnect
                            let event = supervisor::Event::Reconnect(old_session_id);
                            supervisor_tx.send(event);
                        } else {
                            checkpoints = 1;
                        }
                    }
                    Session::Shutdown => {
                        // closing rx is enough to shutdown the reporter
                        rx.close();
                    }

                }
            },
        }
    }; // reporter will reach this line only when it recvs shutdown event and eventually drains its rx.
    // therefore it must drains workers map from stucked requests(if any) to force_consistency.
    force_consistency(&mut stream_ids,&mut workers);
    println!("reporter: {} of shard: {} in node: {}, gracefully shutting down.", reporter_num, shard, address);
}

// init function for reporter
async fn init(Args{address,shard,supervisor_tx ,tx,rx, reporter_num, stream_ids, session_id, sender_tx}: Args) -> State {
    // create local workers map.
    let workers: Workers = HashMap::new();
    // return state
    State {address,shard, reporter_num,stream_ids, workers, tx, rx, sender_tx: Some(sender_tx), session_id, checkpoints: 0, supervisor_tx}
}

// private functions
fn assign_stream_id_to_payload(stream_id: StreamId,mut payload: sender::Payload) -> sender::Payload {
    payload[2] = (stream_id >> 8) as u8; // payload[2] is where the first byte of the stream_id should be,
    payload[3] = stream_id as u8; // payload[3] is the second byte of the stream_id. please refer to cql specs
    payload
}

fn force_consistency(stream_ids: &mut StreamIds, workers: &mut Workers) {
    for (stream_id, mut worker_id) in workers.drain() {
        // push the stream_id back into the stream_ids vector
        stream_ids.push(stream_id);
        // tell worker_id that we lost the idea
        worker_id.send_error(Error::Lost);
    }
}
