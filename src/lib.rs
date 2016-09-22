#[macro_use]
extern crate core_foundation;
extern crate core_foundation_sys;
extern crate core_graphics;
extern crate libc;

macro_rules! types_CFType {
  ($ty:ident, $ref_id:ident, $cid:ident) => {
    #[repr(C)]
    pub struct $cid;
    pub type $ref_id = *const $cid;
    pub struct $ty($ref_id);
    impl Drop for $ty {
      fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef())}
      }
    }
  }
}

pub mod cgimage;
mod cgimagedestination;
mod cfmutabledata;
pub mod errs;
