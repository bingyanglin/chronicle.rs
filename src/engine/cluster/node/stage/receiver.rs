// uses
use crate::engine::cluster::node::stage::reporter::StreamId;
use super::reporter;
use super::supervisor;
use tokio::net::TcpStream;
use tokio::io::ReadHalf;
use tokio::prelude::*;

// types
type Events = Vec<(reporter::StreamId,reporter::Event)>;
// consts
const HEADER_LENGTH: usize = 9;
const BUFFER_LENGTH: usize = 1024000;

// suerpvisor state struct
struct State {
    supervisor_tx: supervisor::Sender,
    reporters: supervisor::Reporters,
    socket: ReadHalf<TcpStream>,
    stream_id: reporter::StreamId,
    total_length: usize,
    header: bool,
    buffer: Vec<u8>,
    i: usize,
    session_id: usize,
    events: Events,
    appends_num: i16,
}

// Arguments struct
pub struct Args {
    pub supervisor_tx: supervisor::Sender,
    pub reporters: supervisor::Reporters,
    pub socket_rx: ReadHalf<TcpStream>,
    pub session_id: usize,
}


macro_rules! create_ring_mod {
    ($module:ident, $reporters:expr) => (
        pub mod $module {
            fn get_reporter_by_stream_id() {
                unimplemented!()
            }
        }
    );
}


pub async fn receiver(args: Args) -> () {
    let State {mut socket, supervisor_tx: _, reporters, mut stream_id, mut header, mut i,
        mut total_length, mut buffer, session_id, mut events, appends_num} = init(args).await;
    // create range lookup (TODO)
    create_ring_mod!(ring, reporters);
    // receiver event loop
    while let Ok(n) = socket.read(&mut buffer[i..]).await {
        // if n != 0 then the socket is not closed
        if n != 0 {
            let mut current_length = i+n; // cuurrent buffer length is i(recent index) + received n-bytes
            if current_length < HEADER_LENGTH {
                i = current_length; // not enough bytes to decode the frame-header
            } else {
                // if no-header decode the header and resize the buffer(if needed).
                if !header {
                    // decode total_length(HEADER_LENGTH + frame_body_length)
                    total_length = get_total_length_usize(&buffer);
                    // decode stream_id
                    stream_id = get_stream_id(&buffer);
                    // resize buffer only if total_length is larger than our buffer
                    if total_length > BUFFER_LENGTH {
                        // resize the len of the buffer.
                        buffer.resize(total_length, 0);
                    }
                }
                if current_length >= total_length {
                    let remaining_buffer = buffer.split_off(total_length);
                    // send event(response) to reporter
                    let event = reporter::Event::Response{giveload: buffer, stream_id: stream_id};
                    let reporter_tx = reporters.get(&compute_reporter_num(stream_id, appends_num)).unwrap();
                    let _ = reporter_tx.send(event);
                    // decrease total_length from current_length
                    current_length -= total_length;
                    // reset total_length to zero
                    total_length = 0;
                    // reset i to new current_length
                    i = current_length;
                    // process any events in the remaining_buffer.
                    buffer = process_remaining(remaining_buffer, &mut stream_id,&mut total_length, &mut header, &mut i, &mut events);
                    // drain acc events
                    for (s_id, event) in events.drain(..) {
                        let reporter_tx = reporters.get(&compute_reporter_num(s_id, appends_num)).unwrap();
                        let _ = reporter_tx.send(event);
                    }
                } else {
                    i = current_length; // update i to n.
                    header = true; // as now we got the frame-header including knowing its body-length
                }
            }
        } else {
            // breaking the while loop as the received n == 0.
            break
        }

    };
    // clean shutdown and closing the session
    for (_,reporter_tx) in &reporters {
        let _ = reporter_tx.send(reporter::Event::Session(reporter::Session::CheckPoint(session_id)));
    }
}

async fn init(Args{socket_rx: socket, reporters, supervisor_tx,session_id}: Args) -> State {
    let buffer: Vec<u8> = vec![0; BUFFER_LENGTH];
    let i: usize = 0; // index of the buffer
    let header: bool = false;
    let total_length: usize = 0;
    let stream_id: reporter::StreamId = 0;
    let events: Events = Vec::with_capacity(1000);
    let appends_num: i16 = 32767/reporters.len() as i16;
    State {socket,session_id,total_length,i, buffer, header, supervisor_tx, reporters, stream_id, events, appends_num}
}


// private functions
fn process_remaining(mut buffer:  Vec<u8>, stream_id: &mut reporter::StreamId, total_length: &mut usize, header: &mut bool, current_length:&mut usize, acc: &mut Events) -> Vec<u8> {
    // first check if current_length hold header at least
    if *current_length >= HEADER_LENGTH {
        // decode and update total_length
        *total_length = get_total_length_usize(&buffer);
        // decode and update stream_id
        *stream_id = get_stream_id(&buffer);
        // check if current_length
        if *current_length >= *total_length {
            let remaining_buffer = buffer.split_off(*total_length);
            let event = reporter::Event::Response{giveload: buffer, stream_id: *stream_id};
            acc.push((*stream_id, event));
            // reset before loop
            *current_length -= *total_length;
            process_remaining(remaining_buffer, stream_id, total_length, header, current_length, acc)
        } else {
            if *total_length > BUFFER_LENGTH {
                buffer.resize(*total_length, 0);
            } else {
                buffer.resize(BUFFER_LENGTH, 0);
            }
            *header = true;
            buffer
        }
    } else {
        // not enough to decode the buffer, make sure to resize(extend) the buffer to BUFFER_LENGTH
        buffer.resize(BUFFER_LENGTH, 0);
        *header = false;
        *total_length = 0;
        buffer
    }
}

fn get_total_length_usize(buffer: &[u8]) -> usize {
    HEADER_LENGTH +
    // plus body length
    ((buffer[5] as usize) << 24) +
    ((buffer[6] as usize) << 16) +
    ((buffer[7] as usize) <<  8) +
    ((buffer[8] as usize) <<  0)
}

fn get_stream_id(buffer: &[u8]) -> reporter::StreamId {
    ((buffer[2] as reporter::StreamId) << 8) | buffer[3] as reporter::StreamId
}

fn compute_reporter_num(stream_id: StreamId, appends_num: i16)  -> u8 {
    (stream_id/appends_num) as u8
}
