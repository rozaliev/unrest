use {Response, Error};
use futures::Future;
use futures::future::Then;

use hyper::header::ContentType;

pub trait Responder {
    fn respond(self) -> Response;
}

pub trait LiftError {
    type Future;

    fn lift_error(self) -> Self::Future;
}

impl<F, T, E> LiftError for F
where
    F: Future<Item = T, Error = E> + 'static,
    T: Responder + 'static,
    E: Responder + 'static,
    Self: Sized + 'static,
{
    type Future = Then<Self, Result<Response, Error>, fn(Result<T, E>) -> Result<Response, Error>>;

    fn lift_error(self) -> Self::Future {
        fn le<T, E>(res: Result<T, E>) -> Result<Response, Error>
        where
            T: Responder + 'static,
            E: Responder + 'static,
        {
            Ok(res.respond())
        }

        self.then(le)
    }
}



impl Responder for String {
    fn respond(self) -> Response {
        Response::new()
            .with_header(ContentType::plaintext())
            .with_body(self)
    }
}

impl Responder for &'static str {
    fn respond(self) -> Response {
        Response::new()
            .with_header(ContentType::plaintext())
            .with_body(self)
    }
}

impl Responder for () {
    fn respond(self) -> Response {
        Response::new()
    }
}

impl<T, E> Responder for Result<T, E>
where
    T: Responder,
    E: Responder,
{
    fn respond(self) -> Response {
        match self {
            Ok(ok) => ok.respond(),
            Err(err) => err.respond(),
        }
    }
}

impl Responder for Response {
    fn respond(self) -> Response {
        self
    }
}