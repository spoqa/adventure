0.2.0 (not released)
--------------------

### Breaking Changes

 - `Request` is splitted into `BaseRequest` and `OneshotRequest`, and
   `Request::into_response` is also renamed to `OneshotRequest::send_once`.
 - `RepeatableRequest` is renamed to `Request`, and extends `BaseRequest`
   instead `OneshotRequest`. Therefore, both of `Request` and `OneshotRequest`
   will have the same base trait.

### New features

 - `RequestExt` trait is added in the prelude.
 - `RequestExt::repeat` is added to transform a request implementing `Clone`
   into a repeatable request.
 - `RequestExt::with_backoff` is added to provide retry behavior with
   exponential backoff.
 - Any type of `Response` can be converted into futures 0.1 `Future`, or
   `std::future::Future`.
 - Implementation of `Request`, `Response`, and related traits for pointer
   types are added.

### Bug fixes

 - If both of `futures01` and `std-future` features are enabled, `&Waker` in
   `Response::poll` will be not ignored even if it is polled from futures 0.1
   or goes into them. It will prevent a potential freezing bug.
 - `Response::Waker` associated type is removed.
 - `std-futures` feature is renamed to `std-future`.

0.1.0 (March 22, 2019)
----------------------

 - Initial release.
