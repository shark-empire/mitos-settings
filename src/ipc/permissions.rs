//! Daemon-side authorization: who is actually on the other end of a
//! connection, and what the daemon itself is allowed to do regardless of
//! who's asking.
//!
//! This used to rely purely on the Unix socket's file permissions (mode
//! 0660 — see `ipc::server::bind`) as the *entire* trust boundary, which
//! meant anyone who could reach the socket at all could apply any change,
//! since the daemon otherwise had no way to distinguish between the
//! different users who might be connected to it. `peer_credentials` closes
//! that gap using `SO_PEERCRED`, a Linux kernel facility that reports the
//! real, unspoofable uid/gid/pid of whoever is on the other end of a Unix
//! socket. See docs/security.md for the full picture.

use crate::permissions::{current_context, PrivilegeLevel};
use crate::settings::schema::SettingSpec;
use std::io;
use std::os::raw::{c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;

/// Even with a correctly-identified peer, the daemon itself has to be
/// capable of the action: a `Root`-level setting can't be honored unless
/// the daemon process is actually running as uid 0, no matter who's
/// asking or how privileged they are.
pub fn ensure_daemon_may_apply(spec: &SettingSpec) -> Result<(), String> {
    if spec.privilege == PrivilegeLevel::Root && current_context().uid != 0 {
        return Err(format!(
            "'{}' requires the daemon to run as root; start it via systemd or `sudo mitos-settings --daemon`",
            spec.key
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct PeerCredentials {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

// The kernel's `struct ucred` (see `man 7 unix`). `#[repr(C)]` so its
// layout matches the C struct exactly for the getsockopt() call below.
#[repr(C)]
struct UCred {
    pid: i32,
    uid: u32,
    gid: u32,
}

const SOL_SOCKET: c_int = 1;
// SO_PEERCRED's numeric value is architecture-dependent in the Linux
// kernel headers. 17 is correct for every architecture this project
// actually targets (it explicitly aims at real desktop/laptop hardware --
// x86, x86_64, arm, aarch64); porting to e.g. MIPS, SPARC, or Alpha would
// need a different constant here. This is exactly the kind of per-target
// table the `libc` crate maintains -- hardcoding one value is the
// deliberate tradeoff for staying dependency-free.
const SO_PEERCRED: c_int = 17;

extern "C" {
    fn getsockopt(sockfd: c_int, level: c_int, optname: c_int, optval: *mut c_void, optlen: *mut u32) -> c_int;
}

/// The kernel-verified identity of whoever is on the other end of `stream`.
/// This is what actually backs per-connection authorization now -- not
/// just "did they reach the socket at all".
pub fn peer_credentials(stream: &UnixStream) -> io::Result<PeerCredentials> {
    let mut cred = UCred { pid: 0, uid: 0, gid: 0 };
    let mut len = std::mem::size_of::<UCred>() as u32;

    // SAFETY: `getsockopt` is given a valid, open socket fd (borrowed from
    // `stream`, which outlives this call), a pointer to a `UCred` that's
    // exactly `len` bytes and matches the kernel's `struct ucred` layout,
    // and a pointer to that same `len`. The kernel writes at most `len`
    // bytes into `cred` and updates `len` to how much it actually wrote --
    // it never reads `cred` before writing it, so leaving it
    // zero-initialized above is safe either way.
    let ret = unsafe {
        getsockopt(stream.as_raw_fd(), SOL_SOCKET, SO_PEERCRED, &mut cred as *mut UCred as *mut c_void, &mut len)
    };

    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(PeerCredentials { pid: cred.pid, uid: cred.uid, gid: cred.gid })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions;
    use crate::settings::value::{Value, ValueKind};

    #[test]
    fn admin_level_settings_are_never_blocked_by_capability_check() {
        let spec = SettingSpec::new(
            "sharing.file_sharing_enabled",
            "sharing",
            "File sharing",
            "desc",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        );
        assert!(ensure_daemon_may_apply(&spec).is_ok());
    }

    #[test]
    fn peer_credentials_of_a_self_connected_pair_matches_our_own_uid() {
        // UnixStream::pair() connects two ends of a socket within this
        // same process, so the kernel reports *our own* uid as the peer on
        // both ends -- a fully self-contained way to test the FFI call
        // actually works, no root or second user required.
        let (a, _b) = UnixStream::pair().expect("create a connected socket pair");
        let cred = peer_credentials(&a).expect("SO_PEERCRED should succeed on a live local socket");
        assert_eq!(cred.uid, permissions::current_context().uid);
    }
}
