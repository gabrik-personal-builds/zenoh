use async_std::task;
use async_std::sync::Arc;
use async_trait::async_trait;
use rand::RngCore;
use zenoh_protocol::core::{PeerId, ResKey, ZInt};
use zenoh_protocol::io::ArcSlice;
use zenoh_protocol::proto::WhatAmI;
use zenoh_protocol::proto::{Primitives, SubMode, QueryConsolidation, QueryTarget, ReplySource};
use zenoh_protocol::session::SessionManager;
use zenoh_router::routing::tables::TablesHdl;

pub struct PrintPrimitives {
}

#[async_trait]
impl Primitives for PrintPrimitives {

    async fn resource(&self, rid: ZInt, reskey: &ResKey) {
        println!("  [RECV] RESOURCE ({:?}) ({:?})", rid, reskey);
    }
    async fn forget_resource(&self, rid: ZInt) {
        println!("  [RECV] FORGET RESOURCE ({:?})", rid);
    }
    
    async fn publisher(&self, reskey: &ResKey) {
        println!("  [RECV] PUBLISHER ({:?})", reskey);
    }
    async fn forget_publisher(&self, reskey: &ResKey) {
        println!("  [RECV] FORGET PUBLISHER ({:?})", reskey);
    }
    
    async fn subscriber(&self, reskey: &ResKey, mode: &SubMode) {
        println!("  [RECV] SUBSCRIBER ({:?}) ({:?})", reskey, mode);
    }
    async fn forget_subscriber(&self, reskey: &ResKey) {
        println!("  [RECV] FORGET SUBSCRIBER ({:?})", reskey);
    }
    
    async fn storage(&self, reskey: &ResKey) {
        println!("  [RECV] STORAGE ({:?})", reskey);
    }
    async fn forget_storage(&self, reskey: &ResKey) {
        println!("  [RECV] FORGET STORAGE ({:?})", reskey);
    }
    
    async fn eval(&self, reskey: &ResKey) {
        println!("  [RECV] EVAL ({:?})", reskey);
    }
    async fn forget_eval(&self, reskey: &ResKey) {
        println!("  [RECV] FORGET EVAL ({:?})", reskey);
    }

    async fn data(&self, reskey: &ResKey, _info: &Option<ArcSlice>, _payload: &ArcSlice) {
        println!("  [RECV] DATA ({:?})", reskey);
    }
    async fn query(&self, reskey: &ResKey, predicate: &str, qid: ZInt, target: &Option<QueryTarget>, consolidation: &QueryConsolidation) {
        println!("  [RECV] QUERY ({:?}) ({:?}) ({:?}) ({:?}) ({:?})", reskey, predicate, qid, target, consolidation);
    }
    async fn reply(&self, qid: ZInt, source: &ReplySource, replierid: &Option<PeerId>, reskey: &ResKey, _info: &Option<ArcSlice>, _payload: &ArcSlice) {
        println!("  [RECV] REPLY ({:?}) ({:?}) ({:?}) ({:?})", qid, source, replierid, reskey);
    }
    async fn pull(&self, is_final: bool, reskey: &ResKey, pull_id: ZInt, max_samples: &Option<ZInt>) {
        println!("  [RECV] PULL ({:?}) ({:?}) ({:?}) ({:?})", is_final, reskey, pull_id, max_samples);
    }

    async fn close(&self) {
        println!("  CLOSE");
    }
}

fn main() {
    task::block_on(async{
        let mut args = std::env::args();
        args.next(); // skip exe name

        let my_primitives = Arc::new(PrintPrimitives {});
    
        let tables = Arc::new(TablesHdl::new());

        let mut pid = vec![0, 0, 0, 0];
        rand::thread_rng().fill_bytes(&mut pid);
    
        let manager = SessionManager::new(0, WhatAmI::Client, PeerId{id: pid.clone()}, 0, tables.clone());

        while let Some(locator) = args.next() {
            if let Err(_err) =  manager.open_session(&locator.parse().unwrap()).await {
                println!("Unable to connect to {}!", locator);
                std::process::exit(-1);
            }
        }
    
        let primitives = tables.new_primitives(my_primitives).await;

        primitives.subscriber(&"/demo/**".to_string().into(), &SubMode::Push).await;

        let res: ResKey = ["/demo/client/", &pid[0].to_string(), &pid[1].to_string(), &pid[2].to_string(), &pid[3].to_string()]
            .concat().to_string().into();
        loop {
            println!("[SEND] DATA ({:?})", &res);
            primitives.data(&res, &None, &ArcSlice::from(vec![1])).await;
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });
}