// example WIP
use chronicle::engine::cluster::node::stage::reporter::{WorkerId as ReporterWorkerId, Status,SendStatus, Error, Sender as ReporterSender};
use chronicle::engine::cluster::node::stage::preparer::try_prepare;
use tokio::sync::mpsc;

const PREPARE_PAYLOAD: [u8; 200] = [0;200]; // imagine this is an encoded prepare payload

type Sender = mpsc::UnboundedSender<WorkerEvent>;

enum WorkerEvent {
  SendStatus{send_status: SendStatus, query_reference: QueryRef},
  Response{giveload: Vec<u8>, query_reference: QueryRef},
  Error{kind: Error, query_reference: QueryRef},
}

// queryRef struct which hold whatever the worker wants to link a given request.
#[derive(Clone,Copy)]
pub struct QueryRef {query_id: usize,prepare_payload: &'static [u8], status: Status}

// QueryRef new
impl QueryRef {
    fn new(query_id: usize, prepare_payload: &'static [u8]) -> Self {
        QueryRef {query_id: query_id, prepare_payload: prepare_payload,  status: Status::New}
    }
}
// worker's WorkerId struct
pub struct WorkerId {worker: Sender, query_reference: QueryRef}

// here you define how the reporter will send you(the worker) events
impl ReporterWorkerId for WorkerId {
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
     fn send_response(&mut self, tx: &ReporterSender, giveload: Vec<u8>) -> Status {
         try_prepare(self.query_reference.prepare_payload, tx, &giveload);
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

}
