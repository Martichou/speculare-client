#[macro_use]
extern crate text_io;
#[macro_use]
extern crate log;
extern crate libloading as lib;
extern crate num_integer;

mod clap;
mod harvest;
mod logger;
mod options;

use harvest::data_harvest::Data;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use num_integer::Integer;
use options::{
    config::{self},
    config_prompt, Config,
};
use std::{io::Error, thread, time::Duration};

/// Entrypoint which start the process and loop indefinietly.
///
/// No other way to stop it than killing the process (for now).
#[tokio::main]
async fn main() {
    // Init the logger and set the debug level correctly
    logger::configure();

    // Construct the --help menu and parse args more efficiently
    let args = clap::init_clap();

    // Detect if the user asked for config mode
    if args.is_present("config") {
        config_prompt::get_config_prompt();
        return;
    }

    // Get the config structure
    let config: Config = config::get_config(&args);

    // Create a Https "client" to be used in the Hyper Client
    let mut https_conn = HttpsConnector::new();
    https_conn.https_only(true);
    // Create a single Client instance for the app
    let client = Client::builder().build::<_, hyper::Body>(https_conn);

    // Int keeping track of the sending status
    let mut syncing_track: u64 = 0;
    // Compute the lcm of harvest_interval and syncing_interval to know when we should sync the data
    let syncing_threshold = config.harvest_interval.lcm(&config.syncing_interval);
    // Debug printing
    info!(
        "Will sync after {} cycles (lcm of harvest_interval and syncing_interval)",
        syncing_threshold
    );

    // Get the default Data instance
    let mut data: Data = Data::default();

    // Syncing memory cache
    let mut data_cache: Vec<Data> = Vec::with_capacity(syncing_threshold as usize);
    info!(
        "Initialized the data_cache with a size of {}",
        syncing_threshold
    );

    // Testing the dynamic lib loading
    // Hardcoded for now, will load dynamically in function of what is present in a folder
    // Need a lot of work to get a great plugin system
    // TODO:
    //  - Find how to get a fixed return type
    //  - Find how to send the info so that the server can understand it correctly
    //  - Add a helper function in the plugin so that the main client can know more info about it (?)
    trace!("Starting to load the plugin (active_users)");
    let lib_res = lib::Library::new("./plugins_compiled/libactive_users.so");
    let lib: lib::Library;
    if lib_res.is_err() {
        error!("the plugin failed to load: {:?}", lib_res.err());
    } else {
        trace!("the plugin (active_users) has been loaded correctly");
        lib = lib_res.unwrap();
        // Calling the function inside the plugin
        unsafe {
            let func: lib::Symbol<unsafe extern "C" fn() -> Result<String, Error>> =
                lib.get(b"entrypoint").unwrap();
            info!(
                "result of the entrypoint of the plugin (active_users) is: {:?}",
                func()
            );
        }
    }

    // Start the app loop (collect metrics and send them)
    loop {
        // Increment track of our syncing status
        syncing_track += 1;
        // Refresh / Populate the Data structure
        data.eat_data();
        // Saving data in a temp var/space if we don't sync it right away
        data_cache.push(data.clone());
        trace!("Data has been added to the data_cache");
        // Checking if we should sync
        if syncing_track % syncing_threshold == 0 {
            // Sending request to the server
            // TODO - Get rid of these unsafe unwrap
            let request = Request::builder()
                .method(Method::POST)
                .uri(&config.api_url)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&data_cache).unwrap()))
                .unwrap();
            // Execute the request
            trace!("sending POST request");
            match client.request(request).await {
                Ok(resp_body) => {
                    trace!("the POST request resulted in {:?}", resp_body);
                    // If no error, clear the data_cache
                    data_cache.clear();
                    trace!("data_cache has cleared");
                    // Reset the tracking counter
                    syncing_track = 0;
                }
                Err(hyper_err) => {
                    error!("the POST request resulted in {:?}", hyper_err);
                    // If data_cache contains too many items due to previous error
                    if data_cache.len() as u64 >= syncing_threshold * 10 {
                        // drain the first (older) items to avoid taking too much memory
                        data_cache.drain(0..(syncing_threshold * 2) as usize);
                        warn!(
                            "draining the first {} items of the data_cache",
                            syncing_threshold * 2
                        )
                    }
                }
            }
        }
        // Wait config.harvest_interval before running again
        // For syncing interval must be greater or equals to the harvest_interval
        // so just base this sleep on the harvest_interval value.
        thread::sleep(Duration::from_secs(config.harvest_interval));
    }
}
