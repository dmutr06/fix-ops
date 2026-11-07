//! Staff sign-in, sessions and account management (REQ-F-001, REQ-F-002, REQ-F-019).

use fixops_domain::staff::{Password, StaffAccount, Username};

use crate::{App, AppResult};

impl<S> App<S> {
    /// Creates the first Administrator, only when no staff account exists yet.
    pub async fn bootstrap_admin(
        &self,
        username: Username,
        password: Password,
    ) -> AppResult<Option<StaffAccount>> {
        let _ = (username, password);
        todo!()
    }
}
