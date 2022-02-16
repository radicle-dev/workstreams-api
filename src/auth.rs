use ethers::types::{Signature, H160};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use siwe::Message;
use std::str::FromStr;
use worker::*;

#[derive(Deserialize, Serialize)]
// We store the message and signature in String format in plcae of their native structs so that
// we can easily Serialize and Deserialize
pub struct AuthRequest {
    message: String,
    signature: String,
}

#[derive(Deserialize, Serialize)]
pub struct Authorization {
    resources: Vec<String>,
    issued_at: String,
    expiration_time: Option<String>,
    not_before: Option<String>,
    pub address: H160,
}

impl Authorization {
    pub async fn parse_request(req: &Request) -> Result<String> {
        let headers = req.headers();
        let bearer = headers.get("BEARER")?;
        let cookie = headers.get("AUTH-SIWE")?;
        match bearer.or(cookie) {
            Some(token) => Ok(token),
            None => return Err(worker::Error::from("no authorization header found")),
        }
    }
    pub async fn get<T>(env: &Env, token: T) -> Result<Option<Authorization>>
    where
        T: Into<String>,
    {
        let store = env.kv("AUTHENTICATION")?;
        store
            .get(&token.into())
            .json::<Authorization>()
            .await
            .map_err(|error| worker::Error::from(error))
    }
    // not sure what is the best return for this function
    pub async fn create(env: &Env, auth: AuthRequest) -> Result<String> {
        let message: Message =
            Message::from_str(&auth.message).map_err(|err| worker::Error::from(err.to_string()))?;
        match message.verify(
            Signature::from_str(&auth.signature)
                .map_err(|err| worker::Error::from(err.to_string()))?
                .into(),
        ) {
            Ok(_) => {
                let authentication = env.kv("AUTHENTICATION")?;
                let mut rng = rand::thread_rng();
                let message: Message = Message::from_str(&auth.message)
                    .map_err(|err| worker::Error::from(err.to_string()))?;
                let mut hasher = Sha256::new();
                let auth = Authorization {
                    resources: message
                        .resources
                        .iter()
                        .map(|x| x.as_str().to_owned())
                        .collect::<Vec<String>>(),
                    issued_at: format!("{}", message.issued_at),
                    expiration_time: message.expiration_time.clone().map(|x| format!("{}", x)),
                    not_before: message.not_before.map(|x| format!("{}", x)),
                    address: H160(message.address),
                };
                let auth_string: String = serde_json::to_string(&auth).unwrap();
                hasher.update(auth_string.as_bytes());
                // add salt to the auth token
                hasher.update(rng.gen::<[u8; 32]>());
                let hash = format!("{:X}", hasher.finalize());
                // store the auth object of a user with the auth token as key
                // the auth object KV will expire when then SIWE message expires as well.
                // That way, we don't have stale auth objects in our KV store
                authentication
                    .put(&hash, &auth_string)?
                    .expiration(
                        message
                            .expiration_time
                            .unwrap()
                            .as_ref()
                            .timestamp()
                            .unsigned_abs(),
                    )
                    .execute()
                    .await?;
                Ok(hash)
            }
            Err(_) => Err(worker::Error::from(
                "Failed to verify supplied message with signature",
            )),
        }
    }
}

impl AuthRequest {
    pub async fn from_req(mut req: Request) -> Result<AuthRequest> {
        let body = req
            .json::<AuthRequest>()
            .await
            .map_err(|error| worker::Error::from(format!("body parsing: {:?}", error)))?;
        let sig: String = body.signature.trim_start_matches("0x").to_owned();
        let msg: String = body.message;
        Ok(AuthRequest {
            message: msg.to_string(),
            signature: sig,
        })
    }
}
