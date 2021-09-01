use std::convert::Infallible;
use std::str::FromStr;
use std::sync::Arc;

use cookie::{Cookie, SameSite};
use http::{header, StatusCode};
use serde::Deserialize;
use warp::filters::method;
use warp::reject::Reject;
use warp::reply::{self, Response};
use warp::{filters, Filter, Rejection, Reply};

use crate::error::{Error, ErrorCell};
use crate::{crypto, database, models, ui};

pub const COOKIE_NAME : &str = "ear7h-token";

type DynReply = Result<Box<dyn Reply>, Infallible>;
type AuthzReply<T> = Result<T, Rejection>;

pub struct ServerInner {
    pub server_name :  String,
    pub token_secret : Vec<u8>,
    pub db :           database::Db,
    pub render :       ui::Renderer,
}

pub type Server = Arc<ServerInner>;

fn with_server(
    server : &Server,
) -> impl Filter<Extract = (Server,), Error = Infallible> + Clone {
    let f = |server : Server| warp::any().map(move || Arc::clone(&server));

    (f)(Arc::clone(server))
}

fn with_authn(
    server : &Server,
) -> impl Filter<Extract = (models::User,), Error = Rejection> + Clone {
    warp::any()
        .and(with_server(server))
        .and(filters::cookie::optional(COOKIE_NAME))
        .and_then(
            async move |server : Server,
                        cookie_str : Option<String>|
                        -> Result<models::User, Rejection> {
                let value =
                    cookie_str.as_ref().ok_or(Error::FailedLogin).and_then(
                        |s| s.split(",").next().ok_or(Error::FailedLogin),
                    )?;

                let tok = crypto::Token::validate(
                    value,
                    &server.token_secret,
                    &server.server_name,
                )
                .map_err(|_| Error::FailedLogin)?;

                let user_id =
                    u32::from_str(&tok.sub).map_err(|_| Error::FailedLogin)?;

                Ok(server.db.get_user(user_id).await?)
            },
        )
}

type BoxReply = Box<dyn Reply>;
type HandlerResult = Result<BoxReply, Infallible>;

macro_rules! handler {
    ($name:ident ( $($aname:ident : $atype:ty),*) $body:block) => {
        pub fn $name (
            $(
                $aname : $atype,
            )*
        ) -> impl Filter<Extract = (BoxReply,) , Error = Rejection> + Clone {
            $body
        }
    }
}

macro_rules! handler_or{
    ($head:expr $(, $tail:expr)*) => {
        $head
        $(
            .or($tail)
            .unify()
            .boxed()
        )*
    };
    ($head:expr $(, $tail:expr)*,) => {
        handler_or!($head $(, $tail)*)
    }
}

pub fn routes(
    server : &Server,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    handler_or!(
        get_login(server),
        post_login(server),
        get_users_links(server),
        post_users_links(server),
    )
    .recover(async move |err : Rejection| -> Result<Error, Rejection> {
        use Error::*;
        let err = if err.is_not_found() {
            RouteNotFound
        } else if let Some(err) = err.find(): Option<&ErrorCell> {
            err.take().await.unwrap_or(Internal)
        } else {
            eprintln!("{:?}", err);
            Internal
        };

        Ok(err)
    })
    .with(warp::log::custom(|info| {
        eprintln!(
            "{} {} {} in {:?}",
            info.status(),
            info.method(),
            info.path(),
            info.elapsed()
        );
    }))
}

handler! { post_users_links (server : &Server) {
    #[derive(Deserialize)]
    struct PostLinksForm{
        links : String,
    }

    warp::path!("api" / "users" / u32 / "links").map(Some)
        .or(warp::path!("api" / "users" / "self" / "links").map(|| None))
        .unify()
        .and(method::post())
        .and(with_authn(server))
        .and_then(async move |
                  user_id : Option<u32>,
                  token : models::User
        | -> AuthzReply<u32> {
            match user_id {
                Some(id) if id != token.id => Err(Error::Unauthorized.into()),
                Some(id) => Ok(id),
                None => Ok(token.id),
            }
        })
        .and(with_server(server))
        .and(filters::body::form())
        .and_then(async move |
              user_id : u32,
              server : Server,
              body : PostLinksForm
        | -> DynReply {

            for line in body.links.lines() {
                let u = match url::Url::parse(&line) {
                    Ok(u) => u,
                    Err(_) => {
                        return Ok(box Error::InvalidUrl(line.to_string()))
                    }
                };

                match server.db.insert_link(user_id, u.as_str()).await {
                    Ok(_) | Err(Error::DuplicateUrl(_)) => {},
                    Err(err) => return Ok(box err),
                }
            }

            let links = match server.db.get_links(user_id).await {
                Ok(links) => links,
                Err(e) => return Ok(box e),
            };

            let user = match server.db.get_user(user_id).await {
                Ok(user) => user,
                Err(e) => return Ok(box e),
            };

            let html = server.render
                .users_links(&user, links.as_slice(), true);

            Ok(box reply::html(html))
        })

}}

