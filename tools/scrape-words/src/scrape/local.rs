use futures::{future, prelude::*};
use std::io::{self, prelude::*};
use {Error, Result};

pub fn read<S>(mut ins: S) -> Result<impl Future<Item = Vec<u8>, Error = Error>>
where
  S: Read,
{
  Ok(future::lazy(move || {
    let mut vec = Vec::new();

    io::copy(&mut ins, &mut vec)
      .into_future()
      .from_err()
      .map(|_| vec)
  }))
}
