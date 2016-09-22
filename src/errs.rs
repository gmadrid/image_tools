use std::io;
use std::result;

#[derive(Debug)]
pub enum Error {
  BitmapConversionFailed,
  CannotFinalizeImageDestination,
  CreateFailed(String),
  FailedToLoadAsJPEG,
  FailedToLoadAsPNG,
  Io(io::Error),
}

pub type Result<T> = result::Result<T, Error>;

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Error {
    Error::Io(err)
  }
}
