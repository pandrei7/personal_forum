//! Module for working with user sessions.
//!
//! Sessions are stored in a database, and are handled mostly server-side.
//! Users receive cookies which identify their session, but do not contain
//! other information themselves.
//!
//! This module contains types which allow you to interact with user sessions,
//! as well as fairings which make database interaction possible.
//!
//! This module also implements the "cleaning" behaviour of old sessions,
//! which removes stale sessions automatically.

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
use rocket_contrib::*;

/// Holds a connection to the sessions database.
///
/// It's a type needed to interact with Rocket.
#[database("sessions")]
pub struct SessionsDbConn(Connection);

/// The path of the database holding session info.
const DB_PATH: &str = "db/sessions.db";

/// The name of the cookie used to hold a session's id.
const SESSION_ID_COOKIE: &str = "session_id";

/// Holds relevant information about a session.
///
/// It's closely tied to a row in the sessions database.
pub struct Session {
    id: String,
    last_update: i64,
    is_admin: bool,
}

impl Session {
    /// Checks if the session belongs to an administrator.
    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    /// Sets the session to belong to an administrator.
    ///
    /// It makes the necessary updates to the database,
    /// and returns true if the operation succeeds.
    pub fn make_admin(&mut self, conn: &Connection) -> bool {
        let sql = format!("UPDATE sessions SET is_admin = 1 WHERE id = '{}';", self.id);
        match conn.execute(&sql, &[]) {
            // The query should update exactly one row.
            Ok(1) => {
                self.is_admin = true;
                true
            }
            _ => false,
        }
    }

    /// Keeps a session "alive" by updating its timestamp.
    fn keep_alive(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        self.last_update = Session::current_timestamp();

        let sql = format!(
            "UPDATE sessions SET last_update = {} WHERE id = '{}';",
            self.last_update, self.id
        );
        conn.execute(&sql, &[]).and(Ok(()))
    }

    /// Tries to retrive the session associated with an id from the database.
    fn from_db(conn: &Connection, id: &str) -> rusqlite::Result<Session> {
        let sql = format!(
            "SELECT last_update, is_admin FROM sessions WHERE id = '{}';",
            id
        );

        conn.query_row(&sql, &[], |row| Session {
            id: String::from(id),
            last_update: row.get(0),
            is_admin: row.get::<usize, i32>(1) == 1,
        })
    }

    /// Tries to start a new session and inserts it into the database.
    fn start_new(conn: &Connection) -> rusqlite::Result<String> {
        let id = Session::new_session_id();
        let last_update = Session::current_timestamp();

        let sql = format!(
            "INSERT INTO sessions (id, last_update, is_admin) VALUES ('{}', {}, {});",
            id, last_update, 0
        );
        conn.execute(&sql, &[]).and(Ok(id))
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
    /// cookie to identify an existing entry in the sessions database.
    fn from_request(req: &'a Request<'r>) -> Outcome<Session, Self::Error> {
        let conn = req.guard::<SessionsDbConn>()?;

        let session_id = match req.cookies().get_private(SESSION_ID_COOKIE) {
            None => return Outcome::Forward(()),
            Some(cookie) => cookie.value().parse::<String>().unwrap(),
        };

        if let Ok(session) = Session::from_db(&conn, &session_id) {
            Outcome::Success(session)
        } else {
            Outcome::Forward(())
        }
    }
}

/// A fairing used to make interaction with the sessions database possible.
#[derive(Default)]
pub struct SessionFairing;

impl SessionFairing {
    /// Makes sure we can interact with the sessions database.
    ///
    /// The database should be empty on each run of the program.
    /// In case the file does not exist, or is empty, the table should
    /// be created. To achieve this easily, we destroy the table on each run,
    /// and create it from scratch.
    fn setup_db() -> rusqlite::Result<()> {
        let conn = Connection::open(DB_PATH)?;

        conn.execute("DROP TABLE IF EXISTS sessions;", &[])?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
            id          TEXT PRIMARY KEY,
            last_update INTEGER NOT NULL,
            is_admin    INTEGER NOT NULL
        );",
            &[],
        )
        .and(Ok(()))
    }

    /// Attempts to start a "cleaner" thread which removes old sessions
    /// from the database.
    ///
    /// The thread cleans the database every `PERIOD` seconds.
    fn start_cleaner() -> rusqlite::Result<()> {
        let conn = Connection::open(DB_PATH)?;

        thread::spawn(move || loop {
            let start = Instant::now();
            if SessionFairing::delete_old(&conn).is_err() {
                eprintln!("Error while cleaning sessions db.");
            }
            let elapsed = start.elapsed();

            const PERIOD: Duration = Duration::from_secs(1800);
            if let Some(remaining) = PERIOD.checked_sub(elapsed) {
                thread::sleep(remaining);
            }
        });

        Ok(())
    }

    /// Deletes "old" sessions from the databse.
    ///
    /// A session is considered old if its last update happened more than
    /// `TIMEOUT_SECS` seconds before the function was called.
    fn delete_old(conn: &Connection) -> rusqlite::Result<()> {
        const TIMEOUT_SECS: i64 = 7200;
        let too_old = Session::current_timestamp() - TIMEOUT_SECS;

        let sql = format!("DELETE FROM sessions WHERE last_update < {};", too_old);
        conn.execute(&sql, &[]).and(Ok(()))
    }
}

/// The fairing is responsible for setting up the sessions database,
/// and making sure all users receive sessions which work correctly.
impl Fairing for SessionFairing {
    fn info(&self) -> Info {
        Info {
            name: "Session Fairing",
            kind: Kind::Attach | Kind::Request,
        }
    }

    /// Makes sure that we can work with the database, and that
    /// old entries are deleted automatically by the cleaner thread.
    /// If any of this fails, the launch is aborted.
    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        if SessionFairing::setup_db().is_err() {
            eprintln!("Could not setup sessions db.");
            return Err(rocket);
        }

        if SessionFairing::start_cleaner().is_err() {
            eprintln!("Could not start cleaner for sessions db.");
            return Err(rocket);
        }

        Ok(rocket)
    }

    /// Makes sure that all users who send us messages are associated with a session.
    ///
    /// If the user is new, the fairing creates a new session
    /// and sets the appropriate cookies. If the user already has a session,
    /// we keep it alive by "refreshing" it.
    fn on_request(&self, req: &mut Request, _: &Data) {
        let conn = match req.guard::<SessionsDbConn>() {
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
