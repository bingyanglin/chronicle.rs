
use chronicle::engine::cluster::node::stage::reporter::{Status,SendStatus, Error};
use chronicle::engine::cluster::node::stage::reporter;
use tokio::sync::mpsc;

type Sender = mpsc::UnboundedSender<WorkerEvent>;

enum WorkerEvent {
  SendStatus{send_status: SendStatus, query_reference: QueryRef},
  Response{giveload: Vec<u8>, query_reference: QueryRef},
  Error{kind: Error, query_reference: QueryRef},
}

// queryRef struct which hold whatever the worker wants to link a given request.
#[derive(Clone,Copy)]
pub struct QueryRef {query_id: usize, status: Status}

// QueryRef new
impl QueryRef {
    fn new(query_id: usize) -> Self {
        QueryRef {query_id: query_id, status: Status::New}
    }
}
// worker's WorkerId struct
pub struct WorkerId {worker: Sender, query_reference: QueryRef}

// here you define how the reporter will send you(the worker) events
impl reporter::WorkerId for WorkerId {
    fn send_sendstatus_ok(&mut self, send_status: SendStatus) -> Status {
        let event = WorkerEvent::SendStatus{send_status, query_reference: self.query_reference};
        self.worker.send(event);
        self.query_reference.status.return_sendstatus_ok()
     }
     fn send_sendstatus_err(&mut self, send_status: SendStatus) -> Status {
         let event = WorkerEvent::SendStatus{send_status, query_reference: self.query_reference};
         self.worker.send(event);
         self.query_reference.status.return_error()
      }
     fn send_response(&mut self, giveload: Vec<u8>) -> Status {
         let event = WorkerEvent::Response{giveload: giveload, query_reference: self.query_reference};
         self.worker.send(event);
         self.query_reference.status.return_response()
      }
      fn send_error(&mut self, error: Error) -> Status {
          let event = WorkerEvent::Error{kind: error, query_reference: self.query_reference};
          self.worker.send(event);
          self.query_reference.status.return_error()
      }
}

#[test]
fn function_name() {
    1 == 1;
}
