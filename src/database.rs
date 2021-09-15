use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{ffi, Connection};
use tokio::sync::Mutex;

use crate::time_utils::TIME_FORMAT;
use crate::{models, Error, Result};

fn error_code_match(
    err : &rusqlite::Error,
    code : ffi::ErrorCode,
    ext : i64,
) -> bool {
    matches!(
            err,
            rusqlite::Error::SqliteFailure(e, _)
                if e.code == code
                && i64::from(e.extended_code) == ext)
}

macro_rules! db_method {
        ($name:ident (
            &$self:ident,
            $conn:ident,
            $($pname:ident : $ptype:ty),*
        ) -> $ret:ty $body:block ) => {
            pub async fn $name (&$self, $( $pname : $ptype, )* ) -> $ret {
                let $conn = $self.conn.lock().await;
                tokio::task::block_in_place(|| $body)
            }
        }
    }

pub struct Db {
    conn : Mutex<Connection>,
}

impl Db {
    pub fn new<P : AsRef<std::path::Path>>(p : P) -> Result<Self> {
        let conn = Connection::open(p)?;

        conn.pragma_update(None, "foreign_keys", &"ON")?;

        Ok(Self {
            conn : Mutex::new(conn),
        })
    }

    db_method! {insert_user(
        &self,
        conn,
        name : &str,
        password : &str
    ) -> Result<()> {
        conn
            .prepare_cached("INSERT INTO users (name, password) VALUES (?, ?)")?
            .execute(rusqlite::params![name, password])
            .map_err(|err| {
                if error_code_match(
                    &err,
                    ffi::ErrorCode::ConstraintViolation,
                    2067
                ) {
                    Error::DuplicateName(name.to_string())
                } else {
                    err.into()
                }
            })?;
        Ok(())
    }}

    db_method! {get_user(&self, conn, user_id : u32) -> Result<models::User> {
        let mut stmt = conn
            .prepare_cached("SELECT * FROM users WHERE users.id = ?")?;

        let mut rows = stmt.query(rusqlite::params![user_id])?;

        let row = rows.next()?
            .ok_or(Error::UserIdNotFound(user_id))?;

        Ok(row_parse(row)?)
    }}

    db_method! {get_user_by_name(
        &self,
        conn,
        username : &str
    ) -> Result<models::User> {
        let mut stmt = conn
            .prepare_cached("SELECT * FROM users WHERE users.name = ?")?;

        let mut rows = stmt.query(rusqlite::params![username])?;

        let row = rows.next()?
            .ok_or(Error::UserNameNotFound(username.to_string()))?;

        Ok(row_parse(row)?)
    }}

    db_method! {insert_link(
        &self,
        conn,
        user_id : u32,
        link : &str
    ) -> Result<()> {
        conn
            .prepare_cached("INSERT INTO links (user_id, url) VALUES (?, ?)")?
            .execute(rusqlite::params![user_id, link])
            .map_err(|err| {
                if error_code_match(
                    &err,
                    ffi::ErrorCode::ConstraintViolation,
                    1555
                ) {
                    Error::DuplicateUrl(link.to_string())
                } else {
                    err.into()
                }
            })?;
        Ok(())
    }}

    db_method! {get_links(
        &self,
        conn,
        user_id : u32
    ) -> Result<Vec<models::Link>> {
        let mut stmt = conn
            .prepare_cached("SELECT * FROM links WHERE links.user_id = ?")?;

        let mut rows = stmt
            .query(rusqlite::params![user_id])?;

        let mut links = Vec::new();
        while let Some(row) = rows.next()? {
            links.push(row_parse::<models::Link>(row)?);
        }

        Ok(links)
    }}
}

struct Row<'a> {
    off :   usize,
    inner : &'a rusqlite::Row<'a>,
    cols :  Vec<&'a str>,
}

impl<'a> From<&'a rusqlite::Row<'a>> for Row<'a> {
    fn from(r : &'a rusqlite::Row<'a>) -> Row<'a> {
        Row {
            off :   0,
            cols :  r.column_names(),
            inner : r,
        }
    }
}

fn row_parse<'a, T : FromRow>(row : &'a rusqlite::Row<'a>) -> Result<T> {
    T::from_row(&mut row.into())
}

impl<'a> Row<'a> {
    fn column_names(&self) -> &[&'a str] {
        &self.cols
    }

    fn get<T : FromSql>(&self, idx : usize) -> rusqlite::Result<T> {
        self.inner.get(idx + self.off)
    }

    fn advance(&mut self, n : usize) {
        self.off += n;
    }
}

trait FromRow: Sized {
    fn column_count() -> usize;
    fn from_row(row : &mut Row) -> Result<Self>;
}

impl<T, U> FromRow for (T, U)
where
    T : FromRow,
    U : FromRow,
{
    fn column_count() -> usize {
        T::column_count() + U::column_count()
    }

    fn from_row(row : &mut Row) -> Result<Self> {
        let t = T::from_row(row)?;
        row.advance(T::column_count());
        let u = U::from_row(row)?;

        Ok((t, u))
    }
}

macro_rules! impl_from_row {
        ($table:ident, $ty:ty { $($field:ident),* }) => {

            impl FromRow for $ty {
                fn column_count() -> usize {
                    const N : usize = [
                        $(
                            stringify!($field),
                        )*
                    ].len();

                    N
                }

                fn from_row(row : &mut Row) -> Result<$ty> {
                    fn find(slc : &[&str], s : &str) -> Option<usize> {
                        for (i, v) in slc.iter().enumerate() {
                            if v == &s {
                                return Some(i)
                            }
                        }

                        None
                    }

                    let cols = row.column_names();

                    let m = &[
                        $(
                            find(&cols, stringify!($field)),
                        )*
                    ];

                    let mut it = m.iter().copied();


                    Ok(Self{
                    $(
                        $field : row.get(it.next().unwrap().unwrap())?,
                    )*
                    })
                }
            }
        }
    }

impl_from_row! {users, models::User {
    id, name, password, token_version, created, deleted
}}

impl_from_row! {links, models::Link {
    user_id, url, created, deleted
}}

impl FromSql for models::Time {
    fn column_result(value : ValueRef) -> FromSqlResult<models::Time> {
        let s : String = String::column_result(value)?;

        let dt = time::PrimitiveDateTime::parse(&s, &TIME_FORMAT)
            .map_err(|err| FromSqlError::Other(Box::new(err)))?;

        Ok(dt.assume_offset(time::UtcOffset::UTC).into())
    }
}
