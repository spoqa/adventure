use core::convert::Infallible;
use core::fmt::{self, Display};

#[cfg(feature = "std")]
use std::error::Error as StdError;

/// Errors encountered by the retrial operation.
#[derive(Debug)]
pub struct RetryError<E = Infallible> {
    inner: RetryErrorKind<E>,
}

#[derive(Debug)]
enum RetryErrorKind<E> {
    Aborted(E),
    Timeout,
    #[allow(dead_code)]
    TimerShutdown,
}

impl<E> From<Infallible> for RetryError<E> {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

impl<E: Display> Display for RetryError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RetryErrorKind::*;
        match &self.inner {
            Aborted(e) => e.fmt(f),
            Timeout => "Timeout reached".fmt(f),
            TimerShutdown => "Timer has gone".fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl<E: StdError + 'static> StdError for RetryError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use RetryErrorKind::*;
        match &self.inner {
            Aborted(e) => Some(&*e),
            _ => None,
        }
    }
}

impl<E> RetryError<E> {
    pub fn from_err(e: E) -> Self {
        RetryError {
            inner: RetryErrorKind::Aborted(e),
        }
    }

    pub(crate) const fn timeout() -> Self {
        RetryError {
            inner: RetryErrorKind::Timeout,
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn shutdown() -> Self {
        RetryError {
            inner: RetryErrorKind::TimerShutdown,
        }
    }

    pub fn as_inner(&self) -> Option<&E> {
        if let RetryErrorKind::Aborted(e) = &self.inner {
            Some(e)
        } else {
            None
        }
    }

    pub fn into_inner(self) -> Option<E> {
        if let RetryErrorKind::Aborted(e) = self.inner {
            Some(e)
        } else {
            None
        }
    }

    /// Returns `true` if the error was caused by the retrial has aborted.
    pub fn is_aborted(&self) -> bool {
        self.as_inner().is_some()
    }

    /// Returns `true` if the error was caused by the operation timed out.
    pub fn is_timeout(&self) -> bool {
        if let RetryErrorKind::Timeout = &self.inner {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the error was caused by the timer begin shutdown.
    ///
    /// This is related the internal state of the timer implementation,
    /// meaning the operation will never be able to complete. This is a
    /// permanent error, this is, once this error is observed, retries will
    /// never succeed in the future.
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
            Aborted(_) => unreachable!(),
            Timeout => Timeout,
            TimerShutdown => TimerShutdown,
        };
        RetryError { inner }
    }
}
