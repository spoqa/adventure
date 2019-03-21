use crate::response::Response;

/// Generalized request/response/error triple
pub trait Request<C> {
    type Ok;
    type Error;
    type Response: Response<Ok = Self::Ok, Error = Self::Error>;

    fn send(&self, client: C) -> Self::Response;
}

pub trait PagedRequest<C>: Request<C> {
    fn advance(&mut self, response: &Self::Ok) -> bool;
}
