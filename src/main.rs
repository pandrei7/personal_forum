#![feature(proc_macro_hygiene, decl_macro)]

use rocket::*;

#[get("/")]
fn index() -> &'static str {
    "It works"
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
