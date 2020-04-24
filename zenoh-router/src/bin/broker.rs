use async_std::task;
use async_std::sync::Arc;
use rand::RngCore;
use zenoh_protocol::core::PeerId;
use zenoh_protocol::link::Locator;
use zenoh_protocol::proto::WhatAmI;
use zenoh_protocol::session::{SessionManager, SessionManagerConfig};
use zenoh_router::routing::tables::TablesHdl;

fn main() {
    task::block_on(async{
        let mut args = std::env::args();
        args.next(); // skip exe name
    
        let tables = Arc::new(TablesHdl::new());

        let mut pid = vec![0, 0, 0, 0];
        rand::thread_rng().fill_bytes(&mut pid);

        let batch_size: Option<usize> = match args.next() { 
            Some(size) => Some(size.parse().unwrap()),
            None => None
        };

        let self_locator: Locator = match args.next() { 
            Some(port) => {
                let mut s = "tcp/127.0.0.1:".to_string();
                s.push_str(&port);
                s.parse().unwrap()
            },
            None => "tcp/127.0.0.1:7447".parse().unwrap()
        };
    
        let config = SessionManagerConfig {
            version: 0,
            whatami: WhatAmI::Broker,
            id: PeerId{id: pid},
            handler: tables.clone(),
            lease: None,
            resolution: None,
            batchsize: batch_size,
            timeout: None,
            max_sessions: None,
            max_links: None 
        };
        let manager = SessionManager::new(config);

        if let Err(_err) = manager.add_locator(&self_locator).await {
            println!("Unable to open listening {}!", self_locator);
            std::process::exit(-1);
        }

        for locator in args {
            if let Err(_err) =  manager.open_session(&locator.parse().unwrap()).await {
                println!("Unable to connect to {}!", locator);
                std::process::exit(-1);
            }
        }
    
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10000));
        }
    });
}