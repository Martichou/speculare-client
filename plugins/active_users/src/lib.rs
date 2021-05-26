use std::io::Error;
use sys_metrics::host::get_logged_users;

#[no_mangle]
pub fn info() -> String {
    String::from("active_users")
}

#[no_mangle]
pub fn entrypoint() -> Result<String, Error> {
    match serde_json::to_string(&get_logged_users()?) {
        Ok(res_str) => Ok(res_str),
        Err(serde_err) => Err(Error::from(serde_err)),
    }
}

#[cfg(test)]
mod tests {
    use super::entrypoint;

    #[test]
    fn get_users() {
        entrypoint().unwrap();
    }
}
