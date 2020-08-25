#![feature(proc_macro_hygiene, decl_macro)]

mod admins;
mod sessions;
mod users;

use std::path::{Path, PathBuf};

use rocket::request::Form;
use rocket::response::status::NotFound;
use rocket::response::{Flash, NamedFile, Redirect};
use rocket::*;

use admins::{Admin, AdminLogin, AdminsDbConn};
use sessions::{Session, SessionFairing, SessionsDbConn};

#[get("/")]
fn index() -> Result<NamedFile, NotFound<String>> {
    static_file(PathBuf::from("index.html"))
}

#[get("/admin_login", rank = 1)]
fn admin_login_for_admin(_admin: Admin) -> Flash<Redirect> {
    Flash::warning(
        Redirect::to("/admin_pane"),
        "You are already logged in as admin.",
    )
}

#[post("/admin_login", data = "<login>")]
fn admin_login(
    mut session: Session,
    login: Form<AdminLogin>,
    sessions_conn: SessionsDbConn,
    admins_conn: AdminsDbConn,
) -> Result<Redirect, Flash<Redirect>> {
    let valid = match login.is_valid(&*admins_conn) {
        Ok(valid) => valid,
        _ => {
            return Err(Flash::error(
                Redirect::to("/admin_login"),
                "Credentials as invalid.",
            ))
        }
    };

    if valid {
        if session.make_admin(&*sessions_conn) {
            Ok(Redirect::to("/admin_pane"))
        } else {
            Err(Flash::error(
                Redirect::to("/admin_login"),
                "Could not log you in as admin.",
            ))
        }
    } else {
        Err(Flash::error(
            Redirect::to("/admin_login"),
            "Credentials are invalid.",
        ))
    }
}

#[get("/admin_login", rank = 2)]
fn admin_login_for_non_admin() -> Result<NamedFile, NotFound<String>> {
    static_file(PathBuf::from("admin_login.html"))
}

#[get("/admin_pane", rank = 1)]
fn admin_pane_for_admin(_admin: Admin) -> Result<NamedFile, NotFound<String>> {
    static_file(PathBuf::from("admin_pane.html"))
}

#[get("/admin_pane", rank = 2)]
fn admin_pane_for_non_admin() -> Flash<Redirect> {
    Flash::error(
        Redirect::to("/admin_login"),
        "You do not have permission to access this page.",
    )
}

#[get("/<file..>", rank = 6)]
fn static_file(file: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    NamedFile::open(&path).map_err(|err| NotFound(err.to_string()))
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes![
                admin_login,
                admin_login_for_admin,
                admin_login_for_non_admin,
                admin_pane_for_admin,
                admin_pane_for_non_admin,
                index,
                static_file
            ],
        )
        .attach(SessionFairing::default())
        .attach(AdminsDbConn::fairing())
        .attach(SessionsDbConn::fairing())
}

fn main() {
    rocket().launch();
}
