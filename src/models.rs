use serde::Serialize;

pub type Time = crate::time_utils::Time;

#[derive(Debug, Serialize)]
pub struct User {
    pub id :            u32,
    pub name :          String,
    pub password :      String,
    pub token_version : u32,
    pub created :       Time,
    pub deleted :       Option<Time>,
}

#[derive(Debug, Serialize)]
pub struct Link {
    pub user_id : u32,
    pub url :     String,
    pub created : Time,
    pub deleted : Option<Time>,
}
