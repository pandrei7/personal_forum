//! Module for working with rooms.
//!
//! Rooms contain threads of messages, and are password protected. Each
//! room has a unique name which cannot be changed after its creation.
//! Passwords should be changeable to allow for easier management.
//!
//! Information about rooms such as their name and password is held
//! in a special database. Apart from this "central" one, each room
//! keeps its messages in a separate database.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::Rocket;
use rocket::*;
use rocket_contrib::databases::rusqlite::{self, Connection};
use rocket_contrib::*;
use sha2::{Digest, Sha256};

use crate::messages::Message;

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

impl RoomsDbConn {
    /// Creates and initializes a room with the given data.
    ///
    /// Each room has a database for its messages.
    /// This database is held in a separate file with path `db_path`.
    /// The path should probably follow the convention `db/rooms/<name>.db`.
    pub fn create_room(
        &self,
        name: String,
        hashed_password: String,
        db_path: String,
    ) -> rusqlite::Result<()> {
        self.execute(
            "INSERT INTO rooms (name, password, db_path) VALUES (?1, ?2, ?3);",
            &[&name, &hashed_password, &db_path],
        )?;

        let conn = Connection::open(db_path)?;
        Message::setup_db(&conn)?;

        Ok(())
    }

    /// Checks if the given credentials allow access to a room.
    pub fn valid_credentials(&self, name: &str, hashed_password: &str) -> rusqlite::Result<bool> {
        let wanted = self.query_row(
            "SELECT password FROM rooms where name = ?",
            &[&name],
            |row| row.get::<usize, String>(0),
        )?;

        Ok(hashed_password == wanted)
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
                db_path  TEXT NOT NULL
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

/// The content of a form used to log into a room.
#[derive(FromForm)]
pub struct RoomLogin {
    pub name: String,
    /// The plaintext password of the room.
    pub password: String,
}

impl RoomLogin {
    /// Checks if the form contains the correct credentials to log into a room.
    pub fn is_valid(&self, conn: &RoomsDbConn) -> rusqlite::Result<bool> {
        let hashed_password = hash_password(&self.password);
        conn.valid_credentials(&self.name, &hashed_password)
    }
}
