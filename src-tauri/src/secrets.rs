use crate::error::AppResult;

const SERVICE: &str = "dev.quinn.lkml-pin";

fn entry(user: &str) -> AppResult<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, user)?)
}

pub fn set_password(user: &str, password: &str) -> AppResult<()> {
    entry(user)?.set_password(password)?;
    Ok(())
}

pub fn get_password(user: &str) -> AppResult<Option<String>> {
    match entry(user)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn has_password(user: &str) -> bool {
    !user.is_empty() && matches!(get_password(user), Ok(Some(_)))
}
