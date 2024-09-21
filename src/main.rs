//! This is a server program for a forum-like web application.
//!
//! It is based on the [Rocket](https://rocket.rs/) framework, and it implements
//! a method of sending forum messages to clients which minimizes the amount
//! of data transferred for this.
//!
//! ## Running the server
//!
//! The server needs a `PostgreSQL` database to run. You should create one and
//! pass its path through an environment variable.
//!
//! ```bash
//! ROCKET_DATABASES={db={url=YOUR_DB_PATH}} cargo run --release
//! ```
//!
//! If you are developing, you should probably **omit the `--release` flag**,
//! to disable features like static file caching.
//!
//! For convenience, you can add your database path into `Rocket.toml`, like so:
//!
//! ```toml
//! [global.databases]
//! db = { url = "YOUR_DB_PATH" }
//! ```
//!
//! This lets you run the server without setting the environment variable.
//!
//! The port on which the server runs can be set similarly, through the
//! `ROCKET_PORT` environment variable, or in `Rocket.toml`, like so:
//!
//! ```toml
//! [debug]
//! port = 8000
//!
//! [release]
//! port = 80
//! ```

mod admins;
mod constraints;
mod db;
mod messages;
mod rooms;
mod sessions;
mod static_resources;
mod template_variables;
mod users;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::request::FlashMessage;
use rocket::response::status::NotFound;
use rocket::response::{Flash, Redirect};
use rocket::serde::json::Json;
use rocket::*;
use rocket_dyn_templates::Template;

use admins::{Admin, AdminLogin};
use constraints::RoomName;
use db::{DbConn, DbInitFairing};
use messages::{Message, MessageJson, Updates};
use rooms::{Room, RoomLogin};
use sessions::{Session, SessionFairing};
use static_resources::StaticFile;
use template_variables::WelcomeMessage;

#[get("/")]
fn index(flash: Option<FlashMessage>, welcome_message: WelcomeMessage) -> Template {
    // Populate the template.
    let mut context = HashMap::new();
    context.insert("welcome_message", welcome_message.0);
    context.insert(
        "info",
        flash
            .map(|flash| flash.message().to_string())
            .unwrap_or_else(|| "".into()),
    );
    Template::render("index", &context)
}

#[get("/admin_login")]
fn admin_login_page(flash: Option<FlashMessage>) -> Template {
    // Populate the template.
    let mut context = HashMap::new();
    context.insert(
        "info",
        flash
            .map(|flash| flash.message().to_string())
            .unwrap_or_else(|| "".into()),
    );
    Template::render("admin_login", &context)
}

#[post("/admin_login", format = "form", data = "<login>")]
async fn admin_login(
    mut session: Session,
    login: Form<AdminLogin>,
    conn: DbConn,
) -> Result<Redirect, Flash<Redirect>> {
    match conn.run(move |c| login.is_valid(c)).await {
        Ok(true) => (),
        _ => {
            return Err(Flash::error(
                Redirect::to("/admin_login"),
                "Your credentials are invalid.",
            ))
        }
    };

    if conn.run(move |c| session.make_admin(c)).await {
        Ok(Redirect::to("/admin_pane"))
    } else {
        Err(Flash::error(
            Redirect::to("/admin_login"),
            "Could not log you in as admin.",
        ))
    }
}

#[get("/admin_pane", rank = 1)]
async fn admin_pane_for_admin(_admin: Admin) -> Result<StaticFile, NotFound<String>> {
    static_file(PathBuf::from("admin_pane.html")).await
}

#[get("/admin_pane", rank = 2)]
fn admin_pane_for_non_admin() -> Flash<Redirect> {
    Flash::error(
        Redirect::to("/admin_login"),
        "You do not have permission to access this page.",
    )
}

#[get("/session_count")]
async fn session_count(_admin: Admin, conn: DbConn) -> Result<String, Status> {
    conn.run(Session::count_sessions)
        .await
        .map(|num| num.to_string())
        .map_err(|_| Status::InternalServerError)
}

#[get("/welcome_message")]
fn welcome_message(_admin: Admin, message: WelcomeMessage) -> String {
    message.0
}

#[post("/change_welcome_message", format = "plain", data = "<message>")]
async fn change_welcome_message(_admin: Admin, message: WelcomeMessage, conn: DbConn) -> String {
    match conn.run(move |c| message.save_to_db(c)).await {
        Ok(_) => "Saved your message succesfully.".into(),
        _ => "Could not save your welcome message.".into(),
    }
}

