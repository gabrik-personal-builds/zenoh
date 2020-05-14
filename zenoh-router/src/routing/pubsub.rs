use std::sync::Arc;
use std::collections::HashMap;
use zenoh_protocol::core::ResKey;
use zenoh_protocol::io::RBuf;
use zenoh_protocol::proto::{WhatAmI, SubInfo};
use crate::routing::face::Face;
use crate::routing::tables::Tables;
use crate::routing::resource::{Resource, Context};

pub type DataRoute = HashMap<usize, (Arc<Face>, u64, String)>;

pub async fn declare_subscription(tables: &mut Tables, face: &mut Arc<Face>, prefixid: u64, suffix: &str, sub_info: &SubInfo) {
    match tables.get_mapping(&face, &prefixid).cloned() {
        Some(mut prefix) => unsafe {
            let mut res = Resource::make_resource(&mut prefix, suffix);
            Resource::match_resource(&tables.root_res, &mut res);
            {
                let res = Arc::get_mut_unchecked(&mut res);
                match res.contexts.get_mut(&face.id) {
                    Some(mut ctx) => {
                        Arc::get_mut_unchecked(&mut ctx).subs = Some(sub_info.clone());
                    }
                    None => {
                        res.contexts.insert(face.id, 
                            Arc::new(Context {
                                face: face.clone(),
                                local_rid: None,
                                remote_rid: None,
                                subs: Some(sub_info.clone()),
                                qabl: false,
                            })
                        );
                    }
                }
            }

            for (id, someface) in &mut tables.faces {
                if face.id != *id && (face.whatami != WhatAmI::Peer || someface.whatami != WhatAmI::Peer) {
                    let (nonwild_prefix, wildsuffix) = Resource::nonwild_prefix(&res);
                    match nonwild_prefix {
                        Some(mut nonwild_prefix) => {
                            if let Some(mut ctx) = Arc::get_mut_unchecked(&mut nonwild_prefix).contexts.get_mut(id) {
                                if let Some(rid) = ctx.local_rid {
                                    someface.primitives.clone().subscriber((rid, wildsuffix).into(), sub_info.clone()).await;
                                } else if let Some(rid) = ctx.remote_rid {
                                    someface.primitives.clone().subscriber((rid, wildsuffix).into(), sub_info.clone()).await;
                                } else {
                                    let rid = someface.get_next_local_id();
                                    Arc::get_mut_unchecked(&mut ctx).local_rid = Some(rid);
                                    Arc::get_mut_unchecked(someface).local_mappings.insert(rid, nonwild_prefix.clone());

                                    someface.primitives.clone().resource(rid, nonwild_prefix.name().into()).await;
                                    someface.primitives.clone().subscriber((rid, wildsuffix).into(), sub_info.clone()).await;
                                }
                            } else {
                                let rid = someface.get_next_local_id();
                                Arc::get_mut_unchecked(&mut nonwild_prefix).contexts.insert(*id, 
                                    Arc::new(Context {
                                        face: someface.clone(),
                                        local_rid: Some(rid),
                                        remote_rid: None,
                                        subs: None,
                                        qabl: false,
                                }));
                                Arc::get_mut_unchecked(someface).local_mappings.insert(rid, nonwild_prefix.clone());

                                someface.primitives.clone().resource(rid, nonwild_prefix.name().into()).await;
                                someface.primitives.clone().subscriber((rid, wildsuffix).into(), sub_info.clone()).await;
                            }
                        }
                        None => {
                            someface.primitives.clone().subscriber(ResKey::RName(wildsuffix), sub_info.clone()).await;
                        }
                    }
                }
            }
            Tables::build_matches_direct_tables(&mut res);
            Arc::get_mut_unchecked(face).subs.push(res);
        }
        None => println!("Declare subscription for unknown rid {}!", prefixid)
    }
}

pub async fn undeclare_subscription(tables: &mut Tables, face: &mut Arc<Face>, prefixid: u64, suffix: &str) {
    match tables.get_mapping(&face, &prefixid) {
        Some(prefix) => {
            match Resource::get_resource(prefix, suffix) {
                Some(mut res) => unsafe {
                    if let Some(mut ctx) = Arc::get_mut_unchecked(&mut res).contexts.get_mut(&face.id) {
                        Arc::get_mut_unchecked(&mut ctx).subs = None;
                    }
                    Arc::get_mut_unchecked(face).subs.retain(|x| ! Arc::ptr_eq(&x, &res));
                    Resource::clean(&mut res)
                }
                None => println!("Undeclare unknown subscription!")
            }
        }
        None => println!("Undeclare subscription with unknown prefix!")
    }
}

pub async fn route_data_to_map(tables: &Tables, face: &Arc<Face>, rid: u64, suffix: &str) -> Option<DataRoute> {
    match tables.get_mapping(&face, &rid) {
        Some(prefix) => {
            match Resource::get_resource(prefix, suffix) {
                Some(res) => {Some(res.route.clone())}
                None => {
                    let mut faces = HashMap::new();
                    for res in Resource::get_matches_from(&[&prefix.name(), suffix].concat(), &tables.root_res) {
                        let res = res.upgrade().unwrap();
                        for (sid, context) in &res.contexts {
                            if context.subs.is_some() {
                                faces.entry(*sid).or_insert_with( || {
                                    let (rid, suffix) = Resource::get_best_key(prefix, suffix, *sid);
                                    (context.face.clone(), rid, suffix)
                                });
                            }
                        }
                    };
                    Some(faces)
                }
            }
        }
        None => {
            println!("Route data with unknown rid {}!", rid); None
        }

    }
}

pub async fn route_data(tables: &Tables, face: &Arc<Face>, rid: u64, suffix: &str, reliable:bool, info: &Option<RBuf>, payload: RBuf) {
    if let Some(outfaces) = route_data_to_map(tables, face, rid, suffix).await {
        for (_id, (outface, rid, suffix)) in outfaces {
            if ! Arc::ptr_eq(face, &outface) {
                let primitives = {
                    if face.whatami != WhatAmI::Peer || outface.whatami != WhatAmI::Peer {
                        Some(outface.primitives.clone())
                    } else {
                        None
                    }
                };
                if let Some(primitives) = primitives {
                    primitives.data((rid, suffix).into(), reliable, info.clone(), payload.clone()).await
                }
            }
        }
    }
}