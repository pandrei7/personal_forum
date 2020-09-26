//! Module for working with user sessions.
//!
//! Sessions are handled mostly server-side. Users receive cookies which
//! identify their session, but do not contain other information themselves.
//!
//! This module also implements the "cleaning" behaviour of old sessions,
//! which removes stale sessions automatically.
//!
//! This module contributes to the incremental-updates mechanism, which allows
//! us to send only those updates which users do not already have. To achieve
//! this, we store the last time a user received updates for each room they
//! visit.

use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::distributions::Alphanumeric;
use rand::prelude::*;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Cookie;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Data, Rocket};
use rocket_contrib::databases::rusqlite;
use rocket_contrib::databases::rusqlite::Connection;

use crate::db::DbConn;

/// The name of the cookie used to hold a session's id.
const SESSION_ID_COOKIE: &str = "session_id";

/// Holds relevant information about a session.
///
/// It's closely tied to a row in the sessions table.
pub struct Session {
    id: String,
    last_update: i64,
    is_admin: bool,
}

impl Session {
    /// Counts the number of sessions in the database.
    pub fn count_sessions(conn: &Connection) -> rusqlite::Result<u32> {
        conn.query_row("SELECT COUNT(*) FROM sessions;", &[], |row| row.get(0))
    }

    /// Checks if the session belongs to an administrator.
    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    /// Returns the session id.
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Sets the session to belong to an administrator.
    ///
    /// It makes the necessary updates to the database,
    /// and returns true if the operation succeeds.
    pub fn make_admin(&mut self, conn: &Connection) -> bool {
        match conn.execute(
            "UPDATE sessions SET is_admin = 1 WHERE id = ?;",
            &[&self.id],
        ) {
            // The query should update exactly one row.
            Ok(1) => {
                self.is_admin = true;
                true
            }
            _ => false,
        }
    }

    /// Saves a room-login attempt for the user with the associated session.
    pub fn save_room_attempt(
        &self,
        conn: &Connection,
        name: &str,
        hashed_password: &str,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO room_attempts (id, name, password) VALUES (?1, ?2, ?3);",
            &[&self.id, &name, &hashed_password],
        )
        .and(Ok(()))
    }

    /// Retrieves the last password associated with a login attempt for a given room, if it exists.
    pub fn get_room_attempt(&self, conn: &Connection, name: &str) -> rusqlite::Result<String> {
        conn.query_row(
            "SELECT password FROM room_attempts WHERE id = ?1 AND name = ?2;",
            &[&self.id, &name],
            |row| row.get::<usize, String>(0),
        )
    }

    /// Sets the given timestamp as the user's last-update time for the given room.
    pub fn save_room_update(
        &self,
        conn: &Connection,
        name: &str,
        timestamp: i64,
    ) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO room_updates (id, name, timestamp) VALUES (?1, ?2, ?3);",
            &[&self.id, &name, &timestamp],
        )
        .and(Ok(()))
    }

    /// Retrieves the timestamp of the last time a user got updates for a given room.
    pub fn get_room_update(&self, conn: &Connection, name: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT timestamp FROM room_updates WHERE id = ?1 AND name = ?2;",
            &[&self.id, &name],
            |row| row.get(0),
        )
    }

    /// Keeps a session "alive" by updating its timestamp.
    fn keep_alive(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        self.last_update = Session::current_timestamp();

        conn.execute(
            "UPDATE sessions SET last_update = ?1 WHERE id = ?2;",
            &[&self.last_update, &self.id],
        )
        .and(Ok(()))
    }

    /// Tries to retrive the session associated with an id from the database.
    fn from_db(conn: &Connection, id: &str) -> rusqlite::Result<Session> {
        conn.query_row(
            "SELECT last_update, is_admin FROM sessions WHERE id = ?;",
            &[&id],
            |row| Session {
                id: String::from(id),
                last_update: row.get(0),
                is_admin: row.get::<usize, i32>(1) == 1,
            },
        )
    }

    /// Tries to start a new session and inserts it into the database.
    fn start_new(conn: &Connection) -> rusqlite::Result<String> {
        let id = Session::new_session_id();
        let last_update = Session::current_timestamp();

        conn.execute(
            "INSERT INTO sessions (id, last_update, is_admin) VALUES (?1, ?2, ?3);",
            &[&id, &last_update, &0],
        )
        .and(Ok(id))
    }

    /// Returns a (probably) new, valid session id.
    fn new_session_id() -> String {
        const ID_LEN: usize = 64;
        rand::thread_rng()
            .sample_iter(Alphanumeric)
            .take(ID_LEN)
            .collect()
    }

    /// Returns the current timestamp, as it should be saved in the database.
    ///
    /// Timestamps represent Unix time points.
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Error while calculating timestamp")
            .as_secs() as i64
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for Session {
    type Error = ();

    /// A `Session` is retrieved from a request by using the `SESSION_ID_COOKIE`
    /// cookie to identify an existing entry in the sessions table.
    fn from_request(req: &'a Request<'r>) -> Outcome<Session, Self::Error> {
        let session_id = match req.cookies().get_private(SESSION_ID_COOKIE) {
            None => return Outcome::Forward(()),
            Some(cookie) => cookie.value().parse::<String>().unwrap(),
        };

        let conn = req.guard::<DbConn>()?;
        if let Ok(session) = Session::from_db(&conn, &session_id) {
            Outcome::Success(session)
        } else {
            Outcome::Forward(())
        }
    }
}