#[get("/active_rooms")]
async fn active_rooms(_admin: Admin, conn: DbConn) -> Result<Json<Vec<String>>, Status> {
    conn.run(Room::active_rooms)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[post("/create_room", format = "form", data = "<room>")]
async fn create_room(_admin: Admin, room: Form<RoomLogin>, conn: DbConn) -> String {
    // Validate the input.
    if let Err(reason) = RoomName::parse(&room.name) {
        return reason;
    }
    if room.password.is_empty() {
        return "The password cannot be empty.".into();
    }

    let name = &room.name;
    let hashed_password = rooms::hash_password(&room.password);

    match conn
        .run({
            let name = name.to_string();
            move |c| Room::create_room(c, name, hashed_password)
        })
        .await
    {
        Ok(_) => format!("Created room {}.", name),
        _ => "Could not create the room.".into(),
    }
}

#[delete("/delete_room", data = "<name>")]
async fn delete_room(_admin: Admin, name: RoomName, conn: DbConn) -> String {
    let name = name.0;

    match conn
        .run({
            let name = name.clone();
            move |c| Room::delete_room(c, &name)
        })
        .await
    {
        Ok(_) => format!("Room {} deleted successfully.", &name),
        Err(reason) => reason,
    }
}

#[post("/change_room_password", format = "form", data = "<form>")]
async fn change_room_password(_admin: Admin, form: Form<RoomLogin>, conn: DbConn) -> String {
    // Validate the input.
    if form.password.is_empty() {
        return "The password cannot be empty.".into();
    }

    let name = form.name.clone();
    let hashed_password = rooms::hash_password(&form.password);

    match conn
        .run(move |c| Room::change_password(c, &name, &hashed_password))
        .await
    {
        Ok(_) => "The password has been changed.".into(),
        _ => "There was an error.".into(),
    }
}

#[post("/enter_room", format = "form", data = "<login>")]
async fn enter_room(
    login: Form<RoomLogin>,
    session: Session,
    conn: DbConn,
) -> Result<Redirect, Flash<Redirect>> {
    if !conn
        .run({
            let login = login.clone();
            move |c| login.can_log_in(c)
        })
        .await
        .unwrap_or(false)
    {
        return Err(Flash::error(
            Redirect::to("/"),
            "Your credentials are invalid.",
        ));
    }

    conn.run({
        let login = login.clone();
        move |c| session.save_room_attempt(c, &login.name, &rooms::hash_password(&login.password))
    })
    .await
    .map(|_| Redirect::to(format!("/room/{}", login.name)))
    .map_err(|_| Flash::error(Redirect::to("/"), "Could not save your login attempt."))
}

#[get("/room/<name>")]
fn room(name: RoomName, room: Option<Room>) -> Result<Template, Flash<Redirect>> {
    if room.is_none() {
        return Err(Flash::error(
            Redirect::to("/"),
            "Your credentials are invalid.",
        ));
    }

    // Populate the room template.
    let mut context = HashMap::new();
    context.insert("name", name);
    Ok(Template::render("room", &context))
}

#[get("/room/<name>/updates")]
async fn get_message_updates(
    name: RoomName,
    room: Option<Room>,
    session: Session,
    conn: DbConn,
) -> Result<Json<Updates>, Status> {
    let room = room.ok_or(Status::Unauthorized)?;
    let name = name.0;

    let last_update = conn
        .run({
            let name = name.clone();
            let session = session.clone();
            move |c| session.get_room_update(c, &name)
        })
        .await
        .unwrap_or(0);
    let now = Message::current_timestamp();

    let updates = conn
        .run(move |c| room.get_updates_between(c, last_update, now))
        .await
        .map_err(|_| Status::InternalServerError)?;
    conn.run(move |c| session.save_room_update(c, &name, now))
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(updates))
}

#[post("/room/<_name>/post", format = "json", data = "<message>")]
async fn post(
    _name: RoomName,
    room: Option<Room>,
    message: Json<MessageJson>,
    session: Session,
    conn: DbConn,
) -> Result<String, Status> {
    let room = room.ok_or(Status::Unauthorized)?;
    let message = message.into_inner();

    if message.content.is_empty() {
        return Ok("Your message cannot be empty.".into());
    }
    if message.content.len() > constraints::MAX_MESSAGE_LEN {
        return Ok("Your message is too long.".into());
    }

    conn.run(move |c| room.add_message(c, message.content, session.id(), message.reply_to))
        .await
        .map(|_| "Your message has been saved.".into())
        .map_err(|_| Status::InternalServerError)
}

#[get("/colors")]
async fn colors() -> Result<StaticFile, NotFound<String>> {
    static_file(PathBuf::from("colors.html")).await
}

#[get("/static/<file..>")]
async fn static_file(file: PathBuf) -> Result<StaticFile, NotFound<String>> {
    let path = Path::new("static/").join(file);
    Ok(StaticFile(
        NamedFile::open(&path)
            .await
            .map_err(|err| NotFound(err.to_string()))?,
    ))
}

#[catch(404)]
async fn not_found() -> Result<StaticFile, NotFound<String>> {
    static_file(PathBuf::from("404.html")).await
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                active_rooms,
                admin_login,
                admin_login_page,
                admin_pane_for_admin,
                admin_pane_for_non_admin,
                change_room_password,
                change_welcome_message,
                colors,
                create_room,
                delete_room,
                enter_room,
                get_message_updates,
                index,
                post,
                room,
                session_count,
                static_file,
                welcome_message,
            ],
        )
        .register("/", catchers![not_found, sessions::session_expired])
        .attach(Template::fairing())
        .attach(DbConn::fairing())
        .attach(DbInitFairing)
        .attach(SessionFairing)
}
