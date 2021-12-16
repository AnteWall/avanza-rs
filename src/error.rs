use std::error;
use std::fmt;

extern crate reqwest;

extern crate serde;
extern crate serde_json;

#[derive(Debug, Clone)]
pub struct UnknownAuthenticationMethod;

impl fmt::Display for UnknownAuthenticationMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "can not handle authentication method")
    }
}
impl error::Error for UnknownAuthenticationMethod {}

#[derive(Debug, Clone)]
pub struct NotAuthenticatedError;

impl fmt::Display for NotAuthenticatedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "not authorized")
    }
}
impl error::Error for NotAuthenticatedError {}

#[derive(Debug)]
pub enum RequestError {
    WebRequestError(reqwest::Error),
    ParseError(serde_json::Error),
    NotAuthenticatedError(),
    UnknownAuthenticationMethod(),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RequestError")
    }
}

impl error::Error for RequestError {
    fn description(&self) -> &str {
        "API internal error"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl From<serde_json::Error> for RequestError {
    fn from(e: serde_json::Error) -> Self {
        RequestError::ParseError(e)
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(e: reqwest::Error) -> Self {
        RequestError::WebRequestError(e)
    }
}
