use async_std::task;
use async_std::sync::Arc;
use async_trait::async_trait;
use std::convert::TryInto;
use zenoh_protocol::core::rname::intersect;
use zenoh_protocol::core::{ResKey, ZInt};
use zenoh_protocol::io::RBuf;
use zenoh_protocol::proto::{Primitives, Mux, Reliability, SubMode, SubInfo, WhatAmI, QueryConsolidation, QueryTarget, Reply};
use zenoh_protocol::session::DummyHandler;
use zenoh_router::routing::tables::Tables;
use zenoh_router::routing::resource::Resource;

#[test]
fn base_test() {
    task::block_on(async{
        let tables = Tables::new();
        let primitives = Arc::new(Mux::new(Arc::new(DummyHandler::new())));
        let sex = Tables::declare_session(&tables, WhatAmI::Client, primitives.clone()).await;
        Tables::declare_resource(&tables, &sex, 1, 0, "/one/two/three").await;
        Tables::declare_resource(&tables, &sex, 2, 0, "/one/deux/trois").await;
        
        let sub_info = SubInfo {
            reliability: Reliability::Reliable,
            mode: SubMode::Push,
            period: None
        };
            Tables::declare_subscription(&tables, &sex, 1, "/four/five", &sub_info).await;

        Tables::print(&tables).await;
    });
}

#[test]
fn match_test() {
    task::block_on(async{
        let rnames = [
            "/", "/a", "/a/", "/a/b", "/*", "/abc", "/abc/", "/*/", "xxx", 
            "/ab*", "/abcd", "/ab*d", "/ab", "/ab/*", "/a/*/c/*/e", "/a/b/c/d/e", 
            "/a/*b/c/*d/e", "/a/xb/c/xd/e", "/a/c/e", "/a/b/c/d/x/e", "/ab*cd", 
            "/abxxcxxd", "/abxxcxxcd", "/abxxcxxcdx", "/**", "/a/b/c", "/a/b/c/", 
            "/**/", "/ab/**", "/**/xyz", "/a/b/xyz/d/e/f/xyz", "/**/xyz*xyz", 
            "/a/b/xyz/d/e/f/xyz", "/a/**/c/**/e", "/a/b/b/b/c/d/d/d/e", 
            "/a/**/c/*/e/*", "/a/b/b/b/c/d/d/c/d/e/f", "/a/**/c/*/e/*", 
            "/x/abc", "/x/*", "/x/abc*", "/x/*abc", "/x/a*", "/x/a*de", 
            "/x/abc*de", "/x/a*d*e", "/x/a*e", "/x/a*c*e", "/x/ade", "/x/c*", 
            "/x/*d", "/x/*e"
        ];

        let tables = Tables::new();
        let primitives = Arc::new(Mux::new(Arc::new(DummyHandler::new())));
        let sex = Tables::declare_session(&tables, WhatAmI::Client, primitives.clone()).await;
        for (i, rname) in rnames.iter().enumerate() {
            Tables::declare_resource(&tables, &sex, i.try_into().unwrap(), 0, rname).await;
        }

        for rname1 in rnames.iter() {
            let res_matches = Tables::get_matches(&tables, rname1).await;
            let matches:Vec<String> = res_matches.iter().map(|m| {m.upgrade().unwrap().name()}).collect();
            for rname2 in rnames.iter() {
                if matches.contains(&String::from(*rname2)) {
                    assert!(   intersect(rname1, rname2));
                } else {
                    assert!( ! intersect(rname1, rname2));
                }
            }
        }
    });
}

