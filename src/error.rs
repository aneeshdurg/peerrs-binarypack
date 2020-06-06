use std::string::FromUtf8Error;
use std::result;

#[derive(Debug)]
pub enum Error {
    EndOfData,
    StringParseError,
}

impl From<FromUtf8Error> for Error {
    fn from(_error: FromUtf8Error) -> Self {
        Error::StringParseError
    }
}

pub type Result<T> = result::Result<T, Error>;
