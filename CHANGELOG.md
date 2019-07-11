0.4.0 (not released)
--------------------

### Breaking Changes

 - 1.36 or higher version of Rust compiler is required.
 - `std-future` feature is removed, because `core::future` always will be
   available.
 - Definition of `Response` has changed. Now it is the same as
   `futures_core::TryFuture`. Moreover, `task` and some modules are removed
   that was related to compatibiliy with futures 0.1.
 - `Request::send` receives `Pin<&mut Self>` instead of `&self`.
 - Implementation of futures 0.1 Stream for Paginator is removed.
 - required version of `futures-preview` is increased to `0.3.0-alpha.17`.
 - required version of Rusoto is increased to `0.40`.

### New features

 - Rewritten to be depends on `core::future` and futures 0.3 by default.
 - Compatible with `#[no_std]`.
 - Paginator will implement FusedStream, mainly to be used easily with
   `select!` macro of futures 0.3.
 - Companion packages for Rusoto, including `adventure-rusoto-ecs`, will offer
   the features like `native-tls` or `rustls`.

0.3.0 (March 24, 2019)
----------------------

### Breaking Changes

 - `retry::ExponentialBackoff` is replaced to a wrapper type which implements
   `Clone`, instead of reexporting from `backoff` crate.
 - A type parameter of `PagedRequest` is removed, to propagate this property
   through a `OneshotRequest<C>` combinator.
 - Adapter types for futures 0.1 like `ResponseFuture` are renamed like
   `Future01Response`.
 - Adapter types for `std::future` like `ResponseStdFuture` are renamed like
   `FutureResponse`.

### Bug fixes

 - The result of `RetriableRequest::retry` will implement `Clone` and
   `Request<C>` if the source request is implementing `Clone`. This makes
   applying other combinators on the request much easier.
 - Forward implementations of request traits like `PagedRequest` are added for
   request combinators, if their original request implements the same trait.

0.2.0 (March 24, 2019)
----------------------

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
