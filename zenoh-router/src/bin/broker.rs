use async_std::future;
use async_std::sync::Arc;
use async_std::task;
use rand::RngCore;
use zenoh_protocol::core::PeerId;
use zenoh_protocol::link::Locator;
use zenoh_protocol::proto::whatami;
use zenoh_protocol::session::{SessionManager, SessionManagerConfig};
use zenoh_router::routing::broker::Broker;

fn print_usage(bin: String) {
    println!(
"Usage:
    cargo run --release --bin {} <locator to listen on>
Example: 
    cargo run --release --bin {} tcp/127.0.0.1:7447",
        bin, bin
    );
}

fn main() {
    task::block_on(async {
        let mut args = std::env::args();
        // Get exe name
        let bin = args.next().unwrap();  

        // Get next arg
        let value = if let Some(value) = args.next() {
            value
        } else {
            return print_usage(bin);
        };
        let listen_on: Locator = if let Ok(v) = value.parse() {
            v
        } else {
            return print_usage(bin);
        };

        // Create the broker
        let broker = Arc::new(Broker::new());
        // Initialize the PID
        let mut pid = vec![0, 0, 0, 0];
        rand::thread_rng().fill_bytes(&mut pid);
    
        let config = SessionManagerConfig {
            version: 0,
            whatami: whatami::BROKER,
            id: PeerId{id: pid},
            handler: broker.clone()
        };
        let manager = SessionManager::new(config, None);

        if let Err(_err) = manager.add_locator(&listen_on).await {
            println!("Unable to open listening {}!", listen_on);
            std::process::exit(-1);
        }

        let attachment = None;
        for locator in args {
            if let Err(_err) =  manager.open_session(&locator.parse().unwrap(), &attachment).await {
                println!("Unable to connect to {}!", locator);
                std::process::exit(-1);
            }
        }
        
        future::pending::<()>().await;
    });
}