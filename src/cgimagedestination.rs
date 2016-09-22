#![allow(improper_ctypes, non_camel_case_types, non_upper_case_globals)]

use cfmutabledata::{CFMutableData, CFMutableDataRef};
use cgimage::{CGImage, CGImageRef};
use core_foundation::base::{CFTypeID, TCFType};
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::string::CFStringRef;
use core_foundation_sys::base::CFRelease;
use errs::{Error, Result};
use libc::size_t;
use std::ptr;

types_CFType!(CGImageDestination,
              CGImageDestinationRef,
              __CGImageDestination);
impl_TCFType!(CGImageDestination,
              CGImageDestinationRef,
              CGImageDestinationGetTypeID);

impl CGImageDestination {
  pub fn jpg_destination_with_data(data: &CFMutableData) -> CGImageDestination {
    unsafe {
      let result =
        CGImageDestinationCreateWithData(data.as_concrete_TypeRef(), kUTTypeJPEG, 1, ptr::null());
      TCFType::wrap_under_create_rule(result)
    }
  }

  pub fn add_image(&self, img: &CGImage) {
    unsafe {
      CGImageDestinationAddImage(self.as_concrete_TypeRef(),
                                 img.as_concrete_TypeRef(),
                                 ptr::null())
    }
  }

  pub fn finalize(&self) -> Result<()> {
    unsafe {
      if CGImageDestinationFinalize(self.as_concrete_TypeRef()) {
        Ok(())
      } else {
        Err(Error::CannotFinalizeImageDestination)
      }
    }
  }
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
  static kUTTypeJPEG: CFStringRef;
  fn CGImageDestinationAddImage(dest: CGImageDestinationRef,
                                image: CGImageRef,
                                properties: CFDictionaryRef);
  fn CGImageDestinationCreateWithData(data: CFMutableDataRef,
                                      typ: CFStringRef,
                                      count: size_t,
                                      options: CFDictionaryRef)
      -> CGImageDestinationRef;
  fn CGImageDestinationGetTypeID() -> CFTypeID;
  fn CGImageDestinationFinalize(dest: CGImageDestinationRef) -> bool;
}
