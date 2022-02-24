use ethers::types::Address;
use ethers::utils::to_checksum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use worker::{wasm_bindgen::UnwrapThrowExt, Date, DateInit, Env, Error};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum WorkstreamType {
    Role,
    Grant,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum PaymentCurrency {
    DAI,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum WorkstreamState {
    Funded,
    Open,
    Finished,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct Receiver {
    address: Address,
    payment_rate: u32,
    payment_currency: PaymentCurrency,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ApplicationState {
    Accepted,
    Rejected,
    Pending,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Workstream {
    pub id: String,
    wtype: WorkstreamType,
    creator: String,
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    description: String,
    drips_config: DripsConfig,
    state: WorkstreamState,
    applications: Option<Vec<Application>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DripsConfig {
    receivers: Vec<Receiver>,
    drips_acct: u32,
    drips_hub: Address,
}

impl Workstream {
    pub fn update(
        old_workstream: &mut Workstream,
        new_workstream: Workstream,
    ) -> Result<(), worker::Error> {
        if old_workstream.drips_config != new_workstream.drips_config {
            if !Workstream::check_drips_config(
                &old_workstream.drips_config,
                &new_workstream.drips_config,
            )? {
                return Err(worker::Error::from("wrong drips configuration"));
            }
            old_workstream.drips_config = new_workstream.drips_config;
        }
        if !Workstream::check_dates(
            &new_workstream.created_at,
            &new_workstream.starting_at,
            &new_workstream.ending_at,
        )? {
            return Err(worker::Error::from("wrong date format"));
        }
        // Update time
        old_workstream.created_at = new_workstream.created_at;
        old_workstream.starting_at = new_workstream.starting_at;
        old_workstream.ending_at = new_workstream.ending_at;
        // update metadata
        old_workstream.description = new_workstream.description;
        old_workstream.wtype = new_workstream.wtype;
        Ok(())
    }
    // check if passed receiver configuration actually exists on-chain
    fn check_drips_config(
        old_config: &DripsConfig,
        new_config: &DripsConfig,
    ) -> Result<bool, worker::Error> {
        Ok(true)
    }
    // check if dates make sense (e.g created_at is before or at the same time with starting_at)
    fn check_dates(
        created_at: &String,
        starting_at: &Option<String>,
        ending_at: &Option<String>,
    ) -> Result<bool, worker::Error> {
        Ok(true)
    }
    pub async fn populate(
        workstream: &mut Workstream,
        user: &str,
        env: &Env,
    ) -> Result<(), worker::Error> {
        workstream.id = Uuid::new_v4().to_string();
        workstream.creator = user.to_owned();
        workstream.state = WorkstreamState::Open;
        workstream.created_at = Date::now().to_string();
        workstream.drips_config.drips_hub = Address::from_slice(
            env.kv("DRIPSHUBS")?
                .get(&to_checksum(&workstream.drips_config.drips_hub, Some(1)))
                .text()
                .await?
                .unwrap_throw()
                .as_bytes(),
        );
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
    pub async fn put(old_workstream: &mut Workstream, new_workstream: &Workstream) {}
}
