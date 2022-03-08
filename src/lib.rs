use auth::{AuthRequest, Authorization};
use ethers::types::Address;
use std::collections::HashMap;
use std::str::FromStr;
use users::User;
use worker::*;
use workstreams::WorkstreamState;

use crate::workstreams::{Application, Workstream};
mod auth;
mod users;
mod utils;
mod workstreams;

/// Log a request to the API. Boilerplate function , useful for debugging purposes.
fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

/// Checks if the request has an authorization token and if that oken is authorized to access
/// the particular resource. Although complex schemes can be used with the Authorization.resources
/// vector, currently we don't use that.
///
/// The authorization scheme is very simple:
///
/// A token that is tied to an Address A, has root access to all resources under `/api/v1/users/A`.
/// For example, they can create a new workstream, edit an old one or delete, because the
/// `workstreams` resource is under the following path: `/api/v1/users/A/workstreams/`.
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

/// Parses a workstream::Request and returns a HashMap of the query strings.
///
/// `/api/v1/workstreams?state=funded` will result in a hasmap with the following key-value pair:
/// "state":"funded".
fn parse_query_string(req: &Request) -> Result<HashMap<String, String>> {
    let url = req.url()?;
    let args: HashMap<String, String> = url.query_pairs().into_owned().collect();
    Ok(args)
}

