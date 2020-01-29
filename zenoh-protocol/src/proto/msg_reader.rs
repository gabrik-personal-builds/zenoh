use crate::io::rwbuf::{RWBuf, OutOfBounds};
use crate::core::{ZInt, PeerId, Property, ResKey, TimeStamp};
use super::msg::*;
use super::decl::{Declaration, SubMode};
use std::sync::Arc;
use std::convert::TryInto;

impl RWBuf {

    pub fn read_message(&mut self) -> Result<Message, OutOfBounds> {
        use super::msg::id::*;

        let mut _kind = MessageKind::FullMessage;
        let mut cid = None;
        let mut reply_context = None;
        let mut properties : Option<Arc<Vec<Property>>> = None;

        loop {
            let header = self.read()?;
            match flag::mid(header) {
                FRAGMENT => {
                    _kind = self.read_decl_frag(header)?;
                    continue;
                }

                CONDUIT => {
                    cid = Some(self.read_decl_conduit(header)?);
                    continue;
                }

                REPLY => {
                    reply_context = Some(self.read_decl_reply(header)?);
                    continue;
                }

                PROPERTIES => {
                    properties = Some(Arc::new(self.read_decl_properties(header)?));
                    continue;
                }

                SCOUT => {
                    let what = if flag::has_flag(header, flag::W) {
                        Some(self.read_zint()?)
                    } else { None };
                    return Ok(Message::make_scout(what, cid, properties));
                }

                HELLO => {
                    let whatami = if flag::has_flag(header, flag::W) {
                        Some(self.read_zint()?)
                    } else { None };
                    let locators = if flag::has_flag(header, flag::L) {
                        Some(self.read_locators()?)
                    } else { None };
                    return Ok(Message::make_hello(whatami, locators, cid, properties));
                }

                OPEN => {
                    let version = self.read()?;
                    let whatami = if flag::has_flag(header, flag::W) {
                        Some(self.read_zint()?)
                    } else { None };
                    let pid = self.read_peerid()?;
                    let lease = self.read_zint()?;
                    let locators = if flag::has_flag(header, flag::L) {
                        Some(self.read_locators()?)
                    } else { None };
                    return Ok(Message::make_open(version, whatami, pid, lease, locators, cid, properties));
                }

                ACCEPT => {
                    let opid = self.read_peerid()?;
                    let apid = self.read_peerid()?;
                    let lease = self.read_zint()?;
                    return Ok(Message::make_accept(opid, apid, lease, cid, properties));
                }

                CLOSE => {
                    let pid = if flag::has_flag(header, flag::P) {
                        Some(self.read_peerid()?)
                    } else { None };
                    let reason = self.read()?;
                    return Ok(Message::make_close(pid, reason, cid, properties));
                }

                KEEP_ALIVE => {
                    let pid = if flag::has_flag(header, flag::P) {
                        Some(self.read_peerid()?)
                    } else { None };
                    return Ok(Message::make_keep_alive(pid, reply_context, cid, properties));
                }

                DECLARE => {
                    let sn = self.read_zint()?;
                    let declarations = self.read_declarations()?;
                    return Ok(Message::make_declare(sn, declarations, cid, properties));
                }

                DATA => {
                    let reliable = flag::has_flag(header, flag::R);
                    let sn = self.read_zint()?;
                    let key = self.read_reskey(flag::has_flag(header, flag::C))?;
                    let info = if flag::has_flag(header, flag::I) {
                        Some(Arc::new(self.read_bytes_array()?))
                    } else { None };
                    let payload = Arc::new(self.read_bytes_array()?);
                    return Ok(Message::make_data(reliable, sn, key, info, payload, reply_context, cid, properties))
                }

                PULL => {
                    let is_final = flag::has_flag(header, flag::F);
                    let sn = self.read_zint()?;
                    let key = self.read_reskey(flag::has_flag(header, flag::C))?;
                    let pull_id = self.read_zint()?;
                    let max_samples = if flag::has_flag(header, flag::N) {
                        Some(self.read_zint()?)
                    } else { None };
                    return Ok(Message::make_pull(is_final, sn, key, pull_id, max_samples, cid, properties))
                }

                QUERY => {
                    let sn = self.read_zint()?;
                    let key = self.read_reskey(flag::has_flag(header, flag::C))?;
                    let predicate = self.read_string()?;
                    let qid = self.read_zint()?;
                    let target = if flag::has_flag(header, flag::T) {
                        Some(self.read_query_target()?)
                    } else { None };
                    let consolidation = self.read_consolidation()?;
                    return Ok(Message::make_query(sn, key, predicate, qid, target, consolidation, cid, properties))
                }

                PING_PONG => {
                    let hash = self.read_zint()?;
                    if flag::has_flag(header, flag::P) {
                        return Ok(Message::make_ping(hash, cid, properties))
                    } else {
                        return Ok(Message::make_pong(hash, cid, properties))
                    }
                }

                SYNC => {
                    let reliable = flag::has_flag(header, flag::R);
                    let sn = self.read_zint()?;
                    let count = if flag::has_flag(header, flag::C) {
                        Some(self.read_zint()?)
                    } else { None };
                    return Ok(Message::make_sync(reliable, sn, count, cid, properties))
                }

                ACK_NACK => {
                    let sn = self.read_zint()?;
                    let mask = if flag::has_flag(header, flag::M) {
                        Some(self.read_zint()?)
                    } else { None };
                    return Ok(Message::make_ack_nack(sn, mask, cid, properties))
                }

                id => panic!("UNEXPECTED ID FOR Message: {}", id)   //@TODO: return error
            }
        }
    }

