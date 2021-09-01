use link_archive::*;

#[tokio::main]
async fn main() {
    let db = database::Db::new("links.sqlite3").unwrap();

    let mut args = std::env::args();

    args.next().unwrap();

    let name = args.next().unwrap();
    let password = args.next().unwrap();

    let password_enc = crypto::encode_password(password.as_bytes()).unwrap();

    db.insert_user(&name, &password_enc).await.unwrap();
}
