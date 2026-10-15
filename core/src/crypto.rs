//! Secrets that live only in memory.
//!
//! The LUKS recovery key is generated here, shown once on the Cifrado screen
//! and handed to `systemd-cryptenroll`/`cryptsetup` during the install. It is
//! never written to the log, to `settings.nix` or to anything inside the
//! encrypted disk; [`zeroize`] wipes it as soon as it goes out of scope.

use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

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
