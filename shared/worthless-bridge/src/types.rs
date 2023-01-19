use std::collections::BTreeMap;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::utils::{deserialize_from_cbor, serialize_to_cbor};

/// The type for arbitrary values.
pub type Value = ciborium::value::Value;

/// Represents the meta part of the protocol.
pub type Meta = BTreeMap<String, Value>;

/// Represents the request to an endpoint on the bridge.
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Request {
    /// The unique ID of the request to allow multiple simultanious
    /// requests through the bridge at once.
    id: Uuid,
    /// key/value pairs of meta information.
    meta: BTreeMap<String, Value>,
    /// When flipped tells the remote side that no response is requested.
    fire_and_forget: bool,
    /// The name of the endpoint to invoke.
    endpoint: String,
    /// The request payload.
    payload: Value,
}

/// Helps to create request objects.
pub struct RequestBuilder {
    request: Option<Request>,
}

/// Represents the response from the bridge.
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Response {
    /// If of the request this response belongs to.
    request_id: Uuid,
    /// Meta information not contained in the payload.
    meta: Meta,
    /// The response payload.
    payload: Result<Value, Error>,
}

/// Helps to create request objects.
pub struct ResponseBuilder {
    response: Option<Response>,
}

/// Represents a protocol error.
///
/// This error is also produced for non protocol situations where the failure
/// exists on one side of the bridge (eg: invalid serialization).
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Error {
    /// The general kind of error
    kind: ErrorKind,
    /// A human readable description of the error.
    description: String,
    /// Optional detail information about the error.
    detail: Option<Value>,
    /// A source error.
    #[serde(skip)]
    source: Option<Box<dyn std::error::Error>>,
}

/// Indicates the kind of an error.
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[repr(u32)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The request went to an unknown endpoint.
    UnknownEndpoint = 404,

    /// An internal error
    InternalError = 500,

    /// Unable to serialize a request or response.
    SerializationError = 999,

    /// Any other error code.
    Other(u32),
}

impl Request {
    /// Creates a basic request to an endpoint
    pub fn new<S, V>(endpoint: S, payload: V) -> Request
    where
        S: Into<String>,
        V: Into<Value>,
    {
        Request {
            id: Uuid::new_v4(),
            meta: BTreeMap::new(),
            fire_and_forget: false,
            endpoint: endpoint.into(),
            payload: payload.into(),
        }
    }

    /// Creates a builder to construct more complex requests.
    pub fn build(endpoint: String) -> RequestBuilder {
        RequestBuilder {
            request: Some(Request::new(endpoint, Value::Null)),
        }
    }

    /// Serializes a request into the wire format.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        serialize_to_cbor(self, "request")
    }

    /// Deserializes the request from the wire format.
    pub fn deserialize(bytes: &[u8]) -> Result<Request, Error> {
        deserialize_from_cbor(bytes, "request")
    }

    /// Returns the ID of the request.
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Returns the meta object of the request.
    pub fn meta(&self) -> &BTreeMap<String, Value> {
        &self.meta
    }

    /// Returns the endpoint targeted by the request.
    pub fn endpoint(&self) -> &str {
        self.endpoint.as_ref()
    }

    /// Borrows the payload of the request.
    pub fn payload(&self) -> &Value {
        &self.payload
    }

    /// Deserializes the payload into a specific structure.
    pub fn deserialize_payload<D: DeserializeOwned>(&self) -> Result<D, Error> {
        self.payload.deserialized().map_err(|err| {
            Error::new(
                ErrorKind::SerializationError,
                format!("failed to match payload against schema"),
            )
            .with_source(err)
        })
    }

    /// Returns `true` if this is a fire and forget request.
    pub fn fire_and_forget(&self) -> bool {
        self.fire_and_forget
    }
}

impl RequestBuilder {
    fn request_mut(&mut self) -> &mut Request {
        self.request.as_mut().expect("builder is already done")
    }

    /// Overrides the default request ID.
    pub fn request_id(&mut self, id: Uuid) -> &mut RequestBuilder {
        self.request_mut().id = id;
        self
    }

    /// Sets the payload of the request.
    pub fn raw_payload<V: Into<Value>>(&mut self, value: V) -> &mut RequestBuilder {
        self.request_mut().payload = value.into();
        self
    }

    /// Sets serialized payload into the request.
    pub fn payload<V: Serialize>(&mut self, value: &V) -> Result<&mut RequestBuilder, Error> {
        self.raw_payload(Value::serialized(value).map_err(|err| {
            Error::new(
                ErrorKind::SerializationError,
                format!("failed to convert payload"),
            )
            .with_source(err)
        })?);
        Ok(self)
    }

