use std::error::Error as StdError;
use std::fmt::{self, Display};

#[derive(Debug)]
pub struct RetryError<E> {
    inner: RetryErrorKind<E>,
}

#[derive(Debug)]
enum RetryErrorKind<E> {
    Inner(E),
    Backoff(BackoffError),
}

impl<E: Display> Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RetryErrorKind::*;
        match &self.inner {
            Inner(e) => e.fmt(f),
            Backoff(e) => e.fmt(f),
        }
    }
}

impl<E: StdError + 'static> StdError for RetryError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use RetryErrorKind::*;
        match &self.inner {
            Inner(e) => Some(&*e),
            Backoff(e) => Some(&*e),
        }
    }
}

impl<E> RetryError<E> {
    pub fn into_inner(self) -> Option<E> {
        if let RetryErrorKind::Inner(e) = self.inner {
            Some(e)
        } else {
            None
        }
    }

    pub fn is_timeout(&self) -> bool {
        if let RetryErrorKind::Backoff(e) = &self.inner {
            e.is_timeout()
        } else {
            false
        }
    }

    pub fn is_shutdown(&self) -> bool {
        if let RetryErrorKind::Backoff(e) = &self.inner {
            e.is_shutdown()
        } else {
            false
        }
    }
}

impl<E> From<BackoffError> for RetryError<E> {
    fn from(e: BackoffError) -> Self {
        RetryError {
            inner: RetryErrorKind::Backoff(e),
        }
    }
}

#[derive(Debug)]
pub struct BackoffError {
    inner: BackoffErrorKind,
}

#[derive(Debug)]
enum BackoffErrorKind {
    Timeout,
    TimerShutdown,
}

impl Display for BackoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BackoffErrorKind::*;
        match self.inner {
            Timeout => "Timeout reached".fmt(f),
            TimerShutdown => "Timer has gone".fmt(f),
        }
    }
}

impl StdError for BackoffError {}

impl BackoffError {
    pub(crate) const fn timeout() -> Self {
        BackoffError {
            inner: BackoffErrorKind::Timeout,
        }
    }
    pub(crate) const fn shutdown() -> Self {
        BackoffError {
            inner: BackoffErrorKind::TimerShutdown,
        }
    }

    pub fn is_timeout(&self) -> bool {
        if let BackoffErrorKind::Timeout = self.inner {
            true
        } else {
            false
        }
    }

    pub fn is_shutdown(&self) -> bool {
        if let BackoffErrorKind::TimerShutdown = self.inner {
            true
        } else {
            false
        }
    }
}
