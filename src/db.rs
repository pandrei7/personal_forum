//! Module for interacting with the database.
//!
//! The app uses one database with many tables. This module provides
//! data types and fairings to connect to this database, and to correctly
//! set it up when starting.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{fairing, Build, Rocket};
use rocket_sync_db_pools::{database, postgres, rocket};

/// A connection to the database.
#[database("db")]
pub struct DbConn(postgres::Client);

/// An error which can result from interacting with the database.
/// This type can "hide" the concrete type used by the database library.
pub type Error = postgres::Error;

/// A fairing which makes sure we can interact with the database correctly.
#[derive(Default)]
pub struct DbInitFairing;

impl DbInitFairing {
    /// Initializes the database, as it should be when starting the server.
    ///
    /// It drops tables which should not be persistent, and makes sure
    /// all tables needed for the server to function are set up correctly.
    fn init_db(client: &mut postgres::Client) -> Result<(), postgres::Error> {
        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS admins (
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
            );
            CREATE TABLE IF NOT EXISTS template_variables (
                name  TEXT PRIMARY KEY,
                value TEXT
            );

            DELETE FROM sessions;
            DELETE FROM room_attempts;
            DELETE FROM room_updates;",
        )
    }
}

#[rocket::async_trait]
impl Fairing for DbInitFairing {
    fn info(&self) -> Info {
        Info {
            name: "Database Init Fairing",
            kind: Kind::Ignite,
        }
    }

    /// Sets up the database so that the server can start working.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let conn = match DbConn::get_one(&rocket).await {
            Some(conn) => conn,
            _ => return Err(rocket),
        };

        match conn.run(DbInitFairing::init_db).await {
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
/// of type `Result<T, db::Error>`, where T is the type returned by the
/// map function. If the query does not return exactly one row, the
/// operation is considered to have failed.
#[macro_export]
macro_rules! query_one_row {
    ($client: expr, $sql:expr, $params:expr, $row_map:expr) => {{
        match $client.query_one($sql, $params) {
            Ok(row) => Ok($row_map(row)),
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
    ($client: expr, $sql:expr, $params:expr, $row_map:expr) => {{
        $client.query($sql, $params)?.into_iter().map($row_map)
    }};
}