    fn read_decl_frag(&mut self, header: u8) -> Result<MessageKind, OutOfBounds> {
        if flag::has_flag(header, flag::F) {
            if flag::has_flag(header, flag::C) {
                let n = self.read_zint()?;
                Ok(MessageKind::FirstFragment{ n:Some(n) })
            } else {
                Ok(MessageKind::FirstFragment{ n:None })
            }
        } else if flag::has_flag(header, flag::L) {
            Ok(MessageKind::LastFragment)
        } else {
            Ok(MessageKind::InbetweenFragment)
        }
    }

    fn read_decl_conduit(&mut self, header: u8) -> Result<ZInt, OutOfBounds> {
        if flag::has_flag(header, flag::Z) {
            let hl = ((flag::flags(header) ^ flag::Z)) >> 5;
            Ok(hl as ZInt)
        } else {
            let id = self.read_zint()?;
            Ok(id)
        }
    }

    fn read_decl_reply(&mut self, header: u8) -> Result<ReplyContext, OutOfBounds> {
        let is_final = flag::has_flag(header, flag::F);
        let source = if flag::has_flag(header, flag::E) { ReplySource::Eval } else { ReplySource::Storage };
        let qid = self.read_zint()?;
        let replier_id = if is_final { None } else {
            Some(self.read_peerid()?)
        };
        Ok(ReplyContext{ is_final, qid, source, replier_id })
    }

    fn read_decl_properties(&mut self, _: u8) -> Result<Vec<Property> , OutOfBounds> {
        let len = self.read_zint()?;
        let mut vec: Vec<Property> = Vec::new();
        for _ in 0..len {
            vec.push(self.read_property()?);
        }
        Ok(vec)
    }

    fn read_property(&mut self) -> Result<Property , OutOfBounds> {
        let key = self.read_zint()?;
        let value = self.read_bytes_array()?;
        Ok(Property{ key, value })
    }

    fn read_locators(&mut self) -> Result<Vec<String>, OutOfBounds> {
        let len = self.read_zint()?;
        let mut vec: Vec<String> = Vec::new();
        for _ in 0..len {
            vec.push(self.read_string()?);
        }
        Ok(vec)
    }

    fn read_declarations(&mut self) -> Result<Vec<Declaration>, OutOfBounds> {
        let len = self.read_zint()?;
        let mut vec: Vec<Declaration> = Vec::new();
        for _ in 0..len {
            vec.push(self.read_declaration()?);
        }
        Ok(vec)
    }

