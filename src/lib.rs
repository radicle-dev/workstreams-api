use auth::{AuthRequest, Authorization};
use ethers::types::Address;
use std::collections::HashMap;
use std::str::FromStr;
use users::User;
use worker::*;

use crate::workstreams::{Application, Workstream};
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
        .on_async(
            "/users/:user/workstreams/:workstream/applications",
            |mut req, ctx| async move {
                let workstream_id = ctx.param("workstream").unwrap();
                let user_address = ctx.param("user").unwrap();
                console_log!(
                    "user {} requested applications from workstream {} with method: {:?}",
                    user_address,
                    workstream_id,
                    req.method()
                );
                return match req.method() {
                    Method::Post => {
                        if !is_authorized(&req, &ctx.env, &ctx).await? {
                            return Response::error("Unauthorized", 401);
                        }
                        let store = ctx.kv("APPLICATIONS")?;
                        let mut application = req.json::<Application>().await?;
                        let mut applications = if let Some(applications) = store
                            .get(workstream_id)
                            .json::<HashMap<String, Application>>()
                            .await?
                        {
                            applications
                        } else {
                            HashMap::new()
                        };
                        Application::populate(&mut application, user_address, workstream_id)?;
                        applications.insert(application.id.clone(), application.clone());
                        store.put(workstream_id, applications)?.execute().await?;
                        Response::from_json::<Application>(&application)
                    }
                    Method::Put => {
                        if !is_authorized(&req, &ctx.env, &ctx).await? {
                            return Response::error("Unauthorized", 401);
                        }
                        let store = ctx.kv("APPLICATIONS")?;
                        let mut new_application = req.json::<Application>().await?;
                        let mut applications = match store
                            .get(workstream_id)
                            .json::<HashMap<String, Application>>()
                            .await?
                        {
                            Some(applications) => {
                                match applications.get(&new_application.id) {
                                    Some(old_application) => {
                                        Application::update(old_application, &mut new_application)?;
                                    }
                                    None => {
                                        Application::populate(
                                            &mut new_application,
                                            user_address,
                                            workstream_id,
                                        )?;
                                    }
                                }
                                applications
                            }
                            None => {
                                Application::populate(
                                    &mut new_application,
                                    user_address,
                                    workstream_id,
                                )?;
                                HashMap::new()
                            }
                        };
                        applications.insert(workstream_id.to_string(), new_application.clone());
                        store.put(workstream_id, applications)?.execute().await?;
                        Response::from_json::<Application>(&new_application)
                    }
                    Method::Get => {
                        return match ctx
                            .kv("APPLICATIONS")?
                            .get(workstream_id)
                            .json::<HashMap<String, Application>>()
                            .await?
                        {
                            Some(applications) => {
                                Response::from_json::<HashMap<String, Application>>(&applications)
                            }
                            None => Response::error("No applications found for workstream", 404),
                        }
                    }
                    _ => Response::error("HTTP Method Not Allowed", 405),
                };
            },
        )
        .on_async(
            "/users/:user/workstreams/:workstream/applications/:application",
            |req, ctx| async move {
                let workstream_id = ctx.param("workstream").unwrap();
                let application_id = ctx.param("application").unwrap();
                match req.method() {
                    Method::Get => {
                        return match ctx
                            .kv("APPLICATIONS")?
                            .get(workstream_id)
                            .json::<HashMap<String, Application>>()
                            .await?
                        {
                            Some(applications) => match applications.get(application_id) {
                                Some(application) => {
                                    Response::from_json::<Application>(application)
                                }
                                None => Response::error("Application Not Found", 404),
                            },
                            None => {
                                Response::error("Workstream not found or has no applications", 404)
                            }
                        }
                    }
                    Method::Delete => {
                        if !is_authorized(&req, &ctx.env, &ctx).await? {
                            return Response::error("Unauthorized", 401);
                        }
                        let store = ctx.kv("APPLICATIONS")?;
                        return match store
                            .get(workstream_id)
                            .json::<HashMap<String, Application>>()
                            .await?
                        {
                            Some(mut applications) => {
                                let res = Response::from_json(&applications.remove(application_id));
                                store.put(&workstream_id, &applications)?.execute().await?;
                                res
                            }
                            None => Response::error("Application not found", 404),
                        };
                    }
                    _ => Response::error("HTTP Method Not Alloawed", 405),
                }
            },
        )
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
            "/users/:user/workstreams/:workstream",
            |mut req, ctx| async move {
                let workstream_id = ctx.param("workstream").unwrap();
                let addr_string = ctx.param("user").unwrap();
                console_log!("path: {}", req.path());
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
                            None => Response::error("Workstream not found", 404),
                        };
                    }
                    Method::Delete => {
                        if !is_authorized(&req, &ctx.env, &ctx).await? {
                            return Response::error("Unauthorized", 401);
                        }
                        let store = ctx.kv("USERS")?;
                        return match store.get(addr_string).json::<User>().await? {
                            Some(mut user) => {
                                let res =
                                    Response::from_json(&user.workstreams.remove(workstream_id));
                                store.put(addr_string, user)?.execute().await?;
                                res
                            }
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
