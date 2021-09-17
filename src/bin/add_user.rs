use link_archive::*;

#[tokio::main]
async fn main() {

    let args = std::env::args().collect::<Vec<_>>();

    match &args[..] {
        [_, db, name, password] => {
            let db = database::Db::new(&db).unwrap();
            let password_enc = crypto::encode_password(password.as_bytes()).unwrap();

            db.insert_user(&name, &password_enc).await.unwrap();
        },
        _ => {
            eprintln!("usage: ./add_user db name password");
            std::process::exit(1);
        }
    }
}
