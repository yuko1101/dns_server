use std::collections::HashMap;

use hickory_server::proto::rr::{Name, RData};
use serde::Deserialize;

use crate::forwarder::Forwarder;

#[derive(Deserialize)]
pub struct Config {
    pub domains: HashMap<Name, DomainData>,
    pub fallbacks: Vec<Forwarder>,
}

#[derive(Deserialize)]
pub struct DomainData {
    pub records: Vec<RecordConfig>,
}

#[derive(Deserialize)]
pub struct RecordConfig {
    pub rdata: RData,
    pub interface_translation: Option<String>,
}
