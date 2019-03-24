use std::error::Error as StdError;
use std::fmt::{self, Display};

pub enum Unreachable {}

#[derive(Debug)]
pub struct RetryError<E = Unreachable> {
    inner: RetryErrorKind<E>,
}

#[derive(Debug)]
enum RetryErrorKind<E> {
    Inner(E),
    Timeout,
    TimerShutdown,
}

impl<E: Display> Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RetryErrorKind::*;
        match &self.inner {
            Inner(e) => e.fmt(f),
            Timeout => "Timeout reached".fmt(f),
            TimerShutdown => "Timer has gone".fmt(f),
        }
    }
}

impl<E: StdError + 'static> StdError for RetryError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use RetryErrorKind::*;
        match &self.inner {
            Inner(e) => Some(&*e),
            _ => None,
        }
    }
}

impl<E> RetryError<E> {
    pub fn from_err(e: E) -> Self {
        RetryError {
            inner: RetryErrorKind::Inner(e),
        }
    }

    pub(crate) const fn timeout() -> Self {
        RetryError {
            inner: RetryErrorKind::Timeout,
        }
    }

    pub(crate) const fn shutdown() -> Self {
        RetryError {
            inner: RetryErrorKind::TimerShutdown,
        }
    }

    pub fn as_inner(&self) -> Option<&E> {
        if let RetryErrorKind::Inner(e) = &self.inner {
            Some(e)
        } else {
            None
        }
    }

    pub fn into_inner(self) -> Option<E> {
        if let RetryErrorKind::Inner(e) = self.inner {
            Some(e)
        } else {
            None
        }
    }

    pub fn is_timeout(&self) -> bool {
        if let RetryErrorKind::Timeout = &self.inner {
            true
        } else {
            false
        }
    }

    pub fn is_shutdown(&self) -> bool {
        if let RetryErrorKind::TimerShutdown = &self.inner {
            true
        } else {
            false
        }
    }
}

impl RetryError {
    pub(crate) fn transform<E>(self) -> RetryError<E> {
        use RetryErrorKind::*;
        let inner = match self.inner {
            Inner(_) => unreachable!(),
            Timeout => Timeout,
            TimerShutdown => TimerShutdown,
        };
        RetryError { inner }
    }
}
