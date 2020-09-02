//! Module for working with messages.
//!
//! Messages are sent to/from users and are held in their own database.
//! This module provides types which allow you to interact with those databases.
//!
//! There are two "types" of messages conceptually: those which start a new
//! thread, and replies to the main thread message.

use std::time::{SystemTime, UNIX_EPOCH};

use rocket_contrib::databases::rusqlite::{self, Connection};
use serde::Serialize;

/// Holds the relevant information of a message.
#[derive(Debug, Serialize)]
pub struct Message {
    id: i32,
    content: String,
    timestamp: i64,
    /// The id of the user author.
    /// It's optional because, as sessions time out, messages can "forget" their author.
    author: Option<String>,
    /// Messages which start new threads have this field set to `None`.
    /// Replies hold the id of the message which started their thread.
    reply_to: Option<i32>,
}

impl Message {
    /// Initializes the database which holds messages.
    pub fn setup_db(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                content   TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                author    TEXT,
                reply_to  INTEGER,
                FOREIGN KEY (reply_to) REFERENCES messages(id)
            );",
            &[],
        )
        .and(Ok(()))
    }

    /// Adds a new message to a given database.
    pub fn add(
        conn: &Connection,
        content: String,
        author: String,
        reply_to: Option<i32>,
    ) -> rusqlite::Result<()> {
        let timestamp = Message::current_timestamp();

        conn.execute(
            "INSERT INTO messages (content, timestamp, author, reply_to) VALUES (?1, ?2, ?3, ?4);",
            &[&content, &timestamp, &author, &reply_to],
        )
        .and(Ok(()))
    }

    /// Returns the current timestamp, as it should be saved in the database.
    ///
    /// Since the server might receive multiple messages quickly, timestamps
    /// should have high enough precision.
    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Error while calculating timestamp")
            .as_millis() as i64
    }
}
