//! Module for interacting with the database.
//!
//! The app uses one database with many tables. This module provides
//! data types and fairings to connect to this database, and to correctly
//! set it up when starting.

use core::ops::Deref;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::request::{FromRequest, Outcome};
use rocket::*;
use rocket_contrib::databases::rusqlite::{self, Connection};
use rocket_contrib::*;

/// A connection to the database.
pub struct DbConn(Helper);

impl DbConn {
    /// Retrieves a fairing that initializes the associated database connection pool.
    pub fn fairing() -> impl Fairing {
        Helper::fairing()
    }

    /// Retrieves a connection from the configured pool.
    pub fn get_one(rocket: &Rocket) -> Option<Self> {
        let conn = Self(Helper::get_one(rocket)?);
        conn.activate_foreign_keys().map(|_| conn).ok()
    }

    /// Activates the enforcement of foreign key constraints for this connection.
    fn activate_foreign_keys(&self) -> rusqlite::Result<()> {
        self.0.execute("PRAGMA foreign_keys=ON;", &[]).and(Ok(()))
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    /// Connections to the database should enforce foreign key constraints.
    /// Because we use SQLite, we have to manually activate these checks with
    /// a pragma statement. This statement only applies to one connection,
    /// so each connection should be set up individually.
    fn from_request(req: &'a Request<'r>) -> Outcome<DbConn, Self::Error> {
        let conn = Self(req.guard::<Helper>()?);

        match conn.activate_foreign_keys() {
            Ok(_) => Outcome::Success(conn),
            _ => Outcome::Failure((rocket::http::Status::InternalServerError, ())),
        }
    }
}

impl Deref for DbConn {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A helper-type which holds a connection to the database.
///
/// It exists so that the main database-connection type can profit
/// from default implementations from `rocket_contrib`.
#[database("db")]
struct Helper(Connection);

/// A fairing which makes sure we can interact with the database correctly.
#[derive(Default)]
pub struct DbInitFairing;

impl DbInitFairing {
    /// Initializes the database, as it should be when starting the server.
    ///
    /// It drops tables which should not be persistent, and makes sure
    /// all tables needed for the server to function are set up correctly.
    fn init_db(&self, conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "DROP TABLE IF EXISTS sessions;
            DROP TABLE IF EXISTS room_attempts;
            DROP TABLE IF EXISTS room_updates;

            CREATE TABLE IF NOT EXISTS admins (
                username TEXT PRIMARY KEY,
                password TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id          TEXT PRIMARY KEY,
                last_update INTEGER NOT NULL,
                is_admin    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS room_attempts (
                id       TEXT NOT NULL,
                name     TEXT NOT NULL,
                password TEXT NOT NULL,
                PRIMARY KEY (id, name),
                FOREIGN KEY (id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (name) REFERENCES rooms(name) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS room_updates (
                id        TEXT NOT NULL,
                name      TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                PRIMARY KEY (id, name),
                FOREIGN KEY (id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (name) REFERENCES rooms(name) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS rooms (
                name     TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                table_id INTEGER PRIMARY KEY AUTOINCREMENT,
                creation INTEGER NOT NULL
            );",
        )
    }
}

impl Fairing for DbInitFairing {
    fn info(&self) -> Info {
        Info {
            name: "Database Init Fairing",
            kind: Kind::Attach,
        }
    }

    /// Sets up the database so that the server can start working.
    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        let conn = match DbConn::get_one(&rocket) {
            Some(conn) => conn,
            _ => return Err(rocket),
        };

        match self.init_db(&conn) {
            Ok(_) => Ok(rocket),
            _ => Err(rocket),
        }
    }
}
