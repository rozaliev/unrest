use serde_json::{self, Value};
use serde::de::DeserializeOwned;
use serde::Serialize;
use {FromData, Error, Responder, Response};
use hyper::header::ContentType;

#[derive(Debug)]
pub struct Json<T = Value>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: DeserializeOwned> FromData for Json<T> {
    fn from_data(buf: &[u8]) -> Result<Self, Error> {
        let inner = serde_json::from_slice(buf).map_err(|e| {
            Error::FromDataError(e.to_string())
        })?;

        Ok(Json(inner))
    }
}

impl<T: Serialize> Responder for Json<T> {
    fn respond(self) -> Response {
        match serde_json::to_vec(&self.0).map_err(|e| Error::OtherServersFault(e.to_string())) {
            Ok(json) => {
                Response::new().with_header(ContentType::json()).with_body(
                    json,
                )
            }
            Err(e) => e.respond(),
        }


    }
}