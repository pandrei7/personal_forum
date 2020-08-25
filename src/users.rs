//! Module for working with regular users, including non-administrators.

use rocket::request::{FromRequest, Outcome, Request};

use crate::sessions::Session;

/// Holds the data of a regular user.
pub struct User(pub Session);

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    /// All users have a session associated with them.
    /// We derive the user data by retrieving their session.
    fn from_request(req: &'a Request<'r>) -> Outcome<User, Self::Error> {
        if let Outcome::Success(session) = req.guard::<Session>() {
            Outcome::Success(User(session))
        } else {
            Outcome::Forward(())
        }
    }
}
