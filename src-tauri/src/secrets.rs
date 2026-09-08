//! The SMTP password lives in the OS keyring under one fixed key, so changing
//! the username or email never detaches it.

use crate::error::AppResult;

const SERVICE: &str = "dev.quinn.lkml-pin";
const ACCOUNT: &str = "smtp-password";

fn entry() -> AppResult<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, ACCOUNT)?)
}

pub fn set_password(password: &str) -> AppResult<()> {
    entry()?.set_password(password)?;
    Ok(())
}

pub fn get_password() -> AppResult<Option<String>> {
    match entry()?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_password() -> AppResult<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Google shows App Passwords as "abcd efgh ijkl mnop"; the spaces are not
/// part of the secret. Any other password is kept exactly as typed.
pub fn normalize(password: &str) -> String {
    let groups: Vec<&str> = password.trim().split(' ').collect();
    let looks_like_app_password = groups.len() == 4
        && groups
            .iter()
            .all(|g| g.len() == 4 && g.chars().all(|c| c.is_ascii_lowercase()));
    if looks_like_app_password {
        groups.concat()
    } else {
        password.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn only_app_password_shape_is_squashed() {
        assert_eq!(normalize("abcd efgh ijkl mnop"), "abcdefghijklmnop");
        assert_eq!(normalize(" abcd efgh ijkl mnop\n"), "abcdefghijklmnop");
        assert_eq!(normalize("correct horse battery staple"), "correct horse battery staple");
        assert_eq!(normalize("abcdefghijklmnop"), "abcdefghijklmnop");
        assert_eq!(normalize("ab cd"), "ab cd");
    }
}
