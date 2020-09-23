#![feature(proc_macro_hygiene, decl_macro)]

mod admins;
mod messages;
mod rooms;
mod sessions;
mod users;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rocket::http::Status;
use rocket::response::status::NotFound;
use rocket::response::{Flash, NamedFile, Redirect};
use rocket::*;
use rocket_contrib::json::Json;
use rocket_contrib::templates::Template;

use admins::{Admin, AdminLogin, AdminsDbConn};
use messages::{Message, MessageJson, Updates};
use rooms::{Room, RoomFairing, RoomLogin, RoomsDbConn};
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

#[post("/admin_login", format = "json", data = "<login>")]
fn admin_login(
    mut session: Session,
    login: Json<AdminLogin>,
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

#[get("/session_count")]
fn session_count(_admin: Admin, conn: SessionsDbConn) -> Result<String, Status> {
    Session::count_sessions(&conn)
        .map(|num| num.to_string())
        .map_err(|_| Status::InternalServerError)
}

#[get("/active_rooms")]
fn active_rooms(_admin: Admin, conn: RoomsDbConn) -> Result<Json<Vec<String>>, Status> {
    Room::active_rooms(&conn)
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[post("/create_room", format = "json", data = "<room>")]
fn create_room(_admin: Admin, room: Json<RoomLogin>, conn: RoomsDbConn) -> String {
    // Validate the input.
    let valid = |ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
    if room.name.is_empty() || !room.name.chars().all(valid) {
        return "The room name contains invalid characters.".into();
    }
    if room.password.is_empty() {
        return "The password cannot be empty.".into();
    }

    let name = &room.name;
    let hashed_password = rooms::hash_password(&room.password);
    let db_path = {
        let mut path = PathBuf::from("db");
        path.push("rooms");
        path.push(name.clone());
        path.set_extension("db");

        match path.to_str() {
            Some(path) => path.into(),
            _ => return "There was an error with the database path.".into(),
        }
    };

    match Room::create_room(&conn, name.clone(), hashed_password, db_path) {
        Ok(_) => format!("Created room {}.", name),
        _ => "Could not create the room.".into(),
    }
}

#[delete("/delete_room", data = "<name>")]
fn delete_room(_admin: Admin, name: String, conn: RoomsDbConn) -> String {
    match Room::delete_room(&conn, &name) {
        Ok(_) => format!("Room {} deleted successfully.", name),
        Err(reason) => reason,
    }
}

#[post("/change_room_password", format = "json", data = "<form>")]
fn change_room_password(_admin: Admin, form: Json<RoomLogin>, conn: RoomsDbConn) -> String {
    // Validate the input.
    if form.password.is_empty() {
        return "The password cannot be empty.".into();
    }

    let name = &form.name;
    let hashed_password = rooms::hash_password(&form.password);

    match Room::change_password(&conn, &name, &hashed_password) {
        Ok(_) => "The password has been changed.".into(),
        _ => "There was an error.".into(),
    }
}

#[post("/enter_room", format = "json", data = "<login>")]
fn enter_room(
    login: Json<RoomLogin>,
    rooms_conn: RoomsDbConn,
    session: Session,
    sessions_conn: SessionsDbConn,
) -> Result<Redirect, Flash<Redirect>> {
    if !login.can_log_in(&rooms_conn).unwrap_or(false) {
        return Err(Flash::error(
            Redirect::to("/"),
            "Credentials are not valid.",
        ));
    }

    session
        .save_room_attempt(
            &sessions_conn,
            &login.name,
            &rooms::hash_password(&login.password),
        )
        .map_err(|_| Flash::error(Redirect::to("/"), "Could not save your login attempt."))
        .map(|_| Redirect::to(format!("/room/{}", login.name)))
}

#[get("/room/<name>")]
fn room(name: String, room: Option<Room>) -> Result<Template, Flash<Redirect>> {
    if room.is_none() {
        return Err(Flash::error(
            Redirect::to("/"),
            "Credentials are not valid.",
        ));
    }

    // Populate the room template.
    let mut context = HashMap::new();
    context.insert("name", name);
    Ok(Template::render("room", &context))
}

#[get("/room/<name>/updates")]
fn get_message_updates(
    name: String,
    room: Option<Room>,
    session: Session,
    conn: SessionsDbConn,
) -> Result<Json<Updates>, Status> {
    let room = room.ok_or(Status::Unauthorized)?;

    let last_update = session.get_room_update(&conn, &name).unwrap_or(0);
    let now = Message::current_timestamp();

    let updates = room
        .get_updates_between(last_update, now)
        .map_err(|_| Status::InternalServerError)?;
    session
        .save_room_update(&conn, &name, now)
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(updates))
}

#[post("/room/<name>/post", format = "json", data = "<message>")]
fn post(
    name: String,
    room: Option<Room>,
    message: Json<MessageJson>,
    session: Session,
    conn: SessionsDbConn,
) -> Result<String, Status> {
    let room = room.ok_or(Status::Unauthorized)?;
    let message = message.into_inner();

    // Users might not know what they are replying to when desynchronized.
    let last_update = session
        .get_room_update(&conn, &name)
        .map_err(|_| Status::InternalServerError)?;
    if room.is_desynchronized(last_update) {
        return Ok("This room seems to have been deleted. Try refreshing the page.".into());
    }

    room.add_message(message.content, session.id(), message.reply_to)
        .map(|_| "Your message has been saved".into())
        .map_err(|_| Status::InternalServerError)
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
                active_rooms,
                admin_login,
                admin_login_for_admin,
                admin_login_for_non_admin,
                admin_pane_for_admin,
                admin_pane_for_non_admin,
                change_room_password,
                create_room,
                delete_room,
                enter_room,
                get_message_updates,
                index,
                post,
                room,
                session_count,
                static_file,
            ],
        )
        .attach(Template::fairing())
        .attach(SessionFairing::default())
        .attach(RoomFairing::default())
        .attach(AdminsDbConn::fairing())
        .attach(SessionsDbConn::fairing())
        .attach(RoomsDbConn::fairing())
}

fn main() {
    rocket().launch();
}