    /// Can be used to mark the request as fire and forget.
    pub fn fire_and_forget(&mut self, yes: bool) -> &mut RequestBuilder {
        self.request_mut().fire_and_forget = yes;
        self
    }

    /// Inserts a key/value pair into the meta dictionary.
    pub fn meta<K, V>(&mut self, key: K, value: V) -> &mut RequestBuilder
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.request_mut().meta.insert(key.into(), value.into());
        self
    }

    /// Creates a request out of the builder.
    ///
    /// The builder at this point is no longer usable and will panic if it's
    /// used for further operations.
    pub fn build(&mut self) -> Request {
        self.request.take().expect("can only build request once")
    }
}

impl Response {
    /// Creates a new response.
    pub fn new(
        request_id: Uuid,
        meta: BTreeMap<String, Value>,
        payload: Result<Value, Error>,
    ) -> Response {
        Response {
            request_id,
            meta,
            payload,
        }
    }

    /// Create a response builder for more complex responses.
    pub fn builder(request_id: Uuid) -> ResponseBuilder {
        ResponseBuilder {
            response: Some(Response::new(request_id, BTreeMap::new(), Ok(Value::Null))),
        }
    }

    /// Returns the ID of the request.
    pub fn request_id(&self) -> &Uuid {
        &self.request_id
    }

    /// Returns the meta dictionary.
    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    /// Consumes the response and returns the payload.  If the
    /// response carries an error it's returned here.
    pub fn into_payload(self) -> Result<Value, Error> {
        self.payload
    }

    /// Peeks at the raw payload.
    pub fn payload_ref(&self) -> Option<&Value> {
        self.payload.as_ref().ok()
    }

    /// Deserializes the payload into a specific structure.
    ///
    /// This consumes the request because it will report the payload error.
    pub fn deserialize_payload<D: DeserializeOwned>(self) -> Result<D, Error> {
        self.payload?.deserialized().map_err(|err| {
            Error::new(
                ErrorKind::SerializationError,
                format!("failed to match payload against schema"),
            )
            .with_source(err)
        })
    }

    /// Peeks at the error.
    pub fn error_ref(&self) -> Option<&Error> {
        self.payload.as_ref().err()
    }

    /// Serializes a response into the wire format.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        serialize_to_cbor(self, "response")
    }

    /// Deserializes the response from the wire format.
    pub fn deserialize(bytes: &[u8]) -> Result<Response, Error> {
        deserialize_from_cbor(bytes, "response")
    }
}

impl ResponseBuilder {
    fn response_mut(&mut self) -> &mut Response {
        self.response.as_mut().expect("builder is already done")
    }

    /// Sets the payload of the response.
    pub fn raw_payload<V: Into<Value>>(&mut self, value: V) -> &mut ResponseBuilder {
        self.response_mut().payload = Ok(value.into());
        self
    }

    /// Sets serialized payload into the response.
    pub fn payload<V: Serialize>(&mut self, value: &V) -> Result<&mut ResponseBuilder, Error> {
        self.raw_payload(Value::serialized(value).map_err(|err| {
            Error::new(
                ErrorKind::SerializationError,
                format!("failed to convert payload"),
            )
            .with_source(err)
        })?);
        Ok(self)
    }

    /// Sets an error response.
    pub fn error(&mut self, value: Error) -> &mut ResponseBuilder {
        self.response_mut().payload = Err(value);
        self
    }

    /// Inserts a key/value pair into the meta dictionary.
    pub fn meta<K, V>(&mut self, key: K, value: V) -> &mut ResponseBuilder
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.response_mut().meta.insert(key.into(), value.into());
        self
    }

    /// Creates a response out of the builder.
    ///
    /// The builder at this point is no longer usable and will panic if it's
    /// used for further operations.
    pub fn build(&mut self) -> Response {
        self.response.take().expect("can only build response once")
    }
}

impl Error {
    /// Creates a new error.
    pub fn new<S: Into<String>>(kind: ErrorKind, description: S) -> Error {
        Error {
            kind,
            description: description.into(),
            detail: None,
            source: None,
        }
    }

    /// Modifies the error to attach a detail.
    pub fn with_detail<V: Into<Value>>(mut self, detail: V) -> Error {
        self.detail = Some(detail.into());
        self
    }

    /// Modifies the error to attach another error as source.
    pub fn with_source<E: std::error::Error + 'static>(mut self, source: E) -> Error {
        self.source = Some(Box::new(source));
        self
    }

    /// Returns the error kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the description.
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.description)?;
        if let Some(Value::Text(ref detail)) = self.detail {
            write!(f, "\n{}", detail)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

serde_plain::derive_display_from_serialize!(ErrorKind);
