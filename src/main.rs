use std::sync::Arc;

use link_archive::*;

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

    let routes = api::routes(&server);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
