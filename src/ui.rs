use handlebars::Handlebars;
use serde::Serialize;

use super::*;

pub struct Renderer(Handlebars<'static>);

impl Renderer {
    pub fn new() -> Self {
        let mut t = Handlebars::new();
        t.set_strict_mode(true);
        t.set_dev_mode(true);

        macro_rules! register {
            ($(($name:expr, $path:expr))*) => {

                $(
                    #[cfg(not(debug_assertions))]
                    t.register_template_string(
                        $name,
                        include_str!($path),
                    ).unwrap();

                    // TODO: find find the project root (git rev-parse --show-toplevel)
                    #[cfg(debug_assertions)]
                    t.register_template_file(
                        $name,
                        $path,
                    ).unwrap();
                )*
            };
        }

        register! {
            ("users-links", "../ui/users-links.html")
        }

        Self(t)
    }

    pub fn users_links(
        &self,
        user : &models::User,
        links : &[models::Link],
        editor : bool,
    ) -> String {
        #[derive(Serialize)]
        struct Ctx<'a> {
            user :   &'a models::User,
            links :  &'a [models::Link],
            editor : bool,
        }

        self.0.render("users-links", &Ctx {
            user,
            links,
            editor,
        })
        .unwrap()
    }

    pub fn login(&self) -> &'static str {
        include_str!("../ui/login.html")
    }
}
