use auth::*;
use sha2::digest::generic_array::sequence::Lengthen;
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
