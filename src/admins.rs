//! Module for working with administrator accounts.
//!
//! This module provides data structures and request guards for
//! authenticating and interacting with administrators.
//!
//! Administrator credentials are held in the `admins` table, which
//! should be populated from outside the program, since the server
//! only reads its contents.

use rocket::request::{FromRequest, Outcome, Request};
use rocket_contrib::databases::postgres::rows::Row;
use rocket_contrib::databases::postgres::{self, Connection};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::sessions::Session;
use crate::users::User;
use crate::*;

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

/// The content of a form used to log in administrators.
#[derive(Deserialize)]
pub struct AdminLogin {
    username: String,
    password: String,
}

impl AdminLogin {
    /// Checks if the login form references an administrator account.
    ///
    /// Administrators are identified by their username,
    /// and their passwords are held in a database as SHA-256 hashes.
    pub fn is_valid(&self, conn: &Connection) -> postgres::Result<bool> {
        let wanted: String = query_one_row!(
            conn,
            "SELECT password FROM admins WHERE username = $1;",
            &[&self.username],
            |row: Row| row.get(0)
        )?;

        let actual = format!("{:x}", Sha256::digest(self.password.as_bytes()));

        Ok(actual == wanted)
    }
}
