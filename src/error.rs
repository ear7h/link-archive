use tokio::sync::Mutex;

use quick_from::QuickFrom;

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

    #[quick_from]
    Sqlite(rusqlite::Error),

    #[quick_from]
    Argon2(argon2::Error),

    #[quick_from]
    Time(std::time::SystemTimeError),

    #[quick_from]
    Jwt(jsonwebtoken::errors::ErrorKind),

    RouteNotFound,
    Internal,
}

/// RejectError lets us take an owned cell from a Rejection
#[derive(Debug)]
pub struct ErrorCell(Mutex<Option<Error>>);

impl ErrorCell {
    pub fn new(err : Error) -> Self {
        Self(Mutex::new(Some(err)))
    }

    pub async fn take(&self) -> Option<Error> {
        self.0.lock().await.take()
    }
}

