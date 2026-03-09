use std::io;
use std::process::{Child, Output};

/// A guard that ensures a child process is killed and reaped if dropped before being consumed.
///
/// On the happy path, call `ChildGuard::wait_with_output` to consume the guard and wait for the
/// child normally. If the guard is dropped without being consumed (e.g. due to an early `?`
/// return), the child is killed and waited on so it is never orphaned.
pub(crate) struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    pub(crate) const fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    /// Consume the guard and wait for the child to finish, returning its output.
    pub(crate) fn wait_with_output(mut self) -> io::Result<Output> {
        self.child
            .take()
            .expect("ChildGuard should always contain a child")
            .wait_with_output()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            drop(child.kill());
            drop(child.wait());
        }
    }
}
