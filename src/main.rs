use std::sync::Arc;
use std::net::SocketAddr;

use link_archive::{database, api, ui};

pub const TOKEN_SECRET : &[u8] = b"super-secret";
pub const SERVER_NAME : &str = "links.ear7h.net";

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let server = Arc::new(api::ServerInner {
        token_secret : TOKEN_SECRET.to_owned(),
        server_name :  SERVER_NAME.to_owned(),
        db :           database::Db::new("links.sqlite3").unwrap(),
        render :       ui::Renderer::new(),
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    http_mux::hyper::serve_addr(api::routes(server), &addr).await.unwrap();
}


