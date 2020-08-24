#![feature(proc_macro_hygiene, decl_macro)]

use std::path::{Path, PathBuf};

use rocket::response::status::NotFound;
use rocket::response::NamedFile;
use rocket::*;

#[get("/")]
fn index() -> Result<NamedFile, NotFound<String>> {
    static_file(PathBuf::from("index.html"))
}

#[get("/<file..>")]
fn static_file(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|err| NotFound(err.to_string()))
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, static_file])
        .launch();
}
