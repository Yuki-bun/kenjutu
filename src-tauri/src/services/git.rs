use git2::Commit;

use crate::models::ChangeId;

pub struct GitService;

impl GitService {
    pub fn get_change_id(commit: &Commit<'_>) -> Option<ChangeId> {
        commit
            .header_field_bytes("change-id")
            .ok()
            .and_then(|buf| buf.as_str().map(String::from).map(ChangeId::from))
    }
}
