use hyper::StatusCode;
use hyper::header::ContentType;
use responder::Responder;
use response::Response;
use std::num::{ParseIntError, ParseFloatError};
use std::string::ParseError as ParseStringError;
use std::str::ParseBoolError;
use hyper::Error as HyperError;

#[derive(Debug)]
pub enum Error {
    RouterError,
    ParamParseError(String),
    ParamNotFound(&'static str),
    HyperError(HyperError),
    FromDataError(String),
    StateNotFound(String),
    OtherUsersFault(String),
    OtherServersFault(String),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match *self {
            Error::RouterError => StatusCode::NotFound,
            Error::ParamParseError(_) => StatusCode::BadRequest,
            Error::ParamNotFound(_) => StatusCode::BadRequest,
            Error::HyperError(_) => StatusCode::BadRequest,
            Error::FromDataError(_) => StatusCode::UnprocessableEntity,
            Error::StateNotFound(_) => StatusCode::InternalServerError,
            Error::OtherUsersFault(_) => StatusCode::BadRequest,
            Error::OtherServersFault(_) => StatusCode::InternalServerError,
        }
    }
}

impl Responder for Error {
    fn respond(self) -> Response {
        Response::new()
            .with_status(self.status_code())
            .with_header(ContentType::json())
            .with_body(format!("{{'status': 'error', 'msg': '{:?}' }}", self))
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Error {
        Error::ParamParseError(e.to_string())
    }
}

impl From<ParseFloatError> for Error {
    fn from(e: ParseFloatError) -> Error {
        Error::ParamParseError(e.to_string())
    }
}

impl From<ParseBoolError> for Error {
    fn from(e: ParseBoolError) -> Error {
        Error::ParamParseError(e.to_string())
    }
}


impl From<ParseStringError> for Error {
    fn from(_: ParseStringError) -> Error {
        Error::ParamParseError("error parsing string from string, really?".to_string())
    }
}

impl From<HyperError> for Error {
    fn from(e: HyperError) -> Error {
        Error::HyperError(e)
    }
}