#[test]
fn clean_test() {
    task::block_on(async{
        let tables = Tables::new();

        let primitives = Arc::new(Mux::new(Arc::new(DummyHandler::new())));
        let sex0 = Tables::declare_session(&tables, WhatAmI::Client, primitives.clone()).await;
        assert!(sex0.upgrade().is_some());

        // --------------
        Tables::declare_resource(&tables, &sex0, 1, 0, "/todrop1").await;
        let optres1 = Resource::get_resource(&tables.read().await._get_root(), "/todrop1").map(|res| {Arc::downgrade(&res)});
        assert!(optres1.is_some());
        let res1 = optres1.unwrap();
        assert!(res1.upgrade().is_some());

        Tables::declare_resource(&tables, &sex0, 2, 0, "/todrop1/todrop11").await;
        let optres2 = Resource::get_resource(&tables.read().await._get_root(), "/todrop1/todrop11").map(|res| {Arc::downgrade(&res)});
        assert!(optres2.is_some());
        let res2 = optres2.unwrap();
        assert!(res2.upgrade().is_some());

        Tables::declare_resource(&tables, &sex0, 3, 0, "/**").await;
        let optres3 = Resource::get_resource(&tables.read().await._get_root(), "/**").map(|res| {Arc::downgrade(&res)});
        assert!(optres3.is_some());
        let res3 = optres3.unwrap();
        assert!(res3.upgrade().is_some());

        Tables::undeclare_resource(&tables, &sex0, 1).await;
        assert!(res1.upgrade().is_some());
        assert!(res2.upgrade().is_some());
        assert!(res3.upgrade().is_some());

        Tables::undeclare_resource(&tables, &sex0, 2).await;
        assert!( ! res1.upgrade().is_some());
        assert!( ! res2.upgrade().is_some());
        assert!(res3.upgrade().is_some());

        Tables::undeclare_resource(&tables, &sex0, 3).await;
        assert!( ! res1.upgrade().is_some());
        assert!( ! res2.upgrade().is_some());
        assert!( ! res3.upgrade().is_some());

        // --------------
        Tables::declare_resource(&tables, &sex0, 1, 0, "/todrop1").await;
        let optres1 = Resource::get_resource(&tables.read().await._get_root(), "/todrop1").map(|res| {Arc::downgrade(&res)});
        assert!(optres1.is_some());
        let res1 = optres1.unwrap();
        assert!(res1.upgrade().is_some());

        let sub_info = SubInfo {
            reliability: Reliability::Reliable,
            mode: SubMode::Push,
            period: None
        };
    
        Tables::declare_subscription(&tables, &sex0, 0, "/todrop1/todrop11", &sub_info).await;
        let optres2 = Resource::get_resource(&tables.read().await._get_root(), "/todrop1/todrop11").map(|res| {Arc::downgrade(&res)});
        assert!(optres2.is_some());
        let res2 = optres2.unwrap();
        assert!(res2.upgrade().is_some());

        Tables::declare_subscription(&tables, &sex0, 1, "/todrop12", &sub_info).await;
        let optres3 = Resource::get_resource(&tables.read().await._get_root(), "/todrop1/todrop12").map(|res| {Arc::downgrade(&res)});
        assert!(optres3.is_some());
        let res3 = optres3.unwrap();
        assert!(res3.upgrade().is_some());

        Tables::undeclare_subscription(&tables, &sex0, 1, "/todrop12").await;
        assert!(res1.upgrade().is_some());
        assert!(res2.upgrade().is_some());
        assert!( ! res3.upgrade().is_some());

        Tables::undeclare_subscription(&tables, &sex0, 0, "/todrop1/todrop11").await;
        assert!(res1.upgrade().is_some());
        assert!( ! res2.upgrade().is_some());
        assert!( ! res3.upgrade().is_some());

        Tables::undeclare_resource(&tables, &sex0, 1).await;
        assert!( ! res1.upgrade().is_some());
        assert!( ! res2.upgrade().is_some());
        assert!( ! res3.upgrade().is_some());

        // --------------
        Tables::declare_resource(&tables, &sex0, 2, 0, "/todrop3").await;
        Tables::declare_subscription(&tables, &sex0, 0, "/todrop3", &sub_info).await;
        let optres1 = Resource::get_resource(&tables.read().await._get_root(), "/todrop3").map(|res| {Arc::downgrade(&res)});
        assert!(optres1.is_some());
        let res1 = optres1.unwrap();
        assert!(res1.upgrade().is_some());

        Tables::undeclare_subscription(&tables, &sex0, 0, "/todrop3").await;
        assert!(res1.upgrade().is_some());

        Tables::undeclare_resource(&tables, &sex0, 2).await;
        assert!( ! res1.upgrade().is_some());

        // --------------
        Tables::declare_resource(&tables, &sex0, 3, 0, "/todrop4").await;
        Tables::declare_resource(&tables, &sex0, 4, 0, "/todrop5").await;
        Tables::declare_subscription(&tables, &sex0, 0, "/todrop5", &sub_info).await;
        Tables::declare_subscription(&tables, &sex0, 0, "/todrop6", &sub_info).await;

        let optres1 = Resource::get_resource(&tables.read().await._get_root(), "/todrop4").map(|res| {Arc::downgrade(&res)});
        assert!(optres1.is_some());
        let res1 = optres1.unwrap();
        let optres2 = Resource::get_resource(&tables.read().await._get_root(), "/todrop5").map(|res| {Arc::downgrade(&res)});
        assert!(optres2.is_some());
        let res2 = optres2.unwrap();
        let optres3 = Resource::get_resource(&tables.read().await._get_root(), "/todrop6").map(|res| {Arc::downgrade(&res)});
        assert!(optres3.is_some());
        let res3 = optres3.unwrap();

        assert!(res1.upgrade().is_some());
        assert!(res2.upgrade().is_some());
        assert!(res3.upgrade().is_some());

        Tables::undeclare_session(&tables, &sex0).await;
        assert!( ! sex0.upgrade().is_some());
        assert!( ! res1.upgrade().is_some());
        assert!( ! res2.upgrade().is_some());
        assert!( ! res3.upgrade().is_some());
    });
}

