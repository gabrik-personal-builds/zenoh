use async_std::sync::{
    Arc,
    channel
};
use async_std::task;
use async_trait::async_trait;

use zenoh_protocol::core::PeerId;
use zenoh_protocol::proto::{
    WhatAmI,
    Locator
};
use zenoh_protocol::session::{
    EmptyCallback,
    Session,
    SessionCallback,
    SessionHandler,
    SessionManager
};


// Session Handler for the router
struct SHRouter {}

impl SHRouter {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SessionHandler for SHRouter {
    async fn get_callback(&self, _peer: &PeerId) -> Arc<dyn SessionCallback + Send + Sync> {
        Arc::new(EmptyCallback::new())
    }

    async fn new_session(&self, _session: Arc<Session>) {}

    async fn del_session(&self, _session: &Arc<Session>) {}
}


// Session Handler for the client
struct SHClient {}

impl SHClient {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SessionHandler for SHClient {
    async fn get_callback(&self, _peer: &PeerId) -> Arc<dyn SessionCallback + Send + Sync> {
        Arc::new(EmptyCallback::new())
    }

    async fn new_session(&self, _session: Arc<Session>) {}

    async fn del_session(&self, _session: &Arc<Session>) {}
}


async fn run(locator: Locator) {
    let (t_sender, t_receiver) = channel::<()>(1);
    let (r_sender, r_receiver) = channel::<()>(1);

    // Broker task
    let l = locator.clone();
    task::spawn(async move {
        // Create the router session handler
        let routing = Arc::new(SHRouter::new());

        // Create the transport session manager
        let version = 0u8;
        let whatami = WhatAmI::Router;
        let id = PeerId{id: vec![0u8]};
        let lease = 60;    
        let manager = SessionManager::new(version, whatami, id, lease, routing);

        // Limit the number of connections to 1 for each listener
        // Not implemented at the moment
        let limit = Some(1);

        // Create the listeners
        match manager.new_listener(&l, limit).await {
            Ok(_) => (),
            Err(_) => ()
        }

        // Wait to be notified by the client
        r_receiver.recv().await;

        // Stop the listener
        let res = manager.del_listener(&l).await;
        assert_eq!(res.is_ok(), true);

        // Notify the main task
        t_sender.send(()).await;
    });

    // Client task
    let l = locator.clone();
    task::spawn(async move {
        // Create the client session handler
        let client = Arc::new(SHClient::new());

        // Create the transport session manager
        let version = 0u8;
        let whatami = WhatAmI::Client;
        let id = PeerId{id: vec![1u8]};
        let lease = 60;    
        let manager = SessionManager::new(version, whatami, id, lease, client);

        // Open session -> This should be accepted
        let res1 = manager.open_session(&l).await;
        assert_eq!(res1.is_ok(), true);

        // Open session -> This should be rejected
        let res2 = manager.open_session(&l).await;
        assert_eq!(res2.is_ok(), true);

        // Close the open session
        let session = res1.unwrap();
        let res3 = manager.close_session(&session.peer, None).await;
        assert_eq!(res3.is_ok(), true);

        // Notify the router we are done
        r_sender.send(()).await;
    });

    t_receiver.recv().await;
}

#[test]
fn session_tcp() {
    let locator: Locator = "tcp/127.0.0.1:8888".parse().unwrap();
    task::block_on(run(locator));
}