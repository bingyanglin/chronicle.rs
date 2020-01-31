// uses
use super::reporter;
use super::supervisor;
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use tokio::io::ReadHalf;
use tokio::prelude::*;


// consts
const HEADER_LENGTH: usize = 9;
const BUFFER_LENGTH: usize = 1024000;

// suerpvisor state struct
struct State {
    supervisor_tx: mpsc::UnboundedSender<supervisor::Event>,
    reporters: supervisor::Reporters,
    socket: ReadHalf<TcpStream>,
    stream_id: u16,
    total_length: usize,
    header: bool,
    zero_buffer: Vec<u8>,
    buffer: Vec<u8>,
    i: usize,
    session_id: usize,
    events: Vec<(u16,reporter::Event)>
}

// Arguments struct
pub struct Args {
    pub supervisor_tx: mpsc::UnboundedSender<supervisor::Event>,
    pub reporters: supervisor::Reporters,
    pub socket_rx: ReadHalf<TcpStream>,
    pub session_id: usize,
}

macro_rules! create_ring_mod {
    ($module:ident, $reporters:expr) => (
        pub mod $module {
            pub fn lookup_by_range() -> () {
                unimplemented!()
            }
        }
    );
}


pub async fn receiver(args: Args) -> () {

    let State {mut socket, supervisor_tx: _, reporters, mut stream_id, mut header, mut i,
        mut total_length, mut buffer, zero_buffer, session_id, mut events} = init(args).await;

    create_ring_mod!(ring, reporters);

    while let Ok(n) = socket.read(&mut buffer[i..]).await {
        let mut current_length = i+n;
        if current_length < HEADER_LENGTH {
            i = current_length; // not enough bytes to decode the frame-header
        } else {
            if !header {
                total_length = get_total_length_usize(&buffer);
                // resize buffer if total_length is larger than our buffer
                stream_id = get_stream_id_u16(&buffer);
                if total_length > BUFFER_LENGTH {
                    // resize the len of the buffer
                    buffer.resize(total_length, 0);
                }
            }
            if current_length >= total_length {
                let remaining_buffer = buffer.split_off(total_length);
                // send event(response) to reporter
                let event = reporter::Event::Response{giveload: buffer, stream_id: stream_id};
                let reporter_tx = reporters.get(&0).unwrap();
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
                    let reporter_tx = reporters.get(&0).unwrap();
                    let _ = reporter_tx.send(event);
                }
            } else {
                i = current_length; // update i to n.
                header = true; // as now we got the frame-header including knowing its body-length
            }
        }
    };
    // clean shutdown
    for (_,reporter_tx) in &reporters {
        let _ = reporter_tx.send(reporter::Event::Session(reporter::Session::CheckPoint(session_id)));
    }
}

async fn init(Args{socket_rx: socket, reporters, supervisor_tx,session_id}: Args) -> State {
    let zero_buffer: Vec<u8> = vec![0; BUFFER_LENGTH];
    let buffer: Vec<u8> = vec![0; BUFFER_LENGTH];
    let i: usize = 0; // index of the buffer
    let header: bool = false;
    let total_length: usize = 0;
    let stream_id: u16 = 0;
    let events: Vec<(u16,reporter::Event)> = Vec::with_capacity(1000);
    State {socket,session_id,total_length,i, zero_buffer, buffer, header, supervisor_tx, reporters, stream_id, events}
}


// private functions
fn process_remaining(mut buffer:  Vec<u8>, stream_id: &mut u16, total_length: &mut usize, header: &mut bool, current_length:&mut usize, acc: &mut Vec<(u16,reporter::Event)>) -> Vec<u8> {
    // first check if current_length hold header at least
    if *current_length >= HEADER_LENGTH {
        // decode and update total_length
        *total_length = get_total_length_usize(&buffer);
        // decode and update stream_id
        *stream_id = get_stream_id_u16(&buffer);
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
            }
            *header = true;
            buffer
        }
    } else {
        // not enough to decode the buffer
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

fn get_stream_id_u16(buffer: &[u8]) -> u16 {
    ((buffer[2] as u16) << 8) | buffer[4] as u16
}