pub struct Data{
    _key: String, 
    _payload: RBuf
}

pub struct ClientPrimitives {
    data: std::sync::Mutex<Option<Data>>,
    mapping: std::sync::Mutex<std::collections::HashMap<ZInt, String>>,
}

impl ClientPrimitives {
    pub fn new() -> ClientPrimitives {
        ClientPrimitives {
            data: std::sync::Mutex::new(None),
            mapping: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn clear_data(&self) {
        *self.data.lock().unwrap() = None;
    }
}

impl Default for ClientPrimitives {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientPrimitives {
    fn get_name(&self, reskey: &ResKey) -> String {
        let mapping = self.mapping.lock().unwrap();
        match reskey {
            ResKey::RName(name) => {name.clone()}
            ResKey::RId(id) => {mapping.get(id).unwrap().clone()}
            ResKey::RIdWithSuffix(id, suffix) => {[&mapping.get(id).unwrap()[..], &suffix[..]].concat()}
        }
    }
}

#[async_trait]
impl Primitives for ClientPrimitives {

    async fn resource(&self, rid: ZInt, reskey: &ResKey) {
        let name = self.get_name(reskey);
        self.mapping.lock().unwrap().insert(rid, name);
    }
    async fn forget_resource(&self, rid: ZInt) {
        self.mapping.lock().unwrap().remove(&rid);
    }
    
    async fn publisher(&self, _reskey: &ResKey) {}
    async fn forget_publisher(&self, _reskey: &ResKey) {}
    
    async fn subscriber(&self, _reskey: &ResKey, _sub_info: &SubInfo) {}
    async fn forget_subscriber(&self, _reskey: &ResKey) {}
    
    async fn queryable(&self, _reskey: &ResKey) {}
    async fn forget_queryable(&self, _reskey: &ResKey) {}

    async fn data(&self, _reskey: &ResKey, _reliable: bool, _info: &Option<RBuf>, _payload: RBuf) {}
    async fn query(&self, _reskey: &ResKey, _predicate: &str, _qid: ZInt, _target: QueryTarget, _consolidation: QueryConsolidation) {}
    async fn reply(&self, _qid: ZInt, _reply: &Reply) {}
    async fn pull(&self, _is_final: bool, _reskey: &ResKey, _pull_id: ZInt, _max_samples: &Option<ZInt>) {}

    async fn close(&self) {}
}


#[test]
fn client_test() {
    task::block_on(async{
        let tables = Tables::new();
        let sub_info = SubInfo {
            reliability: Reliability::Reliable,
            mode: SubMode::Push,
            period: None
        };
        
        let primitives0 = Arc::new(ClientPrimitives::new());
        let sex0 = Tables::declare_session(&tables, WhatAmI::Client, primitives0.clone()).await;
        Tables::declare_resource(&tables, &sex0, 11, 0, "/test/client").await;
        primitives0.resource(11, &ResKey::RName("/test/client".to_string())).await;
        Tables::declare_subscription(&tables, &sex0, 11, "/**", &sub_info).await;
        Tables::declare_resource(&tables, &sex0, 12, 11, "/z1_pub1").await;
        primitives0.resource(12, &ResKey::RIdWithSuffix(11, "/z1_pub1".to_string())).await;

        let primitives1 = Arc::new(ClientPrimitives::new());
        let sex1 = Tables::declare_session(&tables, WhatAmI::Client, primitives1.clone()).await;
        Tables::declare_resource(&tables, &sex1, 21, 0, "/test/client").await;
        primitives1.resource(21, &ResKey::RName("/test/client".to_string())).await;
        Tables::declare_subscription(&tables, &sex1, 21, "/**", &sub_info).await;
        Tables::declare_resource(&tables, &sex1, 22, 21, "/z2_pub1").await;
        primitives1.resource(22, &ResKey::RIdWithSuffix(21, "/z2_pub1".to_string())).await;

        let primitives2 = Arc::new(ClientPrimitives::new());
        let sex2 = Tables::declare_session(&tables, WhatAmI::Client, primitives2.clone()).await;
        Tables::declare_resource(&tables, &sex2, 31, 0, "/test/client").await;
        primitives2.resource(31, &ResKey::RName("/test/client".to_string())).await;
        Tables::declare_subscription(&tables, &sex2, 31, "/**", &sub_info).await;

        
        let result_opt = Tables::route_data_to_map(&tables, &sex0, 0, "/test/client/z1_wr1").await; 
        assert!(result_opt.is_some());
        let result = result_opt.unwrap();

        let opt_sex = result.get(&0);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives0.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_wr1");
        // mapping strategy check
        assert_eq!(*id, 11);
        assert_eq!(suffix, "/z1_wr1");

        let opt_sex = result.get(&1);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives1.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_wr1");
        // mapping strategy check
        // assert_eq!(*id, 21); Temporarily skip this test
        assert_eq!(suffix, "/z1_wr1");

        let opt_sex = result.get(&2);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives2.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_wr1");
        // mapping strategy check
        // assert_eq!(*id, 31); Temporarily skip this test
        assert_eq!(suffix, "/z1_wr1");

        
        let result_opt = Tables::route_data_to_map(&tables, &sex0, 11, "/z1_wr2").await; 
        assert!(result_opt.is_some());
        let result = result_opt.unwrap();

        let opt_sex = result.get(&0);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives0.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_wr2");
        // mapping strategy check
        assert_eq!(*id, 11);
        assert_eq!(suffix, "/z1_wr2");

        let opt_sex = result.get(&1);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives1.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_wr2");
        // mapping strategy check
        // assert_eq!(*id, 21); Temporarily skip this test
        assert_eq!(suffix, "/z1_wr2");

        let opt_sex = result.get(&2);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives2.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_wr2");
        // mapping strategy check
        // assert_eq!(*id, 31); Temporarily skip this test
        assert_eq!(suffix, "/z1_wr2");

        
        let result_opt = Tables::route_data_to_map(&tables, &sex1, 0, "/test/client/**").await; 
        assert!(result_opt.is_some());
        let result = result_opt.unwrap();

        let opt_sex = result.get(&0);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives0.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/**");
        // mapping strategy check
        assert_eq!(*id, 11);
        assert_eq!(suffix, "/**");

        let opt_sex = result.get(&1);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives1.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/**");
        // mapping strategy check
        // assert_eq!(*id, 21); Temporarily skip this test
        assert_eq!(suffix, "/**");

        let opt_sex = result.get(&2);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives2.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/**");
        // mapping strategy check
        // assert_eq!(*id, 31); Temporarily skip this test
        assert_eq!(suffix, "/**");

        
        let result_opt = Tables::route_data_to_map(&tables, &sex0, 12, "").await; 
        assert!(result_opt.is_some());
        let result = result_opt.unwrap();

        let opt_sex = result.get(&0);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives0.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_pub1");
        // mapping strategy check
        assert_eq!(*id, 12);
        assert_eq!(suffix, "");

        let opt_sex = result.get(&1);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives1.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_pub1");
        // mapping strategy check
        // assert_eq!(*id, 21); Temporarily skip this test
        assert_eq!(suffix, "/z1_pub1");

        let opt_sex = result.get(&2);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives2.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z1_pub1");
        // mapping strategy check
        // assert_eq!(*id, 31); Temporarily skip this test
        assert_eq!(suffix, "/z1_pub1");

        
        let result_opt = Tables::route_data_to_map(&tables, &sex1, 22, "").await; 
        assert!(result_opt.is_some());
        let result = result_opt.unwrap();

        let opt_sex = result.get(&0);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives0.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z2_pub1");
        // mapping strategy check
        assert_eq!(*id, 11);
        assert_eq!(suffix, "/z2_pub1");

        let opt_sex = result.get(&1);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives1.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z2_pub1");
        // mapping strategy check
        assert_eq!(*id, 22);
        assert_eq!(suffix, "");

        let opt_sex = result.get(&2);
        assert!(opt_sex.is_some());
        let (_, id, suffix) = opt_sex.unwrap();
        // functionnal check
        assert_eq!(primitives2.get_name(&ResKey::RIdWithSuffix(*id, suffix.clone())), "/test/client/z2_pub1");
        // mapping strategy check
        // assert_eq!(*id, 31); Temporarily skip this test
        assert_eq!(suffix, "/z2_pub1");
    });
}


