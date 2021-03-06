use std::net::SocketAddr;
use std::sync::Arc;
use std::str::FromStr;
use std::convert::TryInto;
use std::time::Duration;

use serde::Deserialize;
use plumb::{Pipe, PipeExt};
use plumb::tuple_utils::{Append, Prepend, Pluck};
use hyper::Body;
use hyper::body::Buf;
use http::{header, StatusCode};
use http_mux::{route, mux};
use cookie::Cookie;

use crate::error::Error;
use crate::{database, ui};

pub const COOKIE_NAME : &str = "ear7h-token";

type Request = http::Request<Body>;
type Response = http::Response<Body>;
type Mux = mux::Mux<Error, (), Body, Response>;

pub struct ServerInner {
    pub db :           database::Db,
    pub render :       ui::Renderer,
    pub authn :        authn::client::Client,
}

pub type Server = Arc<ServerInner>;

#[derive(Deserialize)]
struct Config {
    port : u16,
    database : String,
    authn : authn::client::Config,
}

pub fn new_server(config_file : &str) -> Result<(Server, SocketAddr), Error> {
    let file = std::fs::File::open(config_file)?;
    let conf : Config = serde_json::from_reader(file)?;

    let authn : authn::client::Client = conf.authn.try_into()?;

    let addr = SocketAddr::from(([127, 0, 0, 1], conf.port));

    let server = Arc::new(ServerInner {
        authn,
        db :           database::Db::new(&conf.database)?,
        render :       ui::Renderer::new(),
    });

    Ok((server, addr))
}

/// url variable for user IDs which may be "self" or the user id
struct UserId(Option<u32>);

impl UserId {
    /// compares the optional id in self id with a concrete one, if there's a match
    /// the concrete id is returned.
    fn compare(&self, id : u32) -> Option<u32> {
        if self.0.is_none() || self.0.unwrap() == id {
            Some(id)
        } else {
            None
        }
    }
}

impl FromStr for UserId {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s : &str) -> Result<Self, Self::Err> {
        if s == "self" {
            return Ok(UserId(None))
        }

        u32::from_str(s)
            .map(Some)
            .map(UserId)
    }
}

fn log_middleware<P>(next : P) -> impl Pipe<Input = (SocketAddr, Request), Output = P::Output>
where
    P : Pipe<Input = (Request,), Output = Response> + Send + Sync + 'static,
{
    // TODO: probably not ideal?
    // bettern than cloning the whole mux
    let next = Arc::new(next);

    plumb::id()
    .aseq(|addr, req : Request| async move {
        let pre_details = if let Some(addr) =  req.headers().get("x-forwarded-for") {
            format!(
                "{:?} {} {}",
                addr,
                req.method(),
                req.uri().path(),
            )
        } else {
            format!(
                "{:?} {} {}",
                addr,
                req.method(),
                req.uri().path(),
            )
        };

        let start = tokio::time::Instant::now();

        let res = next.run((req,)).await;

        let end = tokio::time::Instant::now();
        let delta = end - start;

        println!(
            "{} {} {:?}",
            res.status(),
            pre_details,
            delta
        );

        res
    })
}


fn with_authn<S>(server_orig : Server) ->
    impl Fn(S) -> plumb::PinBoxFut<
        Result<
            <S as Append<u32>>::Output,
            Error
        >
    > + Clone
where
    S : Pluck<Head = Request> + Append<u32> + Send + 'static,
    <S as Pluck>::Tail : Send,
{
    move |s : S| {
        let server = server_orig.clone();
        Box::pin(async move {
            let (req, tail) = s.pluck();

            for cookie in req.headers().get_all(http::header::COOKIE) {
                let cookie_str = if let Ok(s) = cookie.to_str() {
                    s
                } else {
                    continue
                };

                match Cookie::parse(cookie_str) {
                    Ok(c) if c.name() == COOKIE_NAME => {

                        let name = server.authn.validate_token(c.value()).await?;
                        let user_id = server.db.get_user_by_name(&name).await?.id;

                        return Ok(tail.prepend(req).append(user_id))
                    },
                    _ => {}
                }
            }

            return Err(Error::FailedLogin)
        })
    }
}



pub fn routes(server : Server) -> impl Pipe<Input = (SocketAddr, Request), Output = Response> {
    macro_rules! register_routes {
        ($($route:ident,)*) => {
            {
                let mux = mux::new_mux::<Error, _, _>();

                $(let mux = $route(Arc::clone(&server), mux);)*

                mux
            }
        }
    }

    let mux = register_routes!{
        get_users_links,
        post_users_links,
        get_login,
        post_login,
        get_logout,
    }
    .tuple()
    .seq(|res : Result<Response, Error>| {
        match res {
            Ok(res) => res,
            Err(err) => render_error(err),
        }
    });

    log_middleware(mux)
}


fn get_users_links(server : Server, m : Mux) -> Mux {

    m.handle(
        route!(GET / "users" / UserId / "links.html"),
        mux::new_handler()
        .map_tuple().aand_then(with_authn(server.clone()))
        .and_then(|req, url_id : UserId, token_id : u32| {
            // authz
            url_id.compare(token_id)
            .map(|id| Ok((req, id)))
            .unwrap_or(Err(Error::Unauthorized))
        })
        .map_bind(server.clone())
        .aand_then(|_req, user_id, server : Server| async move {
            let links = server.db.get_links(user_id).await?;
            let user = server.db.get_user(user_id).await?;
            let page = server.render.users_links(&user, links.as_slice(), true);

            Ok(Response::new(page.into()))
        })
    )
}

