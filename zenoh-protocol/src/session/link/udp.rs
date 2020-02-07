use async_std::net::{
    SocketAddr,
    UdpSocket
};
use async_std::sync::{
    Arc,
    Mutex,
    RwLock,
    Weak
};
use async_std::task;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::{
    ArcSelf,
    zarcself,
    zerror
};
use crate::core::{
    ZError,
    ZErrorKind
};
use crate::io::RWBuf;
use crate::proto::{
    Locator,
    Message
};
use crate::session::{
    Session,
    Link,
    LinkManager
};

/*************************************/
/*              LINK                 */
/*************************************/
pub struct LinkUdp {
    locator: Locator,
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    buff_size: usize,
    session: Mutex<Arc<Session>>,
    next_session: Mutex<Option<Arc<Session>>>,
    manager: Arc<ManagerUdp>
}

impl LinkUdp {
    fn new(socket: Arc<UdpSocket>, addr: SocketAddr, session: Arc<Session>, manager: Arc<ManagerUdp>) -> Self {
        Self {
            locator: Locator::Udp{ addr: addr },
            socket: socket,
            addr: addr,
            buff_size: 8_192,
            session: Mutex::new(session),
            next_session: Mutex::new(None),
            manager: manager
        }
    }

    async fn open(local: SocketAddr, remote: SocketAddr, session: Arc<Session>, manager: Arc<ManagerUdp>) -> async_std::io::Result<Self> {
        let socket = UdpSocket::bind(local).await?;
        Ok(Self::new(Arc::new(socket), remote, session, manager))
    }

    async fn process(&self, msg: Message) {
        let mut session = self.session.lock().await;
        if let Some(next) = self.next_session.lock().await.take() {
            println!("NEXT: {}", next.id);
            *session = next;
        }
        session.receive_message(&self.locator, msg).await;
    }
}

#[async_trait]
impl Link for LinkUdp {
    async fn close(&self) -> Result<(), ZError> {
        self.manager.del_link(&self.addr).await;
        Ok(())
    }

    #[inline]
    async fn send(&self, message: Arc<Message>) -> Result<(), ZError> {
        // let mut buff = RWBuf::new(self.buff_size);
        // match buff.write_message(&message) {
        //     Ok(_) => {
        //         // Need to ensure that send_to is atomic and writes the whole buffer
        //         (&self.socket).send_to(buff.slice(), &self.remote).await?;
        //         return Ok(())
        //     },
        //     Err(_) => {}
        // }
        Ok(())
    }

    #[inline]
    async fn set_session(&self, session: Arc<Session>) -> Result<(), ZError> {
        *self.next_session.lock().await = Some(session);
        Ok(())
    }

    #[inline]
    fn get_locator(&self) -> Locator {
        self.locator.clone()
    }

    #[inline]
    fn get_mtu(&self) -> usize {
        65_536
    }

    #[inline]
    fn is_ordered(&self) -> bool {
        false
    }

    #[inline]
    fn is_reliable(&self) -> bool {
        false
    }
}


/*************************************/
/*          LISTENER                 */
/*************************************/
pub struct ManagerUdp {
    weak_self: RwLock<Weak<Self>>,
    addr: SocketAddr,
    session: Arc<Session>,
    link: RwLock<HashMap<SocketAddr, Arc<LinkUdp>>>,
    limit: Option<usize>
}

zarcself!(ManagerUdp);
impl ManagerUdp {
    pub fn new(addr: SocketAddr, session: Arc<Session>, limit: Option<usize>) -> Self {  
        Self {
            weak_self: RwLock::new(Weak::new()),
            addr: addr,
            session: session,
            link: RwLock::new(HashMap::new()),
            limit: limit
        }
    }

    #[inline]
    async fn add_link(&self, link: Arc<LinkUdp>) -> Option<Arc<LinkUdp>> {
        self.link.write().await.insert(link.addr, link.clone())
    }

    #[inline]
    async fn del_link(&self, addr: &SocketAddr) -> Option<Arc<LinkUdp>> {
        self.link.write().await.remove(addr)
    }
}

#[async_trait]
impl LinkManager for ManagerUdp {
    async fn new_link(&self, locator: &Locator) -> Result<Arc<dyn Link + Send + Sync>, ZError> {
        Err(zerror!(ZErrorKind::Other{
            msg: format!("")
        }))
    }

    async fn del_link(&self, locator: &Locator) -> Result<Arc<dyn Link + Send + Sync>, ZError> {
        Err(zerror!(ZErrorKind::Other{
            msg: format!("")
        }))
    }

    async fn new_listener(&self, locator: &Locator) -> Result<(), ZError> {
        Ok(())
    }

    async fn del_listener(&self, locator: &Locator) -> Result<(), ZError> {
        Ok(())
    }
}

async fn receive_loop(manager: Arc<ManagerUdp>) -> Result<(), ZError> {
    let socket = match UdpSocket::bind(manager.addr).await {
        Ok(socket) => Arc::new(socket),
        Err(e) => return Err(zerror!(ZErrorKind::Other{
            msg: format!("{}", e)
        }))
    }; 
    println!("Listening on: udp://{}", manager.addr);
    let mut buff = RWBuf::new(8_192);
    loop {
        // Wait for incoming traffic
        let peer: SocketAddr;
        match socket.recv_from(&mut buff.writable_slice()).await {
            Ok((n, p)) => {
                buff.set_write_pos(buff.write_pos() + n).unwrap();
                peer = p;
            },
            Err(_) => {
                continue
            }
        }
        // Add a new link if not existing
        let r_guard = manager.link.read().await;
        if !r_guard.contains_key(&peer) {
            if let Some(limit) = manager.limit {
                // Add a new link only if limit of connections is not exceeded
                if r_guard.len() >= limit {
                    continue
                } else {
                    println!("Accepting connection from: {:?}", peer);
                    // Create a new LinkUdp instance
                    let link = Arc::new(LinkUdp::new(socket.clone(), peer.clone(), manager.session.clone(), manager.clone()));
                    // Drop the read guard in order to allow the add_link to gain the write guard
                    drop(r_guard);
                    // Add the new LinkUdp instance to the manager
                    manager.add_link(link).await;
                }
            }
        }
        // Retrieve the link, this operation is expected to always succeed
        let r_guard = manager.link.read().await;
        let link = match r_guard.get(&peer) {
            Some(link) => link,
            None => continue
        };
        // Parse all the messages in the buffer
        loop {
            match buff.read_message() {
                Ok(message) => {
                    link.process(message).await;
                },
                Err(_) => {}
            }
        }
    }
}