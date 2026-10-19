//! Secrets that live only in memory.
//!
//! The LUKS recovery key is generated here, shown once on the Cifrado screen
//! and handed to `systemd-cryptenroll`/`cryptsetup` during the install. It is
//! never written to the log, to `system/lib/defaults.nix` or to anything inside the
//! encrypted disk; [`zeroize`] wipes it as soon as it goes out of scope. The
//! password hash (`mkpasswd`) also lives here because it is another secret.

use color_eyre::eyre::{eyre, Result};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

/// Hashes a password with `mkpasswd -m yescrypt --stdin`.
///
/// The plaintext is only ever written to the child's stdin; it is never logged,
/// put in a display string or written to disk.
pub fn hash_password(password: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("mkpasswd")
        .arg("-m")
        .arg("yescrypt")
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| eyre!("no se pudo ejecutar mkpasswd: {e}"))?;

    // Exact bytes on stdin, no trailing newline: `--stdin` reads the secret
    // verbatim (EOF terminates it), so the hash matches what was typed.
    child
        .stdin
        .take()
        .ok_or_else(|| eyre!("mkpasswd no aceptó la entrada"))?
        .write_all(password.as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(eyre!(
            "mkpasswd falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// systemd's `modhex` alphabet: the 16 letters that are unambiguous when typed
/// from a recovery sheet (no vowels, no look-alikes).
const MODHEX: &[u8; 16] = b"cbdefghijklnrtuv";

/// 8 groups of 4 modhex characters, separated by hyphens (32 chars, 128 bits),
/// the exact shape `systemd-cryptenroll --recovery-key` produces.
pub fn recovery_key() -> Zeroizing<String> {
    let mut rng = OsRng;
    let mut out = String::with_capacity(8 * 4 + 7);
    for group in 0..8 {
        if group > 0 {
            out.push('-');
        }
        for _ in 0..4 {
            let mut byte = [0u8; 1];
            rng.fill_bytes(&mut byte);
            out.push(MODHEX[(byte[0] % 16) as usize] as char);
        }
    }
    Zeroizing::new(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_key_shape() {
        let key = recovery_key();
        assert_eq!(key.len(), 39);
        let groups: Vec<&str> = key.split('-').collect();
        assert_eq!(groups.len(), 8);
        assert!(groups.iter().all(|g| g.len() == 4));
        assert!(key.chars().all(|c| c == '-' || MODHEX.contains(&(c as u8))));
    }

    #[test]
    fn recovery_keys_differ() {
        assert_ne!(*recovery_key(), *recovery_key());
    }
}
