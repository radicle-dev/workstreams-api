use chrone::Datetime;
use ethers::types::Address;

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
    proposals: Vec<Proposals>,
}

pub struct Receiver {
    address: Address,
    payment_rate: u32,
    payment_currency: PaymentCurrency,
}

pub enum WorkstreamType {
    Role,
    Grant,
}

pub enum PaymentCurrency {
    DAI,
}

pub enum WorkstreamState {
    funded,
    open,
    finished,
}

pub struct applications {
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

pub enum ApplicationState {
    accepted,
    rejected,
    pending,
}
