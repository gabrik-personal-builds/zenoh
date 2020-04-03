use async_trait::async_trait;
use crate::core::{ZInt, PeerId, ResKey};
use crate::io::RBuf;
use crate::proto::{SubInfo, QueryTarget, QueryConsolidation, ReplySource};

#[derive(Debug, Clone)]
pub enum Reply {
    ReplyData {source: ReplySource, replier_id: PeerId, reskey: ResKey, info: Option<RBuf>, payload: RBuf, },
    SourceFinal {source: ReplySource, replier_id: PeerId, },
    ReplyFinal,
} 

#[async_trait]
pub trait Primitives {
    async fn resource(&self, rid: ZInt, reskey: ResKey);
    async fn forget_resource(&self, rid: ZInt);
    
    async fn publisher(&self, reskey: ResKey);
    async fn forget_publisher(&self, reskey: ResKey);
    
    async fn subscriber(&self, reskey: ResKey, sub_info: SubInfo);
    async fn forget_subscriber(&self, reskey: ResKey);
    
    async fn storage(&self, reskey: ResKey);
    async fn forget_storage(&self, reskey: ResKey);
    
    async fn eval(&self, reskey: ResKey);
    async fn forget_eval(&self, reskey: ResKey);

    async fn data(&self, reskey: ResKey, reliable: bool, info: Option<RBuf>, payload: RBuf);
    async fn query(&self, reskey: ResKey, predicate: String, qid: ZInt, target: QueryTarget, consolidation: QueryConsolidation);
    async fn reply(&self, qid: ZInt, reply: Reply);
    async fn pull(&self, is_final: bool, reskey: ResKey, pull_id: ZInt, max_samples: Option<ZInt>);

    async fn close(&self);
}