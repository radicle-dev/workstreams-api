use ethers::core::utils::to_checksum;
use ethers::types::{Address, Signature, H160};
use serde::{Deserialize, Serialize};
use serde_json::json;
use worker::Response;
use worker::*;
mod utils;
use auth::*;
use hex::FromHex;
use rand::Rng;
use sha2::{Digest, Sha256};
use siwe::Message;
use std::str::FromStr;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, worker_ctx: Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();
    let authorized: bool = Authorization::is_authorized(&env, &req).await?;
    let router = Router::new();
    router
        .get("/api/v0/info", |req, _ctx| {
            let version = "0.1";
            console_log!("{}", req.url()?.path());
            Response::ok(version)
        })
        .post_async("/authorize", |mut req, ctx| async move {
                    let auth_req: AuthRequest = AuthRequest::from(req);
                    let token: String = Authorization::create(auth_req);
                    let mut headers = Headers::new();
                    console_log!(
                        r#"
########
New Authorization
user: {}
########
"#,
                        to_checksum(&H160(message.address), Some(0)),
                    );
                    headers.set(
                        "Set-cookie",
                        &format!(
                            "SIWE-AUTH={}; Secure; HttpOnly; SameSite=Lax; Expires={}",
                            &token,
                            Date::now().to_string()
                        ),
                    )?;
                    let res =
                        Response::redirect(worker::Url::from_str("http:/localhost/").unwrap())
                            .unwrap()
                            .with_headers(headers);
                    return Ok(res);
                }

            }
        })
        .run(req, worker_env)
        .await
}
