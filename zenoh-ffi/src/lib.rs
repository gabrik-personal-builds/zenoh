use libc::{c_char, c_ulong};
use std::ffi::CStr;
use async_std::task;
use zenoh::net;

use zenoh_protocol::core::{ResKey, ResourceId}; // { rname, PeerId, ResourceId, , ZError, ZErrorKind };

pub struct ZNSession(zenoh::net::Session);

pub struct ZProperties(zenoh::net::Properties);

#[no_mangle]
pub extern "C" fn zn_properties_make() -> *mut ZProperties {
  Box::into_raw(Box::new(ZProperties(zenoh::net::Properties::new())))
}

#[no_mangle]
pub unsafe extern "C" fn zn_properties_add(rps: *mut ZProperties, id: c_ulong, value: *const c_char) -> *mut ZProperties {
  let mut ps = Box::from_raw(rps);  
  let bs = CStr::from_ptr(value).to_bytes();
  ps.0.insert(id as zenoh::net::ZInt, Vec::from(bs));
  rps
}

#[no_mangle]
pub unsafe extern "C" fn zn_properties_free(rps: *mut ZProperties ) {
  let ps = Box::from_raw(rps);  
  drop(ps);
}

#[no_mangle]
pub unsafe extern "C" fn zn_open(locator: *const c_char, _ps: *const ZProperties) -> *mut ZNSession {
  let l = 
  if locator.is_null() { "" } 
  else {
    CStr::from_ptr(locator).to_str().unwrap()
  };
  let s = task::block_on(net::open(l, None)).unwrap();
  Box::into_raw(Box::new(ZNSession(s)))
}

#[no_mangle]
pub unsafe extern "C" fn zn_close(session: *mut ZNSession) {  
  let s = Box::from_raw(session);
  task::block_on(s.0.close()).unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn zn_declare_resource(session: *mut ZNSession, r_name: *const c_char) -> c_ulong {
  if r_name.is_null()  { return 0 };
  let s = Box::from_raw(session);
  let name = CStr::from_ptr(r_name).to_str().unwrap();
  task::block_on(s.0.declare_resource(&ResKey::RName(name.to_string()))).unwrap() as c_ulong
}

#[no_mangle]
pub unsafe extern "C" fn zn_declare_resource_ws(session: *mut ZNSession, rid: c_ulong, suffix: *const c_char) -> c_ulong {
  if suffix.is_null()  { return 0 };
  let s = Box::from_raw(session);
  let sfx = CStr::from_ptr(suffix).to_str().unwrap();
  task::block_on(s.0.declare_resource(&ResKey::RIdWithSuffix(rid as ResourceId, sfx.to_string()))).unwrap() as c_ulong
}