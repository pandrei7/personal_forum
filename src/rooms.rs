//! Module for working with rooms.
//!
//! Rooms are password protected, and contain threads of messages. Each
//! room has a unique name which cannot be changed after its creation.
//! Passwords should be changeable to allow for easier management.
//!
//! Information about rooms such as their name and password is held
//! in the `rooms` table. Apart from this "central" one, each room
//! keeps its messages in a separate table, which is created/deleted as needed.
//!
//! The rooms module contributes to the incremental-updates mechanism, which
//! should help decrease network traffic by avoiding the resending of the
//! entire message-table content repeatedly. To achieve this, the `Room`
//! struct allows retrieving updates only for given time intervals.

use ::serde::Deserialize;
use rocket::outcome::try_outcome;
use rocket::request::{self, FromRequest, Request};
use rocket_sync_db_pools::postgres::error::SqlState;
use rocket_sync_db_pools::postgres::row::Row;
use rocket_sync_db_pools::postgres::Client;
use sha2::{Digest, Sha256};

use crate::db::{self, DbConn};
use crate::messages::{self, Message, Updates};
use crate::sessions::Session;
use crate::*;

/// Returns the hash of a password, as it should be stored in the database.
///
/// Passwords should be stored as SHA-256 hashes.
pub fn hash_password(password: &str) -> String {
    format!("{:x}", Sha256::digest(password.as_bytes()))
}

/// Holds relevant information about a room.
///
/// It's tied to a row in the rooms table.
pub struct Room {
    /// The hashed password used to log into the room.
    password: String,
    /// A number used to identify the table which holds the room's messages.
    table_id: i32,
    creation: i64,
}

impl Room {
    /// Creates and initializes a room with the given data.
    ///
    /// Each room has a table for its messages. To ensure that these tables
    /// receive unique names, each room has an associated `table_id`, which
    /// becomes part of the name. The naming scheme is: `messages{table_id}`.
    pub fn create_room(
        client: &mut Client,
        name: String,
        hashed_password: String,
    ) -> Result<(), db::Error> {
        let creation = Message::current_timestamp();
        client.execute(
            "INSERT INTO rooms (name, password, creation) VALUES ($1, $2, $3);",
            &[&name, &hashed_password, &creation],
        )?;

        let table_id: i32 = query_one_row!(
            client,
            "SELECT table_id FROM rooms WHERE name = $1;",
            &[&name],
            |row: Row| row.get(0)
        )?;

        let table = format!("messages{}", table_id);
        Message::setup_table(client, &table).and(Ok(()))
    }

    /// Deletes a room from the database, also removing its message table.
    ///
    /// If the operation fails, the reason is returned as a readable string.
    pub fn delete_room(client: &mut Client, name: &str) -> Result<(), String> {
        let table_id: i32 = query_one_row!(
            client,
            "SELECT table_id FROM rooms WHERE name = $1;",
            &[&name],
            |row: Row| row.get(0)
        )
        .map_err(|_| "Error while retrieving table_id.")?;

        let table = format!("messages{}", table_id);
        match client.execute("DELETE FROM rooms WHERE name = $1;", &[&name]) {
            Ok(1) => client
                .execute(&format!("DROP TABLE IF EXISTS {};", table), &[])
                .map(|_| ())
                .map_err(|_| "Error while deleting the messages table.".into()),
            _ => Err("Error while deleting room metadata.".into()),
        }
    }

    /// Returns a list with the names of all the rooms stored in the database.
    pub fn active_rooms(client: &mut Client) -> Result<Vec<String>, db::Error> {
        Ok(
            query_and_map!(client, "SELECT name FROM rooms;", &[], |row: Row| row
                .get(0))
            .collect(),
        )
    }

    /// Changes the password of the given room.
    pub fn change_password(
        client: &mut Client,
        name: &str,
        hashed_password: &str,
    ) -> Result<(), db::Error> {
        client
            .execute(
                "UPDATE rooms SET password = $1 WHERE name = $2;",
                &[&hashed_password, &name],
            )
            .and(Ok(()))
            .map_err(Into::into)
    }

