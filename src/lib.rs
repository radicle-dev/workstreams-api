use auth::{AuthRequest, Authorization};
use ethers::types::Address;
use std::collections::HashMap;
use std::str::FromStr;
use users::User;
use worker::*;
use workstreams::Workstream;
mod auth;
mod users;
mod utils;
mod workstreams;
fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

async fn is_authorized(req: &Request, env: &Env, ctx: &RouteContext<()>) -> Result<bool> {
    let token = auth::Authorization::parse_request(req).await?;
    let auth = match auth::Authorization::get(env, token).await? {
        Some(authorization) => authorization,
        None => return Ok(false),
    };
    let addr = Address::from_str(ctx.param("user").unwrap())
        .map_err(|_| worker::Error::from("Cannot parse address"))?;
    Ok(addr == auth.address)
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
        .get("/api/v0/info/", |req, _ctx| {
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
                    Some(user) => Response::from_json(&user.workstreams.get(workstream_id)),
                    None => Response::error("User not found", 404),
                };
            },
        )
        .post_async("/users/:user/workstreams", |mut req, ctx| async move {
            if !is_authorized(&req, &ctx.env, &ctx).await? {
                return Response::error("Unauthorized", 401);
            }
            let addr_string = ctx.param("user").unwrap();
            let mut workstream: Workstream = req.json::<Workstream>().await?;
            Workstream::populate(&mut workstream, addr_string, &ctx.env).await?;
            let store = ctx.kv("USERS")?;
            let mut user = if let Some(user) = store.get(&addr_string).json::<User>().await? {
                user
            } else {
                User {
                    workstreams: HashMap::new(),
                }
            };
            user.workstreams.insert(workstream.id.clone(), workstream);
            store.put(&addr_string, user)?;
            Response::ok("workstream created")
        })
        .post_async("/authorize/", |req, ctx| async move {
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
            let res = Response::ok("authorization created")
                .unwrap()
                .with_headers(headers);
            return Ok(res);
        })
        .run(req, env)
        .await
}
