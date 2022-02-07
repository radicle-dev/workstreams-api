use serde::{Deserialize, Serialize};
use worker::*;

mod auth;
mod types;

#[derive(Deserialize, Serialize)]
struct AuthRequest {
    message: String,
    signature: String,
}

#[derive(Deserialize, Serialize)]
struct Authorization {
    resources: Vec<String>,
    issued_at: String,
    expiration_time: Option<String>,
    not_before: Option<String>,
    address: H160,
}

// We don't implement the `from` trait because of the `async` keyword
//

impl Authorization {
    async fn from_req(env: &Env, req: &Request) -> Result<Option<Authorization>> {
        let headers = req.headers();
        let bearer = headers.get("BEARER")?;
        let cookie = headers.get("AUTH-SIWE")?;
        let token = match bearer.or(cookie) {
            Some(token) => token,
            None => return Err(worker::Error::from("no authorization header found")),
        };
        let store = env.kv("AUTHENTICATION")?;
        store
            .get(&token)
            .json::<Authorization>()
            .await
            .map_err(|error| worker::Error::from(error))
    }
    async fn is_authorized(env: &Env, req: &Request) -> Result<bool> {
        match Authorization::from_req(env, req).await? {
            Some(auth) => Ok(auth.resources.contains(&req.url()?.path().to_string())),
            None => Err(worker::Error::from("could not find auth object")),
        }
    }
    async fn create(signature: String, message: Message) -> Result<Authorization> {
        match message.verify(signature) {
            Ok(_) => {
                let authentication = ctx.kv("AUTHENTICATION")?;
                let mut rng = rand::thread_rng();
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
                hash
            }
            Err(error) => {
                return Response::from_json(&json!(
                        {"verified": false, "error" : format!("{:?}", error) }))
            }
        }
    }
}

#[async_trait]
impl From<Request> for AuthRequest {
    async fn from(req: Request) -> Self {
        let body = req
            .json::<AuthRequest>()
            .await
            .map_err(|error| worker::Error::from(format!("body parsing: {:?}", error)))?;
        let sig = <[u8; 65]>::from_hex(body.signature.trim_start_matches("0x"))
            .map_err(|error| worker::Error::from(format!("signature parsing: {:?}", error)))?;
        let msg: Message = body
            .message
            .parse()
            .map_err(|error| worker::Error::from(format!("siwe message parsing: {:?}", error)))?;
    }
}
