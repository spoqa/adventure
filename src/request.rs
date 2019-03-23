//! A base trait represents a request.

use crate::response::Response;

/// A generalized request-response interface, regardless how client works.
pub trait Request<C> {
    /// The type of successful values from the corresponding response.
    type Ok;
    /// The type of failures from the corresponding response.
    type Error;
    /// The type of corresponding responses of this request.
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    /// Send this request using the given client.
    fn into_response(self, client: C) -> Self::Response;
}

impl<R, C> Request<C> for Box<R>
where
    R: Request<C>,
{
    type Ok = R::Ok;
    type Error = R::Error;
    type Response = R::Response;
    fn into_response(self, client: C) -> Self::Response {
        let inner = *self;
        inner.into_response(client)
    }
}
