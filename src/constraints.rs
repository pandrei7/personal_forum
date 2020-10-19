//! Module for working with constraints which should be enforced.
//!
//! This module provides data types and constants for checking certain
//! conditions which should be met by the data we interact with. For example,
//! the byte-length of a message posted by a user should not exceed a chosen
//! limit.
//!
//! The types provided by this module implement certain traits which should
//! make them easy to use, especially as parameter guards and data guards.

use std::fmt::{self, Display, Formatter};
use std::io::Read;

use rocket::data::{Data, FromDataSimple, Outcome};
use rocket::http::{RawStr, Status};
use rocket::request::{FromParam, Request};
use serde::Serialize;

/// The maximum length (in bytes) allowed for a messsage.
pub const MAX_MESSAGE_LEN: usize = 2048;
/// The maximum length (in bytes) allowed for a room name.
pub const MAX_ROOM_NAME_LEN: usize = 128;

/// Represents a valid name for a room.
#[derive(Serialize)]
pub struct RoomName(pub String);

impl RoomName {
    /// Checks if a given name is valid, and builds the corresponding `RoomName`.
    /// If the name is invalid, a reason is returned as a human-readable string.
    ///
    /// Valid room names are not allowed to be empty. They also should not
    /// be too long. The only characters permitted are alphanumeric ASCII
    /// characters and a few "special" ones, such as: '_' and '-'.
    pub fn parse(name: &str) -> Result<Self, String> {
        if name.is_empty() {
            return Err("The room name cannot be empty.".into());
        }
        if name.len() > MAX_ROOM_NAME_LEN {
            return Err("The room name is too long.".into());
        }

        let valid = |ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
        if name.chars().all(valid) {
            Ok(Self(name.to_string()))
        } else {
            Err("The room name contains invalid characters.".into())
        }
    }
}

impl<'r> FromParam<'r> for RoomName {
    type Error = &'r RawStr;

    fn from_param(param: &'r RawStr) -> Result<Self, Self::Error> {
        RoomName::parse(param).map_err(|_| param)
    }
}

impl FromDataSimple for RoomName {
    type Error = String;

    fn from_data(_req: &Request, data: Data) -> Outcome<Self, Self::Error> {
        let mut name = String::new();
        if let Err(err) = data
            .open()
            .take(MAX_ROOM_NAME_LEN as u64)
            .read_to_string(&mut name)
        {
            return Outcome::Failure((Status::InternalServerError, format!("{:?}", err)));
        }

        match RoomName::parse(&name) {
            Ok(room_name) => Outcome::Success(room_name),
            Err(reason) => Outcome::Failure((Status::UnprocessableEntity, reason)),
        }
    }
}

impl Display for RoomName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
