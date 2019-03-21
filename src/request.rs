//! A various trait of requests by its capabilities.
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
    fn send(&self, client: C) -> Self::Response;
}

/// A request able to send subsequent requests to enumerate the entire result.
pub trait PagedRequest<C>: Request<C> {
    /// Modify itself to retrive the next response, of return `false` if the
    /// given response was the last one.
    fn advance(&mut self, response: &Self::Ok) -> bool;
}
