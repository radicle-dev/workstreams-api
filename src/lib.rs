use auth::{AuthRequest, Authorization};
use ethers::core::utils::to_checksum;
use ethers::types::Address;
use std::str::FromStr;
use types::*;
use worker::*;
mod auth;
mod types;
mod utils;

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
    let router = Router::new();
    if req.path().chars().last() != Some('/') {
        return Response::redirect(Url::from_str(&format!("{}/", req.url()?))?);
    }
    router
        .get("/api/v0/info", |req, _ctx| {
            let version = "0.1";
            console_log!("{}", req.url()?.path());
            Response::ok(version)
        })
        .get_async(
            "/users/:user_address/workstreams/*workstream_id",
            |_req, ctx| async move {
                let workstream_id = ctx
                    .param("workstream_id")
                    .unwrap()
                    .strip_prefix('/')
                    .unwrap();
                let address = ctx.param("user_address").unwrap();
                console_log!("user: {}, requested workstream: {}", address, workstream_id);
                return match ctx.kv("USERS")?.get(address).json::<User>().await? {
                    Some(user) => Response::from_json(
                        &user
                            .workstreams
                            .unwrap()
                            .into_iter()
                            .filter(|workstream| {
                                if workstream_id != "" {
                                    &workstream.id == workstream_id
                                } else {
                                    true
                                }
                            })
                            .collect::<Vec<Workstream>>(),
                    ),
                    None => Response::error("User not found", 404),
                };
            },
        )
        .post_async("/users/:user/workstreams", |mut req, ctx| async move {
            let token = auth::Authorization::parse_request(&req).await?;
            let auth = match auth::Authorization::get(&ctx.env, token).await? {
                Some(authorization) => authorization,
                None => return Response::error("No authorization found", 401),
            };
            let addr = match Address::from_str(ctx.param("user").unwrap()) {
                Ok(address) => address,
                Err(_) => return Response::error("Could not parse address", 502),
            };
            if addr != auth.address {
                return Response::error("Unauthorized user to create workstream", 401);
            }
            let addr_string = to_checksum(&addr, Some(1));
            let workstream: Workstream = req.json::<Workstream>().await?;
            let store = ctx.kv("WORKSTREAMS")?;
            let mut workstreams = store.get(&addr_string).json::<Vec<Workstream>>().await?;
            if let Some(ref mut workstreams) = workstreams {
                let _ = &workstreams.push(workstream);
            } else {
                workstreams = Some(vec![workstream]);
            }
            store.put(&addr_string, workstreams)?;
            Response::ok("workstream created")
        })
        .post_async("/users/*user", |req, ctx| async move {
            let token = auth::Authorization::parse_request(&req).await?;
            let auth = match auth::Authorization::get(&ctx.env, token).await? {
                Some(authorization) => authorization,
                None => return Response::error("No authorization found", 401),
            };
            let addr = match Address::from_str(ctx.param("user").unwrap()) {
                Ok(address) => address,
                Err(_) => return Response::error("Could not parse address", 502),
            };
            if addr != auth.address {
                return Response::error("Unauthorized user to create workstream", 401);
            }
            let user = User {
                address: addr,
                workstreams: None,
            };
            let store = ctx.kv("USER")?;
            let value = serde_json::to_string(&user).map_err(|err| worker::Error::from(err))?;
            let key = ctx.param("user").unwrap();
            store.put(key, value)?;
            return Response::ok("user created");
        })
        .post_async("/authorize", |req, ctx| async move {
            let auth_req: AuthRequest = AuthRequest::from_req(req).await?;
            let token: String = Authorization::create(&ctx.env, auth_req).await?;
            let mut headers = Headers::new();
            headers.set(
                "Set-cookie",
                &format!(
                    "SIWE-AUTH={}; Secure; HttpOnly; SameSite=Lax; Expires={}",
                    &token,
                    Date::now().to_string()
                ),
            )?;
            let res = Response::redirect(worker::Url::from_str("http:/localhost/").unwrap())
                .unwrap()
                .with_headers(headers);
            return Ok(res);
        })
        .run(req, env)
        .await
}
