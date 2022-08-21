//! Module for working with the static resources of the website.
//!
//! This module implements features associated with the static resources
//! exposed by the website, such as HTML documents, scripts, and images.
//! These static resources should not change while the server is running.
//!
//! Since static resources do not change, clients can cache them.
//! This behaviour is implemented by the `StaticFile` custom responder.

use rocket::config::Config;
use rocket::fs::NamedFile;
use rocket::http::hyper::header::CACHE_CONTROL;
use rocket::http::Header;
use rocket::response::{self, Responder, Response};
use rocket::Request;

/// A static file which can be served to clients.
pub struct StaticFile(pub NamedFile);

/// Tells clients that they should cache the file received as a response.
///
/// Caching is not activated while developing, to allow for modifications
/// to the front-end code without constantly clearing the cache.
impl<'r> Responder<'r, 'static> for StaticFile {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        // Send a normal response if developing.
        if Config::DEBUG_PROFILE == *Config::figment().profile() {
            return self.0.respond_to(req);
        }

        /// The maximum duration a file should be cached for, in seconds.
        const CACHE_MAX_AGE: u32 = 31_536_000; // A year.

        // Tell the client to cache the file. The header value was copied from
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#immutable.
        Response::build_from(self.0.respond_to(req)?)
            .header(Header::new(
                CACHE_CONTROL.as_str(),
                format!("public, max-age={}, immutable", CACHE_MAX_AGE),
            ))
            .ok()
    }
}
