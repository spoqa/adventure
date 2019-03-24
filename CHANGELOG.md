0.3.0 (not released)
--------------------

### Breaking Changes

 - `retry::ExponentialBackoff` is replaced to a wrapper type which implements
   `Clone`, instead of reexporting from `backoff` crate.

### Bug fixes

 - The result of `RetriableRequest::retry` on will implement `Clone` if the
   source request is implementing it. This makes applying other combinators on
   the request much easier.

0.2.0 (March 24, 2019)
--------------------

### Breaking Changes

 - `Request` is splitted into `BaseRequest` and `OneshotRequest`, and
   `Request::into_response` is also renamed to `OneshotRequest::send_once`.
 - `RepeatableRequest` is renamed to `Request`, and extends `BaseRequest`
   instead `OneshotRequest`. Therefore, both of `Request` and `OneshotRequest`
   will have the same base trait.
 - Object safety of `Request` and `Response` is broken, as a consequence of
   addition of utility methods.
 - `std-futures` feature is renamed to `std-future`.

### New features

 - `OneshotRequest::repeat` is added to transform a oneshot request
   implementing `Clone` to be repeatable.
 - `RetriableRequest` is added to provide retrial behavior with a customizable
   strategy, like exponential backoff.
 - `PagedRequest::paginate` is added.
 - `Response::into_future` is added to convert into futures 0.1 `Future`, or
   `std::future::Future`.
 - Implementation of `BaseRequest`, `Response`, and related traits for pointer
   types are added.

### Bug fixes

 - If both of `futures01` and `std-future` features are enabled, `&Waker` in
   `Response::poll` will be not ignored even if it is polled from futures 0.1
   or goes into them. It will prevent a potential freezing bug.
 - `Response::Waker` associated type is removed.

0.1.0 (March 22, 2019)
----------------------

 - Initial release.
