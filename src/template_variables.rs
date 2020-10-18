//! Module for working with variables used in webpage templates.
//!
//! This module provides code for retrieving and interacting with some data
//! used as template variables. The data types from this module should probably
//! implement some useful traits to make interaction with them easy.

use std::io::Read;

use rocket::data::{Data, FromDataSimple};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket_contrib::databases::postgres::rows::Row;
use rocket_contrib::databases::postgres::{self, Connection};

use crate::db::DbConn;
use crate::*;

/// The maximum length (in bytes) allowed for a welcome message.
pub const MAX_WELCOME_MESSAGE_LEN: usize = 2048;

/// Represents an HTML string which should be displayed on the main page
/// to greet users and give them some useful information.
pub struct WelcomeMessage(pub String);

impl WelcomeMessage {
    /// Saves the message to the database.
    pub fn save_to_db(&self, conn: &Connection) -> postgres::Result<()> {
        conn.execute(
            "INSERT INTO template_variables (name, value) VALUES ('welcome_message', $1)
            ON CONFLICT (name) DO UPDATE SET value = excluded.value;",
            &[&self.0],
        )
        .and(Ok(()))
    }

    /// Retrieves the current welcome message from the database.
    ///
    /// If the welcome message has not been set, an empty message is returned.
    fn from_db(conn: &Connection) -> WelcomeMessage {
        match query_one_row!(
            conn,
            "SELECT value FROM template_variables WHERE name = 'welcome_message';",
            &[],
            |row: Row| WelcomeMessage(row.get(0))
        ) {
            Ok(message) => message,
            // The message might not exist yet in the database.
            _ => Self("".into()),
        }
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for WelcomeMessage {
    type Error = ();

    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let conn = req.guard::<DbConn>()?;
        request::Outcome::Success(WelcomeMessage::from_db(&conn))
    }
}

impl FromDataSimple for WelcomeMessage {
    type Error = String;

    /// Parses and cleans a welcome message from a body of data.
    ///
    /// The message should be sent as a plaintext string.
    fn from_data(_req: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        let mut message = String::new();
        if let Err(err) = data
            .open()
            .take(MAX_WELCOME_MESSAGE_LEN as u64)
            .read_to_string(&mut message)
        {
            return data::Outcome::Failure((Status::InternalServerError, format!("{:?}", err)));
        }

        let message = ammonia::clean(&message);
        data::Outcome::Success(Self(message))
    }
}
