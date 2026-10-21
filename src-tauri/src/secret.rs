use keyring::Entry;

const SERVICE: &str = "com.rolebit.desktop";
const BRIGHT_DATA_KEY: &str = "bright-data-api-key";
const DECISIONS_API_KEY: &str = "decisions-api-key";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, BRIGHT_DATA_KEY)
        .map_err(|error| format!("Secure credential storage is unavailable: {error}"))
}

fn decisions_entry() -> Result<Entry, String> {
    Entry::new(SERVICE, DECISIONS_API_KEY)
        .map_err(|error| format!("Secure credential storage is unavailable: {error}"))
}

pub fn set_api_key(value: &str) -> Result<(), String> {
    entry()?
        .set_password(value)
        .map_err(|error| format!("Could not store the API key securely: {error}"))
}

pub fn get_api_key() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read the API key securely: {error}")),
    }
}

pub fn delete_api_key() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not remove the API key: {error}")),
    }
}

pub fn set_decisions_api_key(value: &str) -> Result<(), String> {
    decisions_entry()?
        .set_password(value)
        .map_err(|error| format!("Could not store the Decisions API key securely: {error}"))
}

pub fn get_decisions_api_key() -> Result<Option<String>, String> {
    match decisions_entry()?.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!(
            "Could not read the Decisions API key securely: {error}"
        )),
    }
}

pub fn delete_decisions_api_key() -> Result<(), String> {
    match decisions_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not remove the Decisions API key: {error}")),
    }
}
