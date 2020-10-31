//! Module for working with messages.
//!
//! Messages of a room are sent to/from users and are held in their own table.
//! This module provides types which allow you to interact with such a table.
//!
//! There are two "types" of messages conceptually: those which start a new
//! thread, and replies to the main thread message.

use std::time::{SystemTime, UNIX_EPOCH};

use pulldown_cmark::html;
use pulldown_cmark::{Options, Parser};
use rocket_contrib::databases::postgres::rows::Row;
use rocket_contrib::databases::postgres::{self, Connection};
use serde::{Deserialize, Serialize};

use crate::*;

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
    /// It should be skipped when sending messages to clients, because it might
    /// allow attackers to impersonate other users, even admins.
    #[serde(skip_serializing)]
    author: Option<String>,
    /// Messages which start new threads have this field set to `None`.
    /// Replies hold the id of the message which started their thread.
    reply_to: Option<i32>,
}

impl Message {
    /// Initializes the table which holds messages.
    pub fn setup_table(conn: &Connection, table: &str) -> postgres::Result<()> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id        SERIAL PRIMARY KEY,
                content   TEXT NOT NULL,
                timestamp BIGINT NOT NULL,
                author    TEXT,
                reply_to  INT,
                FOREIGN KEY (author) REFERENCES sessions(id) ON DELETE SET NULL,
                FOREIGN KEY (reply_to) REFERENCES {table}(id)
            );",
            table = table
        );
        conn.execute(&sql, &[]).and(Ok(()))
    }

    /// Returns all messages inserted into the table in the given interval.
    ///
    /// The left endpoint is exclusive, and the right one is inclusive -
    /// i.e., (old, new].
    ///
    /// The timestamps should have the format used by the table.
    pub fn get_between(
        conn: &Connection,
        table: &str,
        old: i64,
        new: i64,
    ) -> postgres::Result<Vec<Self>> {
        Ok(query_and_map!(
            conn,
            &format!(
                "SELECT * FROM {} WHERE $1 < timestamp AND timestamp <= $2;",
                table
            ),
            &[&old, &new],
            |row: Row| Message {
                id: row.get(0),
                content: row.get(1),
                timestamp: row.get(2),
                author: row.get(3),
                reply_to: row.get(4),
            }
        )
        .collect())
    }

    /// Adds a new message to a given table.
    pub fn add(
        conn: &Connection,
        table: &str,
        content: String,
        author: String,
        reply_to: Option<i32>,
    ) -> postgres::Result<()> {
        let timestamp = Message::current_timestamp();

        conn.execute(
            &format!(
                "INSERT INTO {} (content, timestamp, author, reply_to) VALUES ($1, $2, $3, $4);",
                table
            ),
            &[&content, &timestamp, &author, &reply_to],
        )
        .and(Ok(()))
    }

    /// Returns the current timestamp, as it should be saved in the table.
    ///
    /// Since the server might receive multiple messages quickly, timestamps
    /// should have high enough precision.
    pub fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Error while calculating the message timestamp")
            .as_millis() as i64
    }
}

/// The content of the JSON form through which users send messages.
#[derive(Deserialize)]
pub struct MessageJson {
    pub content: String,
    pub reply_to: Option<i32>,
}

/// The content of the response sent to users upon an update request.
#[derive(Serialize)]
pub struct Updates {
    pub clean_stored: bool,
    pub messages: Vec<Message>,
}
