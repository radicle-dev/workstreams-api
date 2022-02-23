use ethers::types::Address;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use worker::{wasm_bindgen::UnwrapThrowExt, Date, DateInit, Env, Error};
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
pub struct Receiver {
    address: Address,
    payment_rate: u32,
    payment_currency: PaymentCurrency,
}

#[derive(Deserialize, Serialize)]
pub enum ApplicationState {
    Accepted,
    Rejected,
    Pending,
}

#[derive(Deserialize, Serialize)]
pub struct Workstream {
    pub id: String,
    wtype: WorkstreamType,
    creator: String,
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    receivers: Vec<Receiver>,
    description: String,
    drips_hub: String,
    state: WorkstreamState,
    applications: Option<Vec<Application>>,
}

impl Workstream {
    pub async fn populate(
        workstream: &mut Workstream,
        user: &str,
        env: &Env,
    ) -> Result<(), worker::Error> {
        workstream.id = Uuid::new_v4().to_string();
        workstream.creator = user.to_owned();
        workstream.state = WorkstreamState::Open;
        workstream.created_at = Date::now().to_string();
        workstream.drips_hub = env
            .kv("DRIPSHUBS")?
            .get(&workstream.drips_hub)
            .text()
            .await?
            .unwrap_throw();
        if let Some(start) = &workstream.starting_at {
            if Date::now().as_millis() > Date::from(DateInit::String(start.to_string())).as_millis()
            {
                return Err(Error::from("incorrect starting date"));
            }
        };
        if let Some(end) = &workstream.ending_at {
            if Date::from(DateInit::String(end.to_string())).as_millis() < Date::now().as_millis() {
                return Err(Error::from("incorrect ending date"));
            }
        };
        workstream.applications = None;
        Ok(())
    }
}
