//! Module for working with messages.
//!
//! Messages are sent to/from users and are held in their own database.
//! This module provides types which allow you to interact with those databases.
//!
//! There are two "types" of messages conceptually: those which start a new
//! thread, and replies to the main thread message.

use std::time::{SystemTime, UNIX_EPOCH};

use pulldown_cmark::html;
use pulldown_cmark::{Options, Parser};
use rocket_contrib::databases::rusqlite::{self, Connection};
use serde::{Deserialize, Serialize};

/// Sanitizes a user's message and prepares it for being stored.
///
/// To prevent attacks like HTML-injection, we should sanitize messages before
/// sending them to other users. We also want to support CommonMark in messages,
/// which should be converted to normal HTML.
///
/// To avoid doing this operation each time we need to send updates to a user,
/// we first convert the message to the correct form, then store it like that.
pub fn prepare_for_storage(message: &mut String) {
    let mut cmark_options = Options::empty();
    cmark_options.insert(Options::ENABLE_TABLES);

    let mut unsafe_html = String::new();
    html::push_html(&mut unsafe_html, Parser::new_ext(message, cmark_options));

    let safe_html = ammonia::clean(&unsafe_html);
    *message = safe_html;
}

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

    /// Returns all messages inserted into the database since a given timestamp.
    ///
    /// The timestamp should have the format used by the database.
    pub fn get_since(conn: &Connection, since: i64) -> rusqlite::Result<Vec<Self>> {
        conn.prepare("SELECT * FROM messages WHERE timestamp >= ?;")?
            .query_map(&[&since], |row| Message {
                id: row.get(0),
                content: row.get(1),
                timestamp: row.get(2),
                author: row.get(3),
                reply_to: row.get(4),
            })?
            .collect()
    }

    /// Adds a new message to a given database.
    pub fn add(
        conn: &Connection,
        content: String,
        author: String,
        reply_to: Option<i32>,
    ) -> rusqlite::Result<()> {
        let timestamp = Message::current_timestamp();

        conn.execute("PRAGMA foreign_keys=ON;", &[])?;

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

/// The content of the JSON form through which users send messages.
#[derive(Deserialize)]
pub struct MessageJson {
    pub content: String,
    pub reply_to: Option<i32>,
}
