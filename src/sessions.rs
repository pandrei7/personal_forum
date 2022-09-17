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

use std::time::{SystemTime, UNIX_EPOCH};

use rand::distributions::Alphanumeric;
use rand::prelude::*;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Cookie;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::tokio::time::{sleep, Duration};
use rocket::{Data, Rocket};
use rocket_sync_db_pools::postgres::row::Row;
use rocket_sync_db_pools::postgres::Client;

use crate::db::{self, DbConn};
use crate::*;

/// The name of the cookie used to hold a session's id.
const SESSION_ID_COOKIE: &str = "session_id";

/// The custom HTTP status indicating that a user's session has expired.
const SESSION_EXPIRED: Status = Status::new(491);

/// Holds relevant information about a session.
///
/// It's closely tied to a row in the sessions table.
#[derive(Clone)]
pub struct Session {
    id: String,
    last_update: i64,
    is_admin: bool,
}

impl Session {
    /// Counts the number of sessions in the database.
    pub fn count_sessions(client: &mut Client) -> Result<i64, db::Error> {
        query_one_row!(client, "SELECT COUNT(*) FROM sessions;", &[], |row: Row| {
            row.get(0)
        })
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
    pub fn make_admin(&mut self, client: &mut Client) -> bool {
        match client.execute(
            "UPDATE sessions SET is_admin = TRUE WHERE id = $1;",
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
        client: &mut Client,
        name: &str,
        hashed_password: &str,
    ) -> Result<(), db::Error> {
        client
            .execute(
                "INSERT INTO room_attempts (id, name, password) VALUES ($1, $2, $3)
            ON CONFLICT (id, name) DO UPDATE SET password = excluded.password;",
                &[&self.id, &name, &hashed_password],
            )
            .and(Ok(()))
            .map_err(Into::into)
    }

    /// Retrieves the last password associated with a login attempt for a given room, if it exists.
    pub fn get_room_attempt(&self, client: &mut Client, name: &str) -> Result<String, db::Error> {
        query_one_row!(
            client,
            "SELECT password FROM room_attempts WHERE id = $1 AND name = $2;",
            &[&self.id, &name],
            |row: Row| row.get(0)
        )
    }

    /// Sets the given timestamp as the user's last-update time for the given room.
    pub fn save_room_update(
        &self,
        client: &mut Client,
        name: &str,
        timestamp: i64,
    ) -> Result<(), db::Error> {
        client
            .execute(
                "INSERT INTO room_updates (id, name, timestamp) VALUES ($1, $2, $3)
            ON CONFLICT (id, name) DO UPDATE SET timestamp = excluded.timestamp;",
                &[&self.id, &name, &timestamp],
            )
            .and(Ok(()))
            .map_err(Into::into)
    }

    /// Retrieves the timestamp of the last time a user got updates for a given room.
    pub fn get_room_update(&self, client: &mut Client, name: &str) -> Result<i64, db::Error> {
        query_one_row!(
            client,
            "SELECT timestamp FROM room_updates WHERE id = $1 AND name = $2;",
            &[&self.id, &name],
            |row: Row| row.get(0)
        )
    }

    /// Keeps a session "alive" by updating its timestamp.
    fn keep_alive(&mut self, client: &mut Client) -> Result<(), db::Error> {
        self.last_update = Session::current_timestamp();

        client
            .execute(
                "UPDATE sessions SET last_update = $1 WHERE id = $2;",
                &[&self.last_update, &self.id],
            )
            .and(Ok(()))
            .map_err(Into::into)
    }

    /// Tries to retrive the session associated with an id from the database.
    fn from_db(client: &mut Client, id: &str) -> Result<Session, db::Error> {
        query_one_row!(
            client,
            "SELECT last_update, is_admin FROM sessions WHERE id = $1;",
            &[&id],
            |row: Row| Session {
                id: String::from(id),
                last_update: row.get(0),
                is_admin: row.get(1),
            }
        )
    }

    /// Tries to start a new session and inserts it into the database.
    fn start_new(client: &mut Client) -> Result<String, db::Error> {
        let id = Session::new_session_id();
        let last_update = Session::current_timestamp();

        client
            .execute(
                "INSERT INTO sessions (id, last_update, is_admin) VALUES ($1, $2, $3);",
                &[&id, &last_update, &false],
            )
            .and(Ok(id))
            .map_err(Into::into)
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
            .expect("Error while calculating the session timestamp.")
            .as_secs() as i64
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = ();

    /// A `Session` is retrieved from a request by using the `SESSION_ID_COOKIE`
    /// cookie to identify an existing entry in the sessions table.
    async fn from_request(req: &'r Request<'_>) -> Outcome<Session, Self::Error> {
        // Try to retrieve the user's existing session, if it exists.
        let session_id = match req.cookies().get_private(SESSION_ID_COOKIE) {
            Some(cookie) => cookie.value().parse::<String>().unwrap(),
            None => return Outcome::Forward(()),
        };

        // If the user's session id is not in the database, it expired.
        let conn = try_outcome!(req.guard::<DbConn>().await);
        match conn.run(move |c| Session::from_db(c, &session_id)).await {
            Ok(session) => return Outcome::Success(session),
            _ => return Outcome::Failure((SESSION_EXPIRED, ())),
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
        rocket::tokio::task::spawn(async move {
            loop {
                if conn.run(SessionFairing::delete_old).await.is_err() {
                    eprintln!("Error while cleaning old sessions.");
                }

                const PERIOD: Duration = Duration::from_secs(300);
                sleep(PERIOD).await;
            }
        });
    }

    /// Deletes "old" sessions from the database.
    ///
    /// A session is considered old if its last update happened more than
    /// `TIMEOUT_SECS` seconds before the function was called.
    fn delete_old(client: &mut Client) -> Result<(), db::Error> {
        const TIMEOUT_SECS: i64 = 1200;
        let too_old = Session::current_timestamp() - TIMEOUT_SECS;

        client
            .execute("DELETE FROM sessions WHERE last_update < $1;", &[&too_old])
            .and(Ok(()))
            .map_err(Into::into)
    }
}

/// The fairing is reponsible for assigning sessions to new users, and keeping
/// existing sessions alive. It also removes stale sessions from the database.
#[rocket::async_trait]
impl Fairing for SessionFairing {
    fn info(&self) -> Info {
        Info {
            name: "Session Fairing",
            kind: Kind::Ignite | Kind::Request,
        }
    }

    /// Makes sure stale sessions are removed automatically by a cleaner thread.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        if let Some(conn) = DbConn::get_one(&rocket).await {
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
    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        let conn = match req.guard::<DbConn>().await {
            Outcome::Success(conn) => conn,
            _ => {
                eprintln!("Could not connect to the session database.");
                return;
            }
        };

        // Try to keep alive the existing session.
        // Do not start a new session if an error occurs.
        match req.guard::<Session>().await {
            Outcome::Success(mut session) => {
                if conn.run(move |c| session.keep_alive(c)).await.is_err() {
                    eprintln!("Could not keep the session alive.");
                }
                return;
            }
            Outcome::Failure(_) => return,
            _ => {}
        };

        // Give the user a new session.
        if let Ok(id) = conn.run(Session::start_new).await {
            let cookie = Cookie::build(SESSION_ID_COOKIE, id)
                .http_only(true)
                .finish();
            req.cookies().add_private(cookie);
        } else {
            eprintln!("Could not start a new session.");
        }
    }
}

/// A catcher for SESSION_EXPIRED messages which removes a user's old session id cookie.
#[catch(491)]
pub async fn session_expired(req: &Request<'_>) -> Flash<Redirect> {
    req.cookies()
        .remove_private(Cookie::named(sessions::SESSION_ID_COOKIE));

    Flash::error(
        Redirect::to("/"),
        "It is possible that your session expired. Try again.",
    )
}
