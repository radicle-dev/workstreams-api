use ethers::types::Address;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use std::str::FromStr;
use uuid::Uuid;
use worker::{Date, DateInit, Env, Error};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum WorkstreamType {
    Role,
    Grant,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum PaymentCurrency {
    Dai,
}

impl fmt::Display for PaymentCurrency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Application {
    #[serde(skip_deserializing)]
    pub id: String,
    description: String,
    #[serde(skip_deserializing)]
    workstream_id: String,
    #[serde(skip_deserializing)]
    creator: Address,
    receivers: Vec<Receiver>,
    payment_currency: PaymentCurrency,
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    #[serde(skip_deserializing)]
    state: ApplicationState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Receiver {
    address: Address,
    payment_rate: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ApplicationState {
    Accepted,
    Rejected,
    Pending,
}

impl Default for ApplicationState {
    fn default() -> Self {
        ApplicationState::Pending
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Workstream {
    #[serde(skip_deserializing)]
    pub id: String,
    wtype: WorkstreamType,
    #[serde(skip_deserializing)]
    creator: Address,
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    description: String,
    #[serde(flatten)]
    drips_config: DripsConfig,
    #[serde(skip_deserializing)]
    state: WorkstreamState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WorkstreamState {
    Funded,
    Open,
    Finished,
}

impl Default for WorkstreamState {
    fn default() -> Self {
        WorkstreamState::Open
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DripsConfig {
    receivers: Vec<Receiver>,
    drips_acct: u32,
    payment_currency: PaymentCurrency,
    #[serde(skip_deserializing)]
    drips_hub: Address,
}

impl Workstream {
    pub fn update(
        old_workstream: &mut Workstream,
        new_workstream: Workstream,
    ) -> Result<(), worker::Error> {
        // update drips configuration
        if old_workstream.drips_config != new_workstream.drips_config {
            if !Workstream::check_drips_config(
                &old_workstream.drips_config,
                &new_workstream.drips_config,
            )? {
                return Err(worker::Error::from("wrong drips configuration"));
            }
            old_workstream.drips_config = new_workstream.drips_config;
        }
        // update dates
        check_dates(
            &new_workstream.created_at,
            &new_workstream.starting_at,
            &new_workstream.ending_at,
        )?;
        // Update time
        old_workstream.created_at = new_workstream.created_at;
        old_workstream.starting_at = new_workstream.starting_at;
        old_workstream.ending_at = new_workstream.ending_at;
        // update metadata
        old_workstream.description = new_workstream.description;
        old_workstream.wtype = new_workstream.wtype;
        // update state
        old_workstream.state = new_workstream.state;
        Ok(())
    }
    // check if passed receiver configuration actually exists on-chain
    fn check_drips_config(
        _old_config: &DripsConfig,
        _new_config: &DripsConfig,
    ) -> Result<bool, worker::Error> {
        Ok(true)
    }
    // check if dates make sense (e.g created_at is before or at the same time with starting_at)
    pub async fn populate(
        workstream: &mut Workstream,
        user: &str,
        env: &Env,
    ) -> Result<(), worker::Error> {
        workstream.id = Uuid::new_v4().to_string();
        workstream.creator = Address::from_str(user).map_err(|err| Error::from(err.to_string()))?;
        workstream.state = WorkstreamState::Open;
        workstream.created_at = Date::now().to_string();
        let drips_hub: Option<String> = env
            .kv("DRIPSHUBS")?
            .get(&workstream.drips_config.payment_currency.to_string())
            .text()
            .await?;
        if drips_hub == None {
            return Err(Error::from("no drips hub for given currency"));
        } else {
            workstream.drips_config.drips_hub = Address::from_str(&drips_hub.unwrap())
                .map_err(|err| Error::from(err.to_string()))?;
        }
        if let Some(start) = &workstream.starting_at {
            let starting_date: Date = Date::from(DateInit::String(start.to_string()));
            if Date::now().as_millis() > starting_date.as_millis() {
                return Err(Error::from("incorrect starting date"));
            } else {
                workstream.starting_at = Some(starting_date.to_string());
            }
        };
        if let Some(end) = &workstream.ending_at {
            let ending_date: Date = Date::from(DateInit::String(end.to_string()));
            if ending_date.as_millis() < Date::now().as_millis() {
                return Err(Error::from("incorrect ending date"));
            } else {
                workstream.ending_at = Some(ending_date.to_string());
            }
        };
        Ok(())
    }
}

impl Application {
    pub fn populate(
        application: &mut Application,
        user: &str,
        workstream: &str,
    ) -> Result<(), worker::Error> {
        check_dates(
            &application.created_at,
            &application.starting_at,
            &application.ending_at,
        )?;
        application.id = Uuid::new_v4().to_string();
        application.workstream_id = workstream.to_string();
        application.creator =
            Address::from_str(user).map_err(|err| Error::from(err.to_string()))?;
        application.state = ApplicationState::Pending;
        application.created_at = Date::now().to_string();
        Ok(())
    }

    pub fn update(
        old_application: &Application,
        new_application: &mut Application,
    ) -> Result<(), worker::Error> {
        check_dates(
            &new_application.created_at,
            &new_application.starting_at,
            &new_application.ending_at,
        )?;
        new_application.workstream_id = old_application.workstream_id.clone();
        new_application.creator = old_application.creator;
        new_application.created_at = old_application.created_at.clone();
        Ok(())
    }
}

fn check_dates(
    _created_at: &str,
    _starting_at: &Option<String>,
    _ending_at: &Option<String>,
) -> Result<(), worker::Error> {
    Ok(())
}