/// A fairing used to make interaction with sessions possible.
#[derive(Default)]
pub struct SessionFairing;

impl SessionFairing {
    /// Attempts to start a "cleaner" thread which removes old sessions
    /// from the database.
    ///
    /// The thread cleans the database every `PERIOD` seconds.
    fn start_cleaner(conn: DbConn) {
        thread::spawn(move || loop {
            let start = Instant::now();
            if SessionFairing::delete_old(&conn).is_err() {
                eprintln!("Error while cleaning old sessions.");
            }
            let elapsed = start.elapsed();

            const PERIOD: Duration = Duration::from_secs(1800);
            if let Some(remaining) = PERIOD.checked_sub(elapsed) {
                thread::sleep(remaining);
            }
        });
    }

    /// Deletes "old" sessions from the database.
    ///
    /// A session is considered old if its last update happened more than
    /// `TIMEOUT_SECS` seconds before the function was called.
    fn delete_old(conn: &Connection) -> rusqlite::Result<()> {
        const TIMEOUT_SECS: i64 = 7200;
        let too_old = Session::current_timestamp() - TIMEOUT_SECS;

        conn.execute("DELETE FROM sessions WHERE last_update < ?;", &[&too_old])
            .and(Ok(()))
    }
}

/// The fairing is reponsible for assigning sessions to new users, and keeping
/// existing sessions alive. It also removes stale sessions from the database.
impl Fairing for SessionFairing {
    fn info(&self) -> Info {
        Info {
            name: "Session Fairing",
            kind: Kind::Attach | Kind::Request,
        }
    }

    /// Makes sure stale sessions are removed automatically by a cleaner thread.
    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        if let Some(conn) = DbConn::get_one(&rocket) {
            SessionFairing::start_cleaner(conn);
            Ok(rocket)
        } else {
            Err(rocket)
        }
    }

    /// Makes sure that all users who send us requests are assigned sessions.
    ///
    /// If the user is new, the fairing creates a new session and sets the
    /// appropriate cookies. If the user already has a session, we keep it alive.
    fn on_request(&self, req: &mut Request, _: &Data) {
        let conn = match req.guard::<DbConn>() {
            Outcome::Success(conn) => conn,
            _ => {
                eprintln!("Could not connect to session database.");
                return;
            }
        };

        // Try to retrieve the existing session.
        if let Outcome::Success(mut session) = req.guard::<Session>() {
            if session.keep_alive(&conn).is_err() {
                eprintln!("Could not keep session alive.");
            }
            return;
        }

        // Give the user a new session.
        if let Ok(id) = Session::start_new(&conn) {
            let cookie = Cookie::build("session_id", id).http_only(true).finish();
            req.cookies().add_private(cookie);
        } else {
            eprintln!("Could not start a new session.");
        }
    }
}