    /// Returns the next incremental updates a user should receive when requested.
    ///
    /// The timestamps should be given in the format used by the messages database.
    pub fn get_updates_between(
        &self,
        client: &mut Client,
        last_update: i64,
        now: i64,
    ) -> Result<Updates, db::Error> {
        // If this room is a recreation, the client might have messages from
        // the old room in their caches, so they should remove those first.
        let clean_stored = last_update <= self.creation;

        let table = format!("messages{}", self.table_id);
        let messages = Message::get_between(client, &table, last_update, now)?;

        Ok(Updates {
            clean_stored,
            messages,
        })
    }

    /// Adds a new message to the room.
    pub fn add_message(
        &self,
        client: &mut Client,
        mut content: String,
        author: String,
        reply_to: Option<i32>,
    ) -> Result<(), db::Error> {
        messages::prepare_for_storage(&mut content);

        let table = format!("messages{}", self.table_id);
        Message::add(client, &table, content, author, reply_to)
    }

    /// Tries to retrieve the database entry associated with a room, given its name.
    fn from_db(client: &mut Client, name: &str) -> Result<Room, db::Error> {
        query_one_row!(
            client,
            "SELECT password, table_id, creation FROM rooms WHERE name = $1;",
            &[&name],
            |row: Row| Room {
                password: row.get(0),
                table_id: row.get(1),
                creation: row.get(2),
            }
        )
    }

    /// Checks if the given password allows access to the room.
    fn valid_password(&self, hashed_password: &str) -> bool {
        self.password == hashed_password
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Room {
    type Error = ();

    /// Since rooms are password-protected, we must make sure a user is
    /// allowed access to a room before it can interact with it. This request
    /// guard should be the main way of checking users' access to a room.
    ///
    /// To test if the permission check failed because the user did not provide
    /// a valid password, you can use an `Option<Room>` field in the function
    /// header, like this:
    ///
    /// ```rust ignore
    /// #[get("/room/<name>")]
    /// fn test_room_access(name: RoomName, room: Option<Room>) -> &'static str {
    ///     if room.is_some() {
    ///         "You have access to this room."
    ///     } else {
    ///         "You do not have acces to this room."
    ///     }
    /// }
    /// ```
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // Try to extract the name of the room.
        let name = {
            // Room requests should have URLs that start with `/room/<name>`.
            let mut segs = req.uri().path().segments();
            if segs.next() != Some("room") {
                return request::Outcome::Forward(Status::BadRequest);
            }
            match segs.next() {
                Some(name) => name.to_owned(),
                _ => return request::Outcome::Forward(Status::BadRequest),
            }
        };

        let conn = try_outcome!(req.guard::<DbConn>().await);

        // Retrieve the room entry.
        let room = {
            let name = name.clone();
            match conn.run(move |c| Room::from_db(c, &name)).await {
                Ok(room) => room,
                _ => return request::Outcome::Forward(Status::NotFound),
            }
        };

        // Find the user's password attempt.
        let hashed_password = {
            let name = name.clone();
            let session = try_outcome!(req.guard::<Session>().await);
            match conn.run(move |c| session.get_room_attempt(c, &name)).await {
                Ok(password) => password,
                Err(e) if e.code() == Some(&SqlState::NO_DATA) => {
                    return request::Outcome::Forward(Status::Unauthorized)
                }
                _ => return request::Outcome::Forward(Status::InternalServerError),
            }
        };

        if hashed_password == room.password {
            request::Outcome::Success(room)
        } else {
            request::Outcome::Forward(Status::Unauthorized)
        }
    }
}

/// The content of a form used to hold login credentials for a room.
#[derive(Clone, Deserialize, FromForm)]
pub struct RoomLogin {
    pub name: String,
    /// The plaintext password of the room.
    pub password: String,
}

impl RoomLogin {
    /// Checks if the form contains the correct credentials to log into a room.
    pub fn can_log_in(&self, client: &mut Client) -> Result<bool, db::Error> {
        let hashed_password = hash_password(&self.password);
        let room = Room::from_db(client, &self.name)?;
        Ok(room.valid_password(&hashed_password))
    }
}