    fn read_declaration(&mut self) -> Result<Declaration, OutOfBounds> {
        use super::decl::{Declaration::*, id::*};

        macro_rules! read_key_delc {
            ($buf:ident, $header:ident, $type:ident) => {{
                Ok($type{ 
                    key: $buf.read_reskey(flag::has_flag($header, flag::C))?
                })
            }}
        }

        let header = self.read()?;
        match flag::mid(header) {
            RESOURCE => {
                let rid = self.read_zint()?;
                let key = self.read_reskey(flag::has_flag(header, flag::C))?;
                Ok(Resource{ rid, key })
            }

            FORGET_RESOURCE => {
                let rid = self.read_zint()?;
                Ok(ForgetResource{ rid })
            }

            SUBSCRIBER => {
                let key = self.read_reskey(flag::has_flag(header, flag::C))?;
                let mode = if flag::has_flag(header, flag::S) {
                    self.read_submode()?
                } else {
                    SubMode::Push
                };
                Ok(Subscriber{ key, mode })
            }

            FORGET_SUBSCRIBER => read_key_delc!(self, header, ForgetSubscriber),
            PUBLISHER => read_key_delc!(self, header, Publisher),
            FORGET_PUBLISHER => read_key_delc!(self, header, ForgetPublisher),
            STORAGE => read_key_delc!(self, header, Storage),
            FORGET_STORAGE => read_key_delc!(self, header, ForgetStorage),
            EVAL => read_key_delc!(self, header, Eval),
            FORGET_EVAL => read_key_delc!(self, header, ForgetEval),

            id => panic!("UNEXPECTED ID FOR Declaration: {}", id)   //@TODO: return error
        }
    }

    fn read_submode(&mut self) -> Result<SubMode, OutOfBounds> {
        use super::decl::{SubMode::*, id::*};
        match self.read_zint()? {
            MODE_PUSH => Ok(Push),
            MODE_PULL => Ok(Pull),
            MODE_PERIODIC_PUSH => {
                Ok(PeriodicPush {
                    origin:   self.read_zint()?,
                    period:   self.read_zint()?,
                    duration: self.read_zint()?
                })
            }
            MODE_PERIODIC_PULL => {
                Ok(PeriodicPull {
                    origin:   self.read_zint()?,
                    period:   self.read_zint()?,
                    duration: self.read_zint()?
                })
            }
            id => panic!("UNEXPECTED ID FOR SubMode: {}", id)   //@TODO: return error
        }
    }

    fn read_reskey(&mut self, is_numeric: bool) -> Result<ResKey, OutOfBounds> {
        let id = self.read_zint()?;
        if is_numeric {
            Ok(ResKey::ResId{ id })
        } else {
            let s = self.read_string()?;
            if id == 0 {
                Ok(ResKey::ResName{ name: s })
            } else {
                Ok(ResKey::ResGenId{ id, suffix: s })
            }
        }
    }

    fn read_query_target(&mut self) -> Result<QueryTarget, OutOfBounds> {
        let storage = self.read_target()?;
        let eval = self.read_target()?;
        Ok(QueryTarget{ storage, eval })
    }

    fn read_target(&mut self) -> Result<Target, OutOfBounds> {
        let t = self.read_zint()?;
        match t {
            0 => Ok(Target::BestMatching),
            1 => {
                let n = self.read_zint()?;
                Ok(Target::Complete{n})
            },
            2 => Ok(Target::All),
            3 => Ok(Target::None),
            id => panic!("UNEXPECTED ID FOR Target: {}", id)   //@TODO: return error
        }
    }

    fn read_consolidation(&mut self) -> Result<QueryConsolidation, OutOfBounds> {
        match self.read_zint()? {
            0 => Ok(QueryConsolidation::None),
            1 => Ok(QueryConsolidation::LastBroker),
            2 => Ok(QueryConsolidation::Incremental),
            id => panic!("UNEXPECTED ID FOR QueryConsolidation: {}", id)   //@TODO: return error
        }
    }

    fn read_timestamp(&mut self) -> Result<TimeStamp, OutOfBounds> {
        let time = self.read_zint()?;
        let bytes : [u8; 16] = self.read_slice(16)?.try_into().expect("SHOULDN'T HAPPEN");
        let id = uuid::Builder::from_bytes(bytes).build();
        Ok(TimeStamp { time, id })
    }

    fn read_peerid(&mut self) -> Result<PeerId, OutOfBounds> {
        let id = self.read_bytes_array()?;
        Ok(PeerId { id })
    }

}