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
        req.cf().region().unwrap_or_else(|| "unknown region".into())
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
    console_log!("Authorization is tied with user: {}", addr);
    Ok(addr == auth.address)
}
#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _worker_ctx: Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();
    let router = Router::new();
    router
        .get("/api/v0/info/", |req, _ctx| {
            let version = "0.1";
            console_log!("{}", req.url()?.path());
            Response::ok(version)
        })
        .on_async("/users/:user/workstreams", |mut req, ctx| async move {
            let addr_string = ctx.param("user").unwrap();
            return match req.method() {
                Method::Post => {
                    if !is_authorized(&req, &ctx.env, &ctx).await? {
                        return Response::error("Unauthorized", 401);
                    }
                    let mut workstream = req.json::<Workstream>().await?;
                    Workstream::populate(&mut workstream, addr_string, &ctx.env).await?;
                    console_log!("New Workstream: \n {:?}", workstream);
                    let store = ctx.kv("USERS")?;
                    let mut user = if let Some(user) = store.get(addr_string).json::<User>().await?
                    {
                        user
                    } else {
                        User {
                            workstreams: HashMap::new(),
                        }
                    };
                    user.workstreams
                        .insert(workstream.id.clone(), workstream.clone());
                    console_log!("New user struct: \n {:?}", user);
                    store.put(addr_string, user)?.execute().await?;
                    Response::from_json::<Workstream>(&workstream)
                }
                Method::Get => {
                    return match ctx.kv("USERS")?.get(addr_string).json::<User>().await? {
                        Some(user) => Response::from_json(&user.workstreams),
                        None => Response::error("User not found", 404),
                    };
                }
                _ => Response::error("HTTP Method Not Allowed", 405),
            };
        })
        .on_async(
            "/users/:user/workstreams/:workstream_id",
            |mut req, ctx| async move {
                let workstream_id = ctx.param("workstream_id").unwrap();
                let addr_string = ctx.param("user").unwrap();
                console_log!(
                    "user {} requested workstream {} with method: {:?}",
                    addr_string,
                    workstream_id,
                    req.method()
                );
                return match req.method() {
                    Method::Put => {
                        if !is_authorized(&req, &ctx.env, &ctx).await? {
                            return Response::error("Unauthorized", 401);
                        }
                        let workstream_new: Workstream = req.json::<Workstream>().await?;
                        let store = ctx.kv("USERS")?;
                        if let Some(mut user) = store.get(addr_string).json::<User>().await? {
                            let workstream_old = user.workstreams.get_mut(workstream_id);
                            console_log!(
                                "Editing old workstream \n{:?} \n with:\n{:?}",
                                workstream_old,
                                workstream_new
                            );
                            match workstream_old {
                                Some(wk) => {
                                    Workstream::update(wk, workstream_new.clone())?;
                                    wk
                                }
                                None => {
                                    return Response::error("Unknown workstream ID", 404);
                                }
                            };
                            store.put(addr_string, user)?.execute().await?;
                            return Response::ok("workstream updated");
                        }
                        Response::ok("workstream updated")
                    }
                    Method::Get => {
                        return match ctx.kv("USERS")?.get(addr_string).json::<User>().await? {
                            Some(user) => Response::from_json(&user.workstreams.get(workstream_id)),
                            None => Response::error("User not found", 404),
                        };
                    }
                    _ => Response::error("HTTP Method Not Allowed", 405),
                };
            },
        )
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
            let res = Response::ok("authorization created")
                .unwrap()
                .with_headers(headers);
            Ok(res)
        })
        .run(req, env)
        .await
}
