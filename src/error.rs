use std::result;

#[derive(Debug)]
pub enum Error {
    EndOfData,
}

pub type Result<T> = result::Result<T, Error>;