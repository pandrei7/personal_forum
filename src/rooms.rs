//! Module for working with rooms.
//!
//! Rooms contain threads of messages, and are password protected. Each
//! room has a unique name which cannot be changed after its creation.
//! Passwords should be changeable to allow for easier management.
//!
//! Information about rooms such as their name and password is held
//! in a special database. Apart from this "central" one, each room
//! keeps its messages in a separate database.
//!
//! The rooms module contributes to the incremental-updates mechanism, which
//! should help decrease network traffic by avoiding the resending of the
//! entire message database content repeatedly. To achieve this, the `Room`
//! struct allows retrieving updates only for given time intervals. Each room
//! remembers the timestamp of its creation to avoid issues which might appear
//! when deleting and recreating rooms (for example, an user of the old room
//! might interact with its cached webpage, which could pose some issues).

use std::fs;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::request::{self, FromRequest, Request};
use rocket::Rocket;
use rocket::*;
use rocket_contrib::databases::rusqlite::{self, Connection};
use rocket_contrib::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::messages::{self, Message, Updates};
use crate::sessions::{Session, SessionsDbConn};

/// The path of the rooms database.
const DB_PATH: &str = "db/rooms.db";

/// Returns the hash of a password, as it should be stored in databases.
///
/// Passwords should be stored as SHA-256 hashes.
pub fn hash_password(password: &str) -> String {
    format!("{:x}", Sha256::digest(password.as_bytes()))
}

/// Holds a connection to the rooms database.
///
/// It's a type needed to interact with Rocket.
#[database("rooms")]
pub struct RoomsDbConn(Connection);

/// Holds relevant information about a room.
///
/// It's tied to a row in the rooms database.
pub struct Room {
    name: String,
    /// The hashed password used to log into the room.
    password: String,
    db_path: String,
    creation: i64,
}

impl Room {
    /// Creates and initializes a room with the given data.
    ///
    /// Each room has a database for its messages.
    /// This database is held in a separate file with path `db_path`.
    /// The path should probably follow the convention `db/rooms/<name>.db`.
    pub fn create_room(
        conn: &Connection,
        name: String,
        hashed_password: String,
        db_path: String,
    ) -> rusqlite::Result<()> {
        let creation = Message::current_timestamp();
        conn.execute(
            "INSERT INTO rooms (name, password, db_path, creation) VALUES (?1, ?2, ?3, ?4);",
            &[&name, &hashed_password, &db_path, &creation],
        )?;

        let conn = Connection::open(db_path)?;
        Message::setup_db(&conn)?;

        Ok(())
    }

    /// Deletes a room from the database, also removing its message database.
    ///
    /// If the operation fails, the reason is returned as a readable string.
    pub fn delete_room(conn: &Connection, name: &str) -> Result<(), String> {
        let db_path: String = conn
            .query_row(
                "SELECT db_path FROM rooms WHERE name = ?;",
                &[&name],
                |row| row.get(0),
            )
            .map_err(|_| "Error while retrieving db_path.")?;

        match conn.execute("DELETE FROM rooms WHERE name = ?;", &[&name]) {
            Ok(1) => fs::remove_file(db_path)
                .map_err(|_| "Error while deleting messages database.".into()),
            _ => Err("Error while deleting from database.".into()),
        }
    }

    /// Returns a list with the names of all the rooms stored in the database.
    pub fn active_rooms(conn: &Connection) -> rusqlite::Result<Vec<String>> {
        conn.prepare("SELECT name FROM rooms;")?
            .query_map(&[], |row| row.get(0))?
            .collect()
    }

