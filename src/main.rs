extern crate reqwest;
extern crate sys_info;
extern crate sysinfo;

mod models;

use crate::models::*;

use std::error::Error;
use std::process::Command;
use sys_info::hostname;
use sysinfo::ComponentExt;
use sysinfo::{ProcessorExt, System, SystemExt};

/* Will only succed on iMac - send message and panic if needed */
fn syslog(message: String, fail: bool) {
    Command::new("bash").arg("-c").arg(format!(
        "/bin/syslog.py {}",
        format!("[SPECULARE] - {}", message)
    ));
    if fail {
        panic!(message);
    }
}

/*
 *  MAC - linux specific get_mac_address
 *  using default interface in a safe string
 */
#[cfg(target_os = "linux")]
fn get_mac_address() -> String {
    let interface = Command::new("bash")
        .arg("-c")
        .arg("route | grep '^default' | grep -o '[^ ]*$' | sed '$!d' | tr -d '\n'")
        .output()
        .expect("failed to retrieve default interface");
    let mac_address = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "ifconfig {} | grep 'ether ' | awk {} | tr -d '\n'",
            String::from_utf8_lossy(&interface.stdout),
            "'{print $2}'"
        ))
        .output();
    return match mac_address {
        Ok(val) => String::from_utf8_lossy(&val.stdout).to_string(),
        Err(x) => {
            syslog(x.to_string(), false);
            x.to_string()
        }
    };
}

/*
 *  MAC - macos specific get_mac_address
 *  using default interface in a safe string
 */
#[cfg(target_os = "macos")]
fn get_mac_address() -> String {
    let interface = Command::new("bash")
        .arg("-c")
        .arg("route -n get default | grep 'interface:' | grep -o '[^ ]*$' | sed '$!d' | tr -d '\n'")
        .output()
        .expect("failed to retrieve default interface");
    let mac_address = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "ifconfig {} | grep 'ether ' | awk {} | tr -d '\n'",
            String::from_utf8_lossy(&interface.stdout),
            "'{print $2}'"
        ))
        .output();
    return match mac_address {
        Ok(val) => String::from_utf8_lossy(&val.stdout).to_string(),
        Err(x) => {
            syslog(x.to_string(), false);
            x.to_string()
        }
    };
}

/* Get the user currently logged, if more than 1 user, return the last one */
fn get_logged_user() -> String {
    let logged_users = Command::new("bash")
        .arg("-c")
        .arg("users | awk -F' ' '{print $NF}' | tr -d '\n'")
        .output();
    return match logged_users {
        Ok(val) => String::from_utf8_lossy(&val.stdout).to_string(),
        Err(x) => {
            syslog(x.to_string(), false);
            x.to_string()
        }
    };
}

/* Get the os version (Mac/Linux/Windows) in a safe String */
fn get_os_version() -> String {
    let os_release = os_version::detect();
    return match os_release {
        Ok(val) => val.to_string(),
        Err(x) => {
            syslog(x.to_string(), false);
            x.to_string()
        }
    };
}

/* Get the hostname (Mac/Linux/Windows) in a safe String */
fn get_hostname() -> String {
    return match hostname() {
        Ok(val) => val.to_string(),
        Err(x) => {
            syslog(x.to_string(), false);
            x.to_string()
        }
    };
}

/* Get the uuid of the host (Mac/Linux/Windows) in a safe String */
fn get_uuid() -> String {
    return match machine_uid::get() {
        Ok(val) => val.to_string(),
        Err(x) => {
            syslog(x.to_string(), false);
            x.to_string()
        }
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::from_path("/etc/speculare.config").unwrap_or_else(|_error| {
        syslog("failed to load /etc/speculare.config".to_string(), true);
    });

    let sys = System::new_all();

    let components = sys.get_components();
    let mut sensors: Vec<Sensors> = Vec::with_capacity(components.len());
    for component in components {
        sensors.push(Sensors {
            label: component.get_label().to_string(),
            temp: component.get_temperature(),
        })
    }

    let data = Data {
        os: get_os_version(),
        hostname: get_hostname(),
        uptime: sys.get_uptime(),
        uuid: get_uuid(),
        cpu_freq: sys.get_processors()[0].get_frequency(),
        user: get_logged_user(),
        sensors: sensors,
        mac_address: get_mac_address(),
    };

    let mut url: String = String::new();
    match std::env::var("api_url") {
        Ok(val) => url.push_str(&val),
        Err(x) => {
            syslog(x.to_string(), true);
        }
    };

    let mut token: String = String::new();
    match std::env::var("api_token") {
        Ok(val) => token.push_str(&val),
        Err(x) => {
            syslog(x.to_string(), true);
        }
    };

    let res = reqwest::Client::new()
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&data)
        .send()
        .await?;

    println!("Return code [{}]", res.status());
    syslog(format!("return code [{}]", res.status()), false);
    Ok(())
}
