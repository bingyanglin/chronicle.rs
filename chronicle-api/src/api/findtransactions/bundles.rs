use crate::api::types::Trytes81;
use chronicle_cql::{
    compression::MyCompression,
    frame::{
        consistency::Consistency,
        decoder::{
            ColumnDecoder,
            Decoder,
            Frame,
        },
        header::Header,
        query::Query,
        queryflags::{
            SKIP_METADATA,
            VALUES,
        },
    },
    rows,
};

// ----------- decoding scope -----------
rows!(
    rows: Hashes {hashes: Vec<Trytes81>},
    row: Row(
        Hash
    ),
    column_decoder: BundlesDecoder
);

pub trait Rows {
    fn decode(self) -> Self;
    fn finalize(self) -> Vec<Trytes81>;
}

impl Rows for Hashes {
    fn decode(mut self) -> Self {
        while let Some(_) = self.next() {}
        self
    }
    fn finalize(self) -> Vec<Trytes81> {
        self.hashes
    }
}
// implementation to decode the columns in order to form the hash eventually
impl BundlesDecoder for Hash {
    fn decode_column(start: usize, length: i32, acc: &mut Hashes) {
        // decode transaction hash
        let hash = Trytes81::decode(&acc.buffer()[start..], length as usize);
        acc.hashes.push(hash);
    }
    fn handle_null(_: &mut Hashes) {
        unreachable!()
    }
}

// ----------- encoding scope -----------

/// Create a query frame to lookup for tx-hashes in the edge table using a bundle
pub fn query(bundle: &Trytes81) -> Vec<u8> {
    let Query(payload) = Query::new()
        .version()
        .flags(MyCompression::flag())
        .stream(0)
        .opcode()
        .length()
        .statement("SELECT tx FROM tangle.edge WHERE vertex = ? AND kind = 'bundle'")
        .consistency(Consistency::One)
        .query_flags(SKIP_METADATA | VALUES)
        .value_count(1)
        .value(bundle)
        .build(MyCompression::get());
    payload
}
