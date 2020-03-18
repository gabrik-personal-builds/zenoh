use async_std::task;
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh_protocol::core::{PeerId, ResKey, ZInt};
use zenoh_protocol::io::ArcSlice;
use zenoh_protocol::proto::{Primitives, SubMode, QueryConsolidation, QueryTarget, ReplySource, WhatAmI, Mux, DeMux};
use zenoh_protocol::session::{SessionManager, SessionHandler, MsgHandler};

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

    async fn resource(&self, _rid: ZInt, _reskey: &ResKey) {}
    async fn forget_resource(&self, _rid: ZInt) {}
    
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
    async fn query(&self, _reskey: &ResKey, _predicate: &str, _qid: ZInt, _target: &Option<QueryTarget>, _consolidation: &QueryConsolidation) {}
    async fn reply(&self, _qid: ZInt, _source: &ReplySource, _replierid: &Option<PeerId>, _reskey: &ResKey, _info: &Option<ArcSlice>, _payload: &ArcSlice) {}
    async fn pull(&self, _is_final: bool, _reskey: &ResKey, _pull_id: ZInt, _max_samples: &Option<ZInt>) {}

    async fn close(&self) {}
}

struct LightSessionHandler {
    pub handler: Mutex<Option<Arc<dyn MsgHandler + Send + Sync>>>,
}

impl LightSessionHandler {
    pub fn new() -> LightSessionHandler {
        LightSessionHandler { handler: Mutex::new(None),}
    }
}

#[async_trait]
impl SessionHandler for LightSessionHandler {
    async fn new_session(&self, _whatami: WhatAmI, session: Arc<dyn MsgHandler + Send + Sync>) -> Arc<dyn MsgHandler + Send + Sync> {
        *self.handler.lock().await = Some(session);
        Arc::new(DeMux::new(ThrouputPrimitives::new()))
    }
}

fn main() {
    task::block_on(async{
        let mut args = std::env::args();
        args.next(); // skip exe name

        let mut pid = vec![0, 0, 0, 0];
        rand::thread_rng().fill_bytes(&mut pid);

        let session_handler = Arc::new(LightSessionHandler::new());
        let manager = SessionManager::new(0, WhatAmI::Client, PeerId{id: pid.clone()}, 0, session_handler.clone());

        if let Some(locator) = args.next() {
            if let Err(_err) =  manager.open_session(&locator.parse().unwrap()).await {
                println!("Unable to connect to {}!", locator);
                std::process::exit(-1);
            }
        }
    
        let primitives = Mux::new(session_handler.handler.lock().await.as_ref().unwrap().clone());

        primitives.resource(1, &"/tp".to_string().into()).await;
        let rid = ResKey::RId(1);
        primitives.subscriber(&rid, &SubMode::Push).await;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(10000));
        }
    });
}