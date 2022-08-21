//! Module for working with administrator accounts.
//!
//! This module provides data types and request guards for
//! authenticating and interacting with administrators.
//!
//! Administrator credentials are held in the `admins` table, which
//! should be populated from outside the program, since the server
//! only reads its contents.

use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket_sync_db_pools::postgres::row::Row;
use rocket_sync_db_pools::postgres::Client;
use sha2::{Digest, Sha256};

use crate::db;
use crate::sessions::Session;
use crate::users::User;
use crate::*;

/// Holds the data of an administrator.
pub struct Admin(pub Session);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Admin {
    type Error = ();

    /// Administrators are `User`s whose `is_admin` field is set to `true`.
    async fn from_request(req: &'r Request<'_>) -> Outcome<Admin, Self::Error> {
        let User(session) = try_outcome!(req.guard::<User>().await);
        if session.is_admin() {
            Outcome::Success(Admin(session))
        } else {
            Outcome::Forward(())
        }
    }
}

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
    pub fn is_valid(&self, client: &mut Client) -> Result<bool, db::Error> {
        let wanted: String = query_one_row!(
            client,
            "SELECT password FROM admins WHERE username = $1;",
            &[&self.username],
            |row: Row| row.get(0)
        )?;

        let actual = format!("{:x}", Sha256::digest(self.password.as_bytes()));

        Ok(actual == wanted)
    }
}