handler! { get_users_links (server : &Server) {
    warp::path!("api" / "users" / u32 / "links").map(Some)
        .or(warp::path!("api" / "users" / "self" / "links").map(|| None))
        .unify()
        .and(method::get())
        .and(with_authn(server))
        .and_then(async move |
            user_id : Option<u32>,
            token : models::User
        | -> AuthzReply<u32> {
            match user_id {
                Some(id) if id != token.id => Err(Error::Unauthorized.into()),
                Some(id) => Ok(id),
                None => Ok(token.id),
            }
        })
        .and(with_server(server))
        .and_then(async move |user_id : u32, server : Server| -> DynReply {

            let links = match server.db.get_links(user_id).await {
                Ok(links) => links,
                Err(e) => return Ok(box Error::from(e)),
            };

            let user = match server.db.get_user(user_id).await {
                Ok(user) => user,
                Err(e) => return Ok(box e),
            };

            let html = server.render
                .users_links(&user, links.as_slice(), true);

            Ok(box reply::html(html))
        })
}}

handler! { get_login (server : &Server) {
    warp::path!("api" / "login")
        .and(method::get())
        .and(with_server(server))
        .and_then(async move |server : Server| -> DynReply {
            let html = server.render.login();

            Ok(box reply::html(html))
        })
}}

handler! { post_login (server : &Server) {
    #[derive(Deserialize)]
    struct Req {
        username : String,
        password : String,
    }

    struct Res {
        token : String,
    }

    impl Reply for Res {
        fn into_response(self) -> Response {
            let cookie = Cookie::build(COOKIE_NAME, self.token.clone())
                .http_only(true)
                .same_site(SameSite::Strict)
                .path("/")
                .finish()
                .to_string();
            dbg!(&cookie);

            http::response::Builder::new()
                .header(header::LOCATION, "/api/users/self/links")
                .header(header::SET_COOKIE, cookie)
                .status(StatusCode::SEE_OTHER)
                .body("redirecting".into()).unwrap()
        }
    }

    warp::path!("api" / "login")
        .and(method::post())
        .and(with_server(server))
        .and(filters::body::form())
        .and_then(async move |server : Server, body : Req| -> HandlerResult {
            let user = match server.db.get_user_by_name(&body.username).await {
                Ok(u) => u,
                Err(err) => return Ok(box err),
            };

            match crypto::verify_password(
                &user.password,
                body.password.as_bytes(),
            ) {
                Ok(true) => {},
                Ok(false) => return Ok(box Error::FailedLogin),
                Err(err) => return Ok(box err),
            }

            const DAYS : u64 = 60 * 60 * 24;

            let tok = crypto::Token{
                iss : server.server_name.to_string(),
                aud : server.server_name.to_string(),
                sub : user.id.to_string(),
                version : user.token_version,
            }.issue(
                &server.token_secret,
                std::time::Duration::from_secs(DAYS * 30),
            );

            let tok = match tok {
                Ok(tok) => tok,
                Err(err) => return Ok(box err),
            };

            Ok(box Res{
                token : tok,
            })
        })
}}

impl Reject for ErrorCell {}

impl From<Error> for Rejection {
    fn from(err : Error) -> Rejection {
        ErrorCell::new(err).into()
    }
}

impl Reply for Error {
    fn into_response(self) -> warp::reply::Response {
        use http::StatusCode as S;
        use Error::*;

        eprintln!("{:?}", &self);

        let res = http::response::Builder::new();

        match self {
            InvalidUrl(s) => res
                .status(S::BAD_REQUEST)
                .body(format!("invalid url: {}", s).into()),
            DuplicateUrl(s) => res
                .status(S::CONFLICT)
                .body(format!("duplicate url: {}", s).into()),
            RouteNotFound => res
                .status(S::NOT_FOUND)
                .body(format!("route not found").into()),
            FailedLogin => res
                .status(S::UNAUTHORIZED)
                .body(include_str!("../ui/failed-login.html").into()),
            _ => res
                .status(S::INTERNAL_SERVER_ERROR)
                .body("internal server error".into()),
        }
        .unwrap()
    }
}
