use async_std::task;
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh_protocol::core::{PeerId, ResKey, ZInt};
use zenoh_protocol::io::ArcSlice;
use zenoh_protocol::proto::WhatAmI;
use zenoh_protocol::proto::{Primitives, SubMode, QueryConsolidation, QueryTarget, ReplySource};
use zenoh_protocol::session::SessionManager;
use zenoh_router::routing::tables::TablesHdl;

const N: usize = 100000;

struct Stats {
    count: usize,
    start: SystemTime,
    stop: SystemTime,
}

impl Stats {

    pub fn print(&self) {
        let t0 = self.start.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs()  as f64 
            + self.start.duration_since(UNIX_EPOCH).expect("Time went backwards").subsec_nanos() as f64 / 1000000000.0;
        let t1 = self.stop.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs()  as f64 
            + self.stop.duration_since(UNIX_EPOCH).expect("Time went backwards").subsec_nanos() as f64 / 1000000000.0;
        let thpt = N as f64 / (t1 - t0);
        println!("{} msgs/sec", thpt);
    }
}

pub struct ThrouputPrimitives {
    stats: Mutex<Stats>,
}

impl ThrouputPrimitives {
    pub fn new() -> ThrouputPrimitives {
        ThrouputPrimitives {
            stats: Mutex::new(Stats {
                count: 0,
                start: UNIX_EPOCH,
                stop: UNIX_EPOCH,
            })
        }
    }
}

#[async_trait]
impl Primitives for ThrouputPrimitives {

    async fn resource(&self, _rid: &ZInt, _reskey: &ResKey) {}
    async fn forget_resource(&self, _rid: &ZInt) {}
    
    async fn publisher(&self, _reskey: &ResKey) {}
    async fn forget_publisher(&self, _reskey: &ResKey) {}
    
    async fn subscriber(&self, _reskey: &ResKey, _mode: &SubMode) {}
    async fn forget_subscriber(&self, _reskey: &ResKey) {}
    
    async fn storage(&self, _reskey: &ResKey) {}
    async fn forget_storage(&self, _reskey: &ResKey) {}
    
    async fn eval(&self, _reskey: &ResKey) {}
    async fn forget_eval(&self, _reskey: &ResKey) {}

    async fn data(&self, _reskey: &ResKey, _info: &Option<ArcSlice>, _payload: &ArcSlice) {
        let mut stats = self.stats.lock().await;
        if stats.count == 0 {
            stats.start = SystemTime::now();
            stats.count += 1;
        } else if stats.count < N {
            stats.count += 1;
        } else {
            stats.stop = SystemTime::now();
            stats.print();
            stats.count = 0;
        }  
    }
    async fn query(&self, _reskey: &ResKey, _predicate: &String, _qid: &ZInt, _target: &Option<QueryTarget>, _consolidation: &QueryConsolidation) {}
    async fn reply(&self, _qid: &ZInt, _source: &ReplySource, _replierid: &Option<PeerId>, _reskey: &ResKey, _info: &Option<ArcSlice>, _payload: &ArcSlice) {}
    async fn pull(&self, _is_final: bool, _reskey: &ResKey, _pull_id: &ZInt, _max_samples: &Option<ZInt>) {}

    async fn close(&self) {}
}

fn main() {
    task::block_on(async{
        let mut args = std::env::args();
        args.next(); // skip exe name

        let my_primitives = Arc::new(ThrouputPrimitives::new());
    
        let tables = Arc::new(TablesHdl::new());

        let mut pid = vec![0, 0, 0, 0];
        rand::thread_rng().fill_bytes(&mut pid);
    
        let manager = SessionManager::new(0, WhatAmI::Client, PeerId{id: pid}, 0, tables.clone());

        while let Some(locator) = args.next() {
            if let Err(_err) =  manager.open_session(&locator.parse().unwrap()).await {
                println!("Unable to connect to {}!", locator);
                std::process::exit(-1);
            }
        }
    
        let primitives = tables.new_primitives(my_primitives).await;

        primitives.resource(&1, &"/tp".to_string().into()).await;
        let rid = ResKey::RId(1);
        primitives.subscriber(&rid, &SubMode::Push).await;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(10000));
        }
    });
}