use quick_from::QuickFrom;
use http_mux::mux::MuxError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(QuickFrom, Debug)]
pub enum Error {
    InvalidUrl(String),
    DuplicateUrl(String),
    DuplicateName(String),
    TokenDurationTooBig,
    UserNameNotFound(String),
    UserIdNotFound(u32),
    FailedLogin,
    Unauthorized,
    BadRequest,
    RouteNotFound,
    Internal,

    #[quick_from]
    Sqlite(rusqlite::Error),

    #[quick_from]
    Time(std::time::SystemTimeError),

    #[quick_from]
    Hyper(hyper::Error),

    #[quick_from]
    Json(serde_json::Error),

    #[quick_from]
    Io(std::io::Error),

    #[quick_from]
    Authn(authn::client::Error),
}

impl From<MuxError> for Error {
    fn from(err : MuxError) -> Self {
        use MuxError::*;
        match err {
            NotFound(_) | MethodNotAllowed(_, _) => Error::RouteNotFound,
            Parse(_, _) => Error::BadRequest,
        }
    }
}