fn post_users_links(server : Server, m : Mux) -> Mux {
    m.handle(
        route!(POST / "users" / UserId / "links.html"),
        mux::new_handler()
        .map_tuple().aand_then(with_authn(server.clone()))
        .and_then(|req, url_id : UserId, token_id : u32| {
            // authz
            if let Some(user_id) = url_id.compare(token_id) {
                Ok((req, user_id))
            } else {
                Err(Error::Unauthorized)
            }
        })
        .map_bind(server.clone())
        .aand_then(|req : Request, user_id, server : Server| async move {
            let reader = hyper::body::aggregate(req.into_body()).await?.reader();

            #[derive(Deserialize)]
            struct PostLinksForm{
                links : String,
            }

            let form : PostLinksForm = serde_urlencoded::from_reader(reader)
                .map_err(|_| Error::BadRequest)?;

            for line in form.links.lines().filter(|l| !l.is_empty()) {
                let u = url::Url::parse(&line)
                    .map_err(|_| Error::InvalidUrl(line.to_string()))?;

                match server.db.insert_link(user_id, u.as_str()).await {
                    Ok(_) | Err(Error::DuplicateUrl(_)) => {},
                    Err(err) => return Err(err)
                }
            }

            let links = server.db.get_links(user_id).await?;
            let user = server.db.get_user(user_id).await?;

            let page = server.render.users_links(&user, links.as_slice(), true);

            Ok(Response::new(page.into()))
        })
    )
}

fn get_login(server : Server, m : Mux) -> Mux {

    m.handle(
        route!(GET / "login.html"),
        mux::new_handler()
        .map_tuple().amap(with_authn(server.clone()))
        .map_tuple().map_bind(server.clone())
        .map(|cookie : Result<(_, u32), _>, server : Server| {
            if cookie.is_ok() {
                http::response::Builder::new()
                    .header(header::LOCATION, "/users/self/links.html")
                    .status(StatusCode::SEE_OTHER)
                    .body("redirecting".into()).unwrap()
            } else {
                Response::new(server.render.login().into())
            }
        })
    )
}

fn get_logout(_server : Server, m : Mux) -> Mux {

    m.handle(
        route!(GET / "logout.html"),
        mux::new_handler()
        .map(|_| {
            let cookie = Cookie::build(COOKIE_NAME, "<logged out>")
                .http_only(true)
                .same_site(cookie::SameSite::Strict)
                .path("/")
                .finish()
                .to_string();

            http::response::Builder::new()
                .header(header::LOCATION, "/login.html")
                .status(StatusCode::SEE_OTHER)
                .header(header::SET_COOKIE, cookie)
                .body("redirecting".into()).unwrap()
        })
    )
}

fn post_login(server : Server, m : Mux) -> Mux {
    #[derive(Deserialize)]
    struct Req {
        username : String,
        password : String,
    }

    struct Res {
        token : String,
    }

    impl Into<Response> for Res {
        fn into(self) -> Response {
            let cookie = Cookie::build(COOKIE_NAME, self.token.clone())
                .http_only(true)
                .same_site(cookie::SameSite::Strict)
                .path("/")
                .finish()
                .to_string();

            http::response::Builder::new()
                .header(header::LOCATION, "/users/self/links.html")
                .header(header::SET_COOKIE, cookie)
                .status(StatusCode::SEE_OTHER)
                .body("redirecting".into()).unwrap()
        }
    }

    m.handle(
        route!(POST / "login.html"),
        mux::new_handler()
        .map_bind(server.clone())
        .aand_then(|req : Request, server : Server| async move {
            let reader = hyper::body::aggregate(req.into_body()).await?.reader();

            let form : Req = serde_urlencoded::from_reader(reader)
                .map_err(|_| Error::BadRequest)?;

            let token = server.authn.login(
                &form.username,
                &form.password,
                Duration::from_secs(60 * 60 * 24 * 7),
            ).await.map_err(|err| {
                eprintln!("{:?}", err);
                Error::FailedLogin
            })?;

            server.db.upsert_user(&form.username).await?;

            Ok(Res{token}.into())
        })
    )
}


fn render_error(err : Error) -> Response {
    use http::StatusCode as S;
    use Error::*;

    eprintln!("{:?}", &err);

    let status;
    let body;

    match err {
        InvalidUrl(s) => {
            status = S::BAD_REQUEST;
            body = format!("invalid url: {}", s);
        },
        DuplicateUrl(s) => {
            status = S::CONFLICT;
            body = format!("duplicate url: {}", s);
        },
        RouteNotFound => {
           status = S::NOT_FOUND;
           body = "route not found".to_string();
        },
        FailedLogin => {
           status = S::UNAUTHORIZED;
           body = include_str!("../ui/failed-login.html").to_string();
        },
        _ => {
            status = S::INTERNAL_SERVER_ERROR;
            body   = "internal server error".to_string();
        }
    }

   http::response::Builder::new()
       .status(status)
       .body(body.into())
       .unwrap()
}
