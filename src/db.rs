//! Module for interacting with the database.
//!
//! The app uses one database with many tables. This module provides
//! data types and fairings to connect to this database, and to correctly
//! set it up when starting.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::*;
use rocket_contrib::databases::postgres::{self, Connection};
use rocket_contrib::*;

/// A connection to the database.
#[database("db")]
pub struct DbConn(Connection);

/// A fairing which makes sure we can interact with the database correctly.
#[derive(Default)]
pub struct DbInitFairing;

impl DbInitFairing {
    /// Initializes the database, as it should be when starting the server.
    ///
    /// It drops tables which should not be persistent, and makes sure
    /// all tables needed for the server to function are set up correctly.
    fn init_db(&self, conn: &Connection) -> postgres::Result<()> {
        conn.batch_execute(
            "DROP TABLE IF EXISTS sessions CASCADE;
            DROP TABLE IF EXISTS room_attempts;
            DROP TABLE IF EXISTS room_updates;

            CREATE TABLE IF NOT EXISTS admins (
                username TEXT PRIMARY KEY,
                password TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id          TEXT PRIMARY KEY,
                last_update BIGINT NOT NULL,
                is_admin    BOOLEAN NOT NULL
            );
            CREATE TABLE IF NOT EXISTS rooms (
                name     TEXT PRIMARY KEY,
                password TEXT NOT NULL,
                table_id SERIAL NOT NULL,
                creation BIGINT NOT NULL
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
                timestamp BIGINT NOT NULL,
                PRIMARY KEY (id, name),
                FOREIGN KEY (id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (name) REFERENCES rooms(name) ON DELETE CASCADE
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

/// Executes a database query which should return exactly one row.
///
/// This is a convenience macro which makes sure that a given query
/// returns only one row. You are also allowed to map that row using
/// a function - `$row_map`. The macro gets evaluated to a value
/// of type postgres::Result<T>, where T is the type returned by the
/// map function. If the query does not return exactly one row, the
/// operation is considered to have failed.
#[macro_export]
macro_rules! query_one_row {
    ($conn: expr, $sql:expr, $params:expr, $row_map:expr) => {{
        use rocket_contrib::databases::postgres;
        use std::io::{Error, ErrorKind};

        match $conn.query($sql, $params) {
            Ok(rows) if rows.len() == 1 => Ok($row_map(rows.get(0))),
            Ok(_) => {
                let io_err = Error::new(ErrorKind::Other, "Wrong number of rows");
                Err(postgres::Error::from(io_err))
            }
            Err(err) => Err(err),
        }
    }};
}

/// Executes a database query and maps a given function over the returned rows.
///
/// This is a convenience macro which both queries the database, and maps
/// a function over the rows. The result is an iterator over the mapped rows.
/// If the query fails, the macro returns the error early.
#[macro_export]
macro_rules! query_and_map {
    ($conn: expr, $sql:expr, $params:expr, $row_map:expr) => {{
        $conn.query($sql, $params)?.into_iter().map($row_map)
    }};
}
