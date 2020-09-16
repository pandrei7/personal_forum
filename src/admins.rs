//! Module for working with administrator accounts.
//!
//! Administrator credentials are held in a database.
//! This database should be populated from outside the program,
//! which only accesses it.
//!
//! This module also contains types used to authenticate users as
//! administrators, which is done through a login form.

use rocket::request::{FromRequest, Outcome, Request};
use rocket::*;
use rocket_contrib::databases::rusqlite::{self, Connection};
use rocket_contrib::*;
use sha2::{Digest, Sha256};

use crate::sessions::Session;
use crate::users::User;

/// Holds the data of an administrator.
pub struct Admin(pub Session);

impl<'a, 'r> FromRequest<'a, 'r> for Admin {
    type Error = ();

    /// Administrators are `User`s whose `is_admin` field is set to `true`.
    fn from_request(req: &'a Request<'r>) -> Outcome<Admin, Self::Error> {
        let User(session) = req.guard::<User>()?;
        if session.is_admin() {
            Outcome::Success(Admin(session))
        } else {
            Outcome::Forward(())
        }
    }
}

/// Holds a connection to the admin database.
///
/// It's a type needed to interact with Rocket.
#[database("admins")]
pub struct AdminsDbConn(Connection);

/// The content of a form used to log in administrators.
#[derive(FromForm)]
pub struct AdminLogin {
    username: String,
    password: String,
}

impl AdminLogin {
    /// Checks if the login form references an administrator account.
    ///
    /// Administrators are identified by their username,
    /// and their passwords are held in a database as SHA-256 hashes.
    pub fn is_valid(&self, conn: &Connection) -> rusqlite::Result<bool> {
        let wanted = conn.query_row(
            "SELECT password FROM admins WHERE username = ?;",
            &[&self.username],
            |row| row.get::<usize, String>(0),
        )?;

        let actual = format!("{:x}", Sha256::digest(self.password.as_bytes()));

        Ok(actual == wanted)
    }
}
