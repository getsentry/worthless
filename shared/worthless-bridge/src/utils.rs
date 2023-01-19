use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::{Error, ErrorKind};

pub fn serialize_to_cbor<T: Serialize>(value: &T, ty_name: &'static str) -> Result<Vec<u8>, Error> {
    let mut rv = Vec::<u8>::new();
    ciborium::ser::into_writer(value, &mut rv).map_err(|err| {
        Error::new(
            ErrorKind::SerializationError,
            format!("failed to serialize {}", ty_name),
        )
        .with_source(err)
    })?;
    Ok(rv)
}

pub fn deserialize_from_cbor<T>(bytes: &[u8], ty_name: &'static str) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    ciborium::de::from_reader(bytes).map_err(|err| {
        Error::new(
            ErrorKind::SerializationError,
            format!("failed to deserialize {}", ty_name),
        )
        .with_source(err)
    })
}
