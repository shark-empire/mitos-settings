use std::fs;

#[derive(Debug, Clone)]
pub struct SystemAccount {
    pub username: String,
    pub uid: u32,
    pub shell: String,
}

/// Lists "real" (non-system) accounts by reading `/etc/passwd` — the same
/// source `getent passwd` draws from. Deliberately read-only: creating or
/// removing accounts belongs to a dedicated user-management tool running
/// with real root privileges, not a generic settings `set` call.
pub fn list() -> Vec<SystemAccount> {
    let Ok(content) = fs::read_to_string("/etc/passwd") else { return Vec::new() };
    content
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let username = fields.next()?.to_string();
            let uid: u32 = fields.nth(1)?.parse().ok()?; // skip password field, land on uid
            let shell = fields.last()?.to_string();
            let is_human = (1000..65534).contains(&uid) && !shell.ends_with("nologin") && !shell.ends_with("false");
            is_human.then_some(SystemAccount { username, uid, shell })
        })
        .collect()
}