/// # API schema
///
/// ## /api/v1/users
///
/// The route accepts the following HTTP methods: GET
///
/// ## GET
///
/// It returns an array of the addresses of all the users:
/// ```
/// [
/// "0xDFA1fEa9915EF18b1f2A752343b168cA9c9d97aB"
/// ]
/// ```
///
/// ## /api/v1/workstreams
///
/// The route accepts the following HTTP methods: GET
///
/// ### GET
///
/// It returns an array of all the workstreams. It accepts a filter based on the state as a query
/// string. For example `/api/v1/workstreams?state=funded`. The accepted states are defined in
/// the WorkstreamState enum.
///
/// Response example:
/// ```
/// [
///    {
///        "id": "e0173d95-37a6-4089-b127-9eceee95574b",
///        "wtype": "Grant",
///        "creator": "0xdfa1fea9915ef18b1f2a752343b168ca9c9d97ab",
///        "created_at": "Wed Mar 02 2022 12:46:38 GMT+0000 (Coordinated Universal Time)",
///        "starting_at": "March 5, 2022 12:17:31 GMT",
///        "ending_at": "March 10, 2022 16:17:31 GMT",
///        "description": "lorem ipsum",
///        "receivers": [
///            {
///                "address": "0x7ad046baed02ef99423ef6b53c5940987c5c159b",
///                "payment_rate": 150
///            }
///        ],
///        "drips_acct": 0,
///        "payment_currency": "Dai",
///        "drips_hub": "0x0000000000000000000000000000000000000000",
///        "state": "Open"
///    }
/// ]
/// ```
///
/// ### /api/v1/users/:user/workstreams/:worksteam/applications
///
/// HTTP methods: GET, POST, PUT
///
/// Require Authorization: POST, PUT
///
/// The route accepts the following parameters encoded into the path:
/// - user
/// - workstream
///
/// For example: `api/v1/users/0xdfa1fea9915ef18b1f2a752343b168ca9c9d97ab/workstreams/e0173d95-37a6-4089-b127-9eceee95574b/applications`
///
/// ### GET
///
/// Returns an array of all Applications of the workstream with id = `:worktream`.
///
/// ### POST
///
/// Creates a new Application for the workstream with id = `workstream`.
///
/// The application is stored at the KV store of the API.
///
/// The user must pass a json object in the body of the request with a schema that follows the
/// fields in the Application struct. All fields that have the `default` decorator, can be omitted,
/// as they are populated by the API.
///
/// For example:
///{
///     "wtype": "Grant",
///     "creator": "0xDFA1fEa9915EF18b1f2A752343b168cA9c9d97aB",
///     "starting_at": "March 13, 2022 16:17:31 GMT",
///     "ending_at": "March 10, 2022 16:17:31 GMT",
///     "description": "NEW TEST ipsum",
///     "payment_currency": "Dai",
///     "receivers": [
///         {
///             "address": "0x7ad046baed02ef99423ef6b53c5940987c5c159b",
///             "payment_rate": 150
///         }
///     ],
/// }
///
/// ### PUT
///
/// Edits an existing Application by replacing all the fields of the old Application with the ones
/// of the new, passed in the body of the request as JSON. The applications are matched by `id` and
/// the following fields do not change:
/// - id
/// - created_at
/// - creator
///
/// ## `/api/v1/users/:user/workstreams/:workstream/applications/:application`
///
/// HTTP Methods: GET, DELETE
///
/// Required Authorization: DELETE
///
/// ### GET
///
/// Returns the Application object with id = `:application`
///
///### DELETE
///
/// Delete the Application object with id = `:application` from the KV STORE.
///
/// ## `/api/v1/users/:user/workstreams
///
/// HTTP Methods: GET, POST, PUT
///
/// Required Authorization: POST, PUT
///
/// ### GET
///
/// Returns all workstreams of the user `:user`.
///
/// ### POST
///
/// Creates a new workstream based on the Workstream struct that is passed as a JSON object in the
/// body of the request.
///
/// The workstream is saved at the KV store of the worker
///
/// Not all fields of the Workstream must be supplied by the user, as some are populated by the
/// API.
///
/// The API will populate the following fields:
/// - id
/// - creator (with :user)
/// - created_at
/// - DripsHub
/// - state
///
///
/// ### PUT
///
/// Edits an existing workstream and replaces it fields with the ones defined in the workstream
/// object that is passed as a JSON in the body of the request.
///
/// The following fields will not change:
/// - id
/// - creator
/// - created_at
/// - Dripshub
///
/// ## `/api/v1/users/:user/workstreams/:workstream
///
/// HTTP Methods: GET, DELETE
///
/// Require Authorization: DELETE
///
/// ### GET
///
/// Returns the workstream with id = `:workstream`.
///
/// ### DELETE
///
/// Deletes the workstream with id = `:workstream` from the KV store of the API.
///
///
/// ## /api/v1/authorize
///
/// HTTP Methods: POST
///
/// Required Authorization: None
///
/// It authorizes an ethereum address to the API and generates a token that is returned to the
/// user. Using that token, the user can access all the resources that have to do with that
/// particular ethereum address (/users/:user/..).
///
/// It accepts an AuthRequest object as a JSON encoded object in the body of the request.
///
/// The message and signature **must** comform to EIP4361: https://eips.ethereum.org/EIPS/eip-4361
///
/// The can be easily generated using:
/// - [siwe-js](https://github.com/spruceid/siwe)
/// - [siwe-rs](https://github.com/spruceid/siwe-rs)
///
/// A succesful response will include the following cookie in the headers: `SIWE-AUTH=XXXXXX`,
/// where XXXXX is the authorization token.
///
/// With that token, the user can authorize a request to access a resource via a method that
/// requires authorization. The token expires automatically based on the AuthRequest object that
/// was sent and must be renewed using the same mechanism.
///
/// An example flow of the API:
/// ```
///      ┌─────────┐                                              ┌───┐                             ┌────────┐
///      │0xab03..4│                                              │API│                             │KV_STORE│
///      └────┬────┘                                              └─┬─┘                             └───┬────┘
///           │POST /authorize {signature: "0x..", message: "{..}"} │                                   │     
///           │────────────────────────────────────────────────────>│                                   │     
///           │                                                     │                                   │     
///           │                                                     ────┐                               │     
///           │                                                         │ AuthRequest::from_req()       │     
///           │                                                     <───┘                               │     
///           │                                                     │                                   │     
///           │                                                     ────┐                               │     
///           │                                                         │ AuthRequest::create()         │     
///           │                                                     <───┘                               │     
///           │                                                     │                                   │     
///           │                                                     │{key: token, value: Authorization }│     
///           │                                                     │───────────────────────────────────>     
///           │                                                     │                                   │     
///           │                       token                         │                                   │     
///           │<────────────────────────────────────────────────────│                                   │     
///           │                                                     │                                   │     
///           │         POST /users/0xab03..4/workstreams           │                                   │     
///           │────────────────────────────────────────────────────>│                                   │     
///      ┌────┴────┐                                              ┌─┴─┐                             ┌───┴────┐
///      │0xab03..4│                                              │API│                             │KV_STORE│
///      └─────────┘                                              └───┘                             └────────┘
/// ```
///
/// AuthRequest serialized in JSON:
///
/// ```
/// '{\n    \"signature\": \"0x49a6e2a1995fde3bd10bd9ae2ecefe199ecfcb576125cc8582ee8458a4efd62668539b11f7bdb10e07f94b223f266cdd5ed592b37db4a2941541336a696d820a1c\",\n    \"message\": \"localhost:4361 wants you to sign in with your Ethereum account:\\n0xDFA1fEa9915EF18b1f2A752343b168cA9c9d97aB\\n\\nSIWE Notepad Example\\n\\nURI: http://localhost:4361\\nVersion: 1\\nChain ID: 1\\nNonce: zPPtgK5pMVHnnr8Co\\nIssued At: 2022-03-02T10:56:48.478Z\\nExpiration Time: 2022-03-02T20:56:48.474Z\\nResources:\\n- http://localhost:4361/address/0xDFA1fEa9915EF18b1f2A752343b168cA9c9d97aB\"\n}'
/// ```
///
/// If the authorization is succesful, the response will have the following header where the
/// `SIWE-AUTH` cookie is the authorization token.
///
/// ```
/// "set-cookie": "SIWE-AUTH=EACB9E10D0FD122CF0D2BA5F282CEBA0D71B48DD40A04893AAB94D1BE3F16F7D;
/// Secure; HttpOnly; SameSite=Lax; Expires=Tue Mar 08 2022 20:51:45 GMT+0000 (Coordinated
/// Universal Time)"
/// ```
///
///
///
///
#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _worker_ctx: Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();
    let router = Router::new();
    router
        .get_async("/api/v1/users", |_req, ctx| async move {
            let store = ctx.kv("USERS")?;
            let users: Vec<String> = store
                .list()
                .execute()
                .await?
                .keys
                .iter()
                .map(|x| x.name.clone())
                .collect();
            Response::from_json(&users)
        })
        .get_async("/api/v1/workstreams", |req, ctx| async move {
            let store = ctx.kv("USERS")?;
            let args = parse_query_string(&req)?;
            let addresses: Vec<String> = store
                .list()
                .execute()
                .await?
                .keys
                .iter()
                .map(|x| x.name.clone())
                .collect();
            let workstream_state: Option<WorkstreamState> = if let Some(state) = args.get("state") {
                Some(WorkstreamState::from_str(state)?)
            } else {
                None
            };
            let mut workstreams: Vec<Workstream> = vec![];
            for address in addresses {
                let user = store.get(&address).json::<User>().await?.unwrap();
                workstreams.extend(
                    user.workstreams
                        .into_values()
                        .filter(|x| {
                            if let Some(state) = &workstream_state {
                                &x.state == state
                            } else {
                                true
                            }
                        })
                        .collect::<Vec<Workstream>>(),
                );
            }
            Response::from_json(&workstreams)
        })
        .on_async(
            "/api/v1/users/:user/workstreams/:workstream/applications",
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
            "/api/v1/users/:user/workstreams/:workstream/applications/:application",
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
                                store.put(workstream_id, &applications)?.execute().await?;
                                res
                            }
                            None => Response::error("Application not found", 404),
                        };
                    }
                    _ => Response::error("HTTP Method Not Alloawed", 405),
                }
            },
        )
        .on_async(
            "/api/v1/users/:user/workstreams",
            |mut req, ctx| async move {
                let addr_string = ctx.param("user").unwrap();
                return match req.method() {
                    Method::Post => {
                        if !is_authorized(&req, &ctx.env, &ctx).await? {
                            return Response::error("Unauthorized", 401);
                        }
                        let mut workstream = req.json::<Workstream>().await?;
                        let workstream_id =
                            Workstream::populate(&mut workstream, addr_string, &ctx.env).await?;
                        console_log!("New Workstream: \n {:?}", workstream);
                        let store = ctx.kv("USERS")?;
                        let mut user =
                            if let Some(user) = store.get(addr_string).json::<User>().await? {
                                user
                            } else {
                                User {
                                    workstreams: HashMap::new(),
                                }
                            };
                        user.workstreams.insert(workstream_id, workstream.clone());
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
            },
        )
        .on_async(
            "/api/v1/users/:user/workstreams/:workstream",
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
        .post_async("/api/v1/authorize", |req, ctx| async move {
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
