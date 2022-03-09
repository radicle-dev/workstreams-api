use ethers::types::Address;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use std::str::FromStr;
use uuid::Uuid;
use worker::{Date, DateInit, Env, Error};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
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
    #[serde(default)]
    pub id: String,
    title: String,
    description: String,
    #[serde(default)]
    workstream_id: String,
    #[serde(default)]
    creator: Address,
    receivers: Vec<Receiver>,
    payment_currency: PaymentCurrency,
    #[serde(default)]
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    wtype: WorkstreamType,
    #[serde(default)]
    creator: Address,
    #[serde(default)]
    created_at: String,
    starting_at: Option<String>,
    ending_at: Option<String>,
    description: String,
    #[serde(flatten)]
    drips_config: DripsConfig,
    #[serde(default)]
    pub state: WorkstreamState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WorkstreamState {
    Funded,
    Open,
    Finished,
}

impl FromStr for WorkstreamState {
    type Err = worker::Error;
    fn from_str(input: &str) -> Result<Self, worker::Error> {
        let lower = input.to_lowercase();
        match lower.as_ref() {
            "funded" => Ok(WorkstreamState::Funded),
            "open" => Ok(WorkstreamState::Open),
            "finished" => Ok(WorkstreamState::Finished),
            _ => Err(worker::Error::from("can't parse Workstream State")),
        }
    }
}
impl Default for WorkstreamState {
    fn default() -> Self {
        WorkstreamState::Open
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DripsConfig {
    drips_acct: u32,
    payment_currency: PaymentCurrency,
    #[serde(skip_deserializing)]
    drips_hub: Address,
}

impl Workstream {
    /// Updates a Workstream instance with the fields from another Workstream instance. We don't
    /// update all the fields, for security reasons (e.g creator, creation_type).  The
    /// old_workstream is usually the object retrieved from the KV store and the new_workstream is
    /// the object passed by the user.
    ///
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
        check_dates(&new_workstream.starting_at, &new_workstream.ending_at)?;
        // Update time
        old_workstream.starting_at = new_workstream.starting_at;
        old_workstream.ending_at = new_workstream.ending_at;
        // update metadata
        old_workstream.description = new_workstream.description;
        old_workstream.wtype = new_workstream.wtype;
        old_workstream.title = new_workstream.title;
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
    /// Populate a new workstream instance passed by the user. Populate is different from update,
    /// because here the user creates an incomplete Workstream object (with some fields missing)
    /// and the API is responsible for populating these fields (like workstream_id, creation_date,
    /// etc.).
    ///
    /// The API is configured to use the official DripsHub contracts, which are usually tied to a
    /// particular ERC20. The KV store for the address of the dripshubs is populated by the DevOps
    /// team ahead of deployment. If the API didn't populate this field, the user could pass
    /// an arbitrary smart contract, with important security implications.
    ///
    pub async fn populate(
        workstream: &mut Workstream,
        user: &str,
        env: &Env,
    ) -> Result<String, worker::Error> {
        workstream.id = Uuid::new_v4().to_string();
        workstream.creator = Address::from_str(user).map_err(|err| Error::from(err.to_string()))?;
        workstream.state = WorkstreamState::Open;
        check_dates(&workstream.starting_at, &workstream.ending_at)?;
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
        Ok(workstream.id.to_string())
    }
}

impl Application {
    /// Populate a new application instance. It follows the same philosphy as
    /// Workstream::populate().
    pub fn populate(
        application: &mut Application,
        user: &str,
        workstream: &str,
    ) -> Result<(), worker::Error> {
        check_dates(&application.starting_at, &application.ending_at)?;
        application.created_at = Date::now().to_string();
        application.id = Uuid::new_v4().to_string();
        application.workstream_id = workstream.to_string();
        application.creator =
            Address::from_str(user).map_err(|err| Error::from(err.to_string()))?;
        application.state = ApplicationState::Pending;
        application.created_at = Date::now().to_string();
        Ok(())
    }

    /// Update an application instance. It follows the same philosophy as Workstream::update().
    pub fn update(
        old_application: &Application,
        new_application: &mut Application,
    ) -> Result<(), worker::Error> {
        check_dates(&new_application.starting_at, &new_application.ending_at)?;
        new_application.workstream_id = old_application.workstream_id.clone();
        new_application.creator = old_application.creator;
        new_application.created_at = old_application.created_at.clone();
        Ok(())
    }
}
/// Performs sanity check to the dates passed to either Workstream or Application
/// with the following simple rule: `starting_a`t should be before now() and before `ending_at`
fn check_dates(
    starting_at: &Option<String>,
    ending_at: &Option<String>,
) -> Result<(), worker::Error> {
    if let Some(start) = starting_at {
        let starting_date: Date = Date::from(DateInit::String(start.to_string()));
        if Date::now().as_millis() > starting_date.as_millis() {
            return Err(Error::from("incorrect starting date"));
        }
    }
    if let Some(end) = ending_at {
        let ending_date: Date = Date::from(DateInit::String(end.to_string()));
        if ending_date.as_millis() < Date::now().as_millis() {
            return Err(Error::from("incorrect ending date"));
        }
    }
    Ok(())
}
