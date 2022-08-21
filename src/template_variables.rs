//! Module for working with variables used in webpage templates.
//!
//! This module provides code for retrieving and interacting with some data
//! used as template variables. The data types from this module should probably
//! implement some useful traits to make interaction with them easy.

use rocket::data::ToByteUnit;

use rocket::data::{Data, FromData};
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{self, FromRequest, Request};
use rocket_sync_db_pools::postgres::row::Row;
use rocket_sync_db_pools::postgres::Client;

use crate::db::{self, DbConn};
use crate::*;

/// The maximum length (in bytes) allowed for a welcome message.
pub const MAX_WELCOME_MESSAGE_LEN: usize = 2048;

/// Represents an HTML string which should be displayed on the main page
/// to greet users and give them some useful information.
pub struct WelcomeMessage(pub String);

impl WelcomeMessage {
    /// Saves the message to the database.
    pub fn save_to_db(&self, client: &mut Client) -> Result<(), db::Error> {
        client
            .execute(
                "INSERT INTO template_variables (name, value) VALUES ('welcome_message', $1)
            ON CONFLICT (name) DO UPDATE SET value = excluded.value;",
                &[&self.0],
            )
            .and(Ok(()))
            .map_err(Into::into)
    }

    /// Retrieves the current welcome message from the database.
    ///
    /// If the welcome message has not been set, an empty message is returned.
    fn from_db(client: &mut Client) -> WelcomeMessage {
        match query_one_row!(
            client,
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WelcomeMessage {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let conn = try_outcome!(req.guard::<DbConn>().await);
        let msg = conn.run(WelcomeMessage::from_db).await;
        request::Outcome::Success(msg)
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for WelcomeMessage {
    type Error = String;

    /// Parses and cleans a welcome message from a body of data.
    ///
    /// The message should be sent as a plaintext string.
    ///
    /// It's important that this message is cleaned, otherwise an attacker
    /// who manages to obtain admin rights might insert malicious code which
    /// all users would receive.
    async fn from_data(_req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        let message = match data
            .open(MAX_WELCOME_MESSAGE_LEN.bytes())
            .into_string()
            .await
        {
            Ok(string) => string.into_inner(),
            Err(err) => {
                return data::Outcome::Failure((Status::InternalServerError, format!("{:?}", err)));
            }
        };

        let message = ammonia::clean(&message);
        data::Outcome::Success(Self(message))
    }
}
