
use link_archive::api;

#[tokio::main]
async fn main() {
    println!("starting server");

    let args = std::env::args().collect::<Vec<_>>();
    let config = match &args[..] {
        [_, config] => config,
        _ => {
            eprintln!("usage: ./link-archive config.json");
            std::process::exit(1);
        }
    };

    let (server, addr) = api::new_server(&config).unwrap();
    http_mux::hyper::serve_addr(api::routes(server), &addr).await.unwrap();
}


