//! Module for working with the static resources of the website.
//!
//! This module implements features associated with the static resources
//! exposed by the website, such as HTML documents, scripts, and images.
//! These static resources should not change while the server is running.
//!
//! Since static resources do not change, clients can cache them.
//! This behaviour is implemented by the `StaticFile` custom responder.

use rocket::config::Environment;
use rocket::http::hyper::header::{CacheControl, CacheDirective};
use rocket::response::{self, NamedFile, Responder, Response};
use rocket::Request;

/// A static file which can be served to clients.
pub struct StaticFile(pub NamedFile);

/// Tells clients that they should cache the file received as a response.
///
/// Caching is not activated while developing, to allow for modifications
/// to the front-end code without constantly clearing the cache.
impl<'r> Responder<'r> for StaticFile {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        // Send a normal response if developing.
        if let Ok(Environment::Development) = Environment::active() {
            return self.0.respond_to(req);
        }

        /// The maximum duration a file should be cached for, in seconds.
        const CACHE_MAX_AGE: u32 = 31536000; // A year.

        // Tell the client to cache the file.
        Response::build_from(self.0.respond_to(req)?)
            .header(CacheControl(vec![
                CacheDirective::Public,
                CacheDirective::MaxAge(CACHE_MAX_AGE),
                CacheDirective::Extension("immutable".into(), None),
            ]))
            .ok()
    }
}
