// uses
use super::reporter;
use super::supervisor;
use tokio::sync::mpsc;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::prelude::*;

// types
pub type Sender = mpsc::UnboundedSender<Event>;
pub type Receiver = mpsc::UnboundedReceiver<Event>;
pub type Payload = Vec<u8>; // Payload type is vector<unsigned-integer-8bit>.
// args struct, each actor must have public Arguments struct,
// to pass options when starting the actor.
pub struct Args {
    pub tx: Sender, // sender's tx only to pass it to reporters (if recoonect == true).
    pub rx: Receiver, // sender's rx to recv events.
    pub supervisor_tx: supervisor::Sender,
    pub socket_tx: WriteHalf<TcpStream>,
    pub reporters: supervisor::Reporters,
    pub session_id: usize,
    pub reconnect: bool,
}
// private sender's state struct.
struct State {
    supervisor_tx: supervisor::Sender,
    reporters: supervisor::Reporters,
    session_id: usize,
    socket: WriteHalf<TcpStream>, // the socket_writehalf side to that given shard
    rx: Receiver,
}

pub enum Event {
    Payload {
            stream_id: reporter::StreamId,
            payload: Payload,
            reporter: mpsc::UnboundedSender<reporter::Event>
        },
}




pub async fn sender(args: Args) -> () {
    // init the actor
    let State {supervisor_tx, reporters, session_id, mut socket, mut rx} = init(args).await;
    // loop to process event by event.
    while let Some(Event::Payload{stream_id, payload, reporter}) = rx.recv().await {
        // write the payload to the socket, make sure the result is valid
        match socket.write_all(&payload).await {
            Ok(()) => {
                // send to reporter send_status::Ok(stream_id)
                reporter.send(reporter::Event::SendStatus(reporter::SendStatus::Ok(stream_id)));
            },
            Err(err) => {
                // send to reporter send_status::Err(stream_id)
                reporter.send(reporter::Event::SendStatus(reporter::SendStatus::Err(stream_id)));
                // close channel to prevent any further Payloads to be sent from reporters
                rx.close();
            },
        }
    } // if sender reached this line, then either write_all returned IO Err(err) or reporter(s) droped sender_tx(s)
    // probably not needed
    socket.shutdown().await;
    // send checkpoint to all reporters because the socket is mostly closed
    for (_,reporter_tx) in &reporters {
        reporter_tx.send(reporter::Event::Session(reporter::Session::CheckPoint(session_id)));
    }
}


async fn init(args: Args) -> State {
    if args.reconnect {
        for (_,reporter_tx) in &args.reporters {
            reporter_tx.send(reporter::Event::Session(reporter::Session::New(args.session_id,args.tx.clone())));
        }
    }
    // return state
    State {socket: args.socket_tx, rx: args.rx, supervisor_tx: args.supervisor_tx, reporters: args.reporters, session_id: args.session_id}
}
