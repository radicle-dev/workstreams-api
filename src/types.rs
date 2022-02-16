use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct User {
    pub address: Address,
    pub workstreams: Option<Vec<Workstream>>,
}

#[derive(Deserialize, Serialize)]
pub struct Workstream {
    pub id: String,
    wtype: WorkstreamType,
    pub creator: Address,
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    receivers: Vec<Receiver>,
    description: String,
    drips_hub: Address,
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
    Funded,
    Open,
    Finished,
}

#[derive(Deserialize, Serialize)]
pub struct Application {
    id: String,
    description: String,
    workstream_id: String,
    creator: Address,
    receivers: Vec<Receiver>,
    created_at: String,
    starting_at: String,
    ending_at: Option<String>,
    state: ApplicationState,
}

#[derive(Deserialize, Serialize)]
pub enum ApplicationState {
    Accepted,
    Rejected,
    Pending,
}
