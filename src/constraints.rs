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

use rocket::data::{Data, FromData, Outcome, ToByteUnit};
use rocket::http::Status;
use rocket::request::{self, FromParam, Request};
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
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        RoomName::parse(param).map_err(|_| param)
    }
}

#[rocket::async_trait]
impl<'r> FromData<'r> for RoomName {
    type Error = String;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let name = match data.open(MAX_ROOM_NAME_LEN.bytes()).into_string().await {
            Ok(string) => string.into_inner(),
            Err(err) => return Outcome::Error((Status::InternalServerError, format!("{:?}", err))),
        };

        // Suggested replacement for `FromDataSimple`.
        // See https://github.com/SergioBenitez/Rocket/blob/v0.5-rc/CHANGELOG.md#data-and-forms
        // and https://api.rocket.rs/v0.5-rc/rocket/data/trait.FromData.html.
        let name = request::local_cache!(req, name);

        match RoomName::parse(name) {
            Ok(room_name) => Outcome::Success(room_name),
            Err(reason) => Outcome::Error((Status::UnprocessableEntity, reason)),
        }
    }
}

impl Display for RoomName {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