    /// Changes the password of the given room.
    pub fn change_password(
        conn: &Connection,
        name: &str,
        hashed_password: &str,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "UPDATE rooms SET password = ?1 WHERE name = ?2;",
            &[&hashed_password, &name],
        )
        .and(Ok(()))
    }

    /// Returns the next incremental updates a user should receive when requested.
    ///
    /// The timestamps should be given in the format used by the messages database.
    pub fn get_updates_between(&self, last_update: i64, now: i64) -> rusqlite::Result<Updates> {
        // If a user is desynchronized, they should delete the messages of the old room.
        let clean_stored = self.is_desynchronized(last_update);

        let conn = Connection::open(&self.db_path)?;
        let messages = Message::get_between(&conn, last_update, now)?;

        Ok(Updates {
            clean_stored,
            messages,
        })
    }

    /// Adds a new message to the room.
    pub fn add_message(
        &self,
        mut content: String,
        author: String,
        reply_to: Option<i32>,
    ) -> rusqlite::Result<()> {
        let conn = Connection::open(&self.db_path)?;
        messages::prepare_for_storage(&mut content);
        Message::add(&conn, content, author, reply_to)
    }

    /// Checks if a user is desynchronized.
    ///
    /// A user is considered "desynchronized" if the last time they received
    /// updates for this room happened before the room was even created.
    ///
    /// This can happen if, for example, a user is logged into a room which
    /// gets deleted, then recreated. The user might still interact with the
    /// old version of the webpage, which could lead to weird behaviour.
    pub fn is_desynchronized(&self, last_update: i64) -> bool {
        last_update <= self.creation
    }

    /// Tries to retrieve the database entry associated with a room, given its name.
    fn from_db(conn: &Connection, name: &str) -> rusqlite::Result<Room> {
        conn.query_row(
            "SELECT password, db_path, creation FROM rooms WHERE name = ?;",
            &[&name],
            |row| Room {
                name: String::from(name),
                password: row.get(0),
                db_path: row.get(1),
                creation: row.get(2),
            },
        )
    }

    /// Checks if the given password allows access to the room.
    fn valid_password(&self, hashed_password: &str) -> bool {
        self.password == hashed_password
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Room {
    type Error = ();

    /// Since rooms are password-protected, we must make sure a user is
    /// allowed access to a room before it can interact with it. This request
    /// guard should be the main way of checking users' access to a room.
    ///
    /// To test if the permission check failed because the user did not provide
    /// a valid password, you can use an `Option<Room>` field in the function
    /// header, like this:
    ///
    /// ```rust ignore
    /// #[get("/room/<name>")]
    /// fn test_room_access(name: String, room: Option<Room>) -> String {
    ///     if room.is_some() {
    ///         "You have access to this room."
    ///     } else {
    ///         "You do not have acces to this room."
    ///     }
    /// }
    /// ```
    // TODO(pandrei7): Refactor this.
    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        // Try to extract the name of the room.
        let name = {
            let mut segs = req.uri().segments();
            if segs.next() != Some("room") {
                return request::Outcome::Forward(());
            }
            match segs.next() {
                Some(name) => name,
                _ => return request::Outcome::Forward(()),
            }
        };

        // Retrieve the room entry.
        let room = {
            let conn = req.guard::<RoomsDbConn>()?;
            match Room::from_db(&conn, &name) {
                Ok(room) => room,
                _ => return request::Outcome::Forward(()),
            }
        };

        // Find the user's password attempt.
        let hashed_password = {
            let session = req.guard::<Session>()?;
            let sessions_conn = req.guard::<SessionsDbConn>()?;
            match session.get_room_attempt(&sessions_conn, &name) {
                Ok(password) => password,
                _ => return request::Outcome::Forward(()),
            }
        };

        if hashed_password == room.password {
            request::Outcome::Success(room)
        } else {
            request::Outcome::Forward(())
        }
    }
}

/// A fairing used to make interaction with the rooms database possible.
#[derive(Default)]
pub struct RoomFairing;

impl RoomFairing {
    /// Initializes the rooms database.
    ///
    /// The database should be "persistent", meaning that it is not
    /// cleaned on startup of the server program.
    fn setup_db() -> rusqlite::Result<()> {
        let conn = Connection::open(DB_PATH)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS rooms (
                name     TEXT PRIMARY KEY,
                password TEXT NOT NULL,
                db_path  TEXT NOT NULL,
                creation INTEGER NOT NULL
            );",
            &[],
        )
        .and(Ok(()))
    }
}

/// The fairing is responsible for setting up the rooms database.
impl Fairing for RoomFairing {
    fn info(&self) -> Info {
        Info {
            name: "Room Fairing",
            kind: Kind::Attach,
        }
    }

    /// Makes sure that we can interact with the rooms database.
    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        if RoomFairing::setup_db().is_err() {
            eprintln!("Could not setup rooms db.");
            return Err(rocket);
        }

        Ok(rocket)
    }
}

/// The content of a form used to hold login credentials for a room.
#[derive(Deserialize, FromForm)]
pub struct RoomLogin {
    pub name: String,
    /// The plaintext password of the room.
    pub password: String,
}

impl RoomLogin {
    /// Checks if the form contains the correct credentials to log into a room.
    pub fn can_log_in(&self, conn: &Connection) -> rusqlite::Result<bool> {
        let hashed_password = hash_password(&self.password);
        let room = Room::from_db(&conn, &self.name)?;
        Ok(room.valid_password(&hashed_password))
    }
}
