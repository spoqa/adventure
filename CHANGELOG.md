0.2.0 (not released)
--------------------

### Breaking changes

 - `RetryError::and_then` is removed.

### New features

 - Any type of `Response` can be converted into futures 0.1 `Future`, or
   `std::future::Future`.
 - Add implementations of `Response` for pointer types.
 - Add `ResponseExt::with_backoff`, which provides retry behavior with
   exponential backoff.

### Bug fixes

 - If both of `futures01` and `std-future` features are enabled, `&Waker` in
   `Response::poll` will be not ignored even if it is polled from futures 0.1
   or goes into them. It will prevent a potential freezing bug.
 - `Response::Waker` associated type is removed.
 - `std-futures` feature is renamed to `std-future`.

0.1.0 (March 22, 2019)
----------------------

 - Initial release.
