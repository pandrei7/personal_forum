//! Module for working with regular users, including non-administrators.

use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};

use crate::sessions::Session;

/// Holds the data of a regular user.
pub struct User(pub Session);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();

    /// All users have a session associated with them.
    /// We derive the user data by retrieving their session.
    async fn from_request(req: &'r Request<'_>) -> Outcome<User, Self::Error> {
        let session = try_outcome!(req.guard::<Session>().await);
        Outcome::Success(User(session))
    }
}
