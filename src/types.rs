use chrono::Datetime;
use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct User {
    address: Address,
    pub workstreams: Option<Vec<Workstream>>,
}

#[derive(Deserialize, Serialize)]
pub struct Workstream {
    id: String,
    wtype: WorkstreamType,
    creator: Address,
    created_at: DateTime,
    starting_at: Option<Datetime>,
    ending_at: Option<DateTime>,
    receivers: Vec<Receiver>,
    description: String,
    dripsHub: Address,
    state: WorkstreamState,
    applications: Option<Vec<Application>>,
}

#[derive(Deserialize, Serialize)]
pub struct Receiver {
    address: Address,
    payment_rate: u32,
    payment_currency: PaymentCurrency,
}

#[derive(Deserialize, Serialize)]
pub enum WorkstreamType {
    Role,
    Grant,
}

#[derive(Deserialize, Serialize)]
pub enum PaymentCurrency {
    DAI,
}

#[derive(Deserialize, Serialize)]
pub enum WorkstreamState {
    funded,
    open,
    finished,
}

#[derive(Deserialize, Serialize)]
pub struct Application {
    id: String,
    description: String,
    workstream_id: String,
    creator: Address,
    receivers: Vec<Receiver>,
    created_at: Datetime,
    starting_at: Datetime,
    ending_at: Option<Datetime>,
    state: ApplicationState,
}

#[derive(Deserialize, Serialize)]
pub enum ApplicationState {
    accepted,
    rejected,
    pending,
}
