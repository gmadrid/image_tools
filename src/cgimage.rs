#![allow(improper_ctypes, non_camel_case_types, non_upper_case_globals)]

use cfmutabledata::CFMutableData;
use cgimagedestination::CGImageDestination;
use core_foundation::base::{CFRelease, CFTypeID, TCFType};
use core_foundation::data::{CFData, CFDataRef};
use core_graphics::color_space::{CGColorSpace, CGColorSpaceRef};
use core_graphics::context::{CGContext, CGContextRef};
use core_graphics::data_provider::{CGDataProvider, CGDataProviderRef};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use errs::{Error, Result};
use libc::{c_void, size_t};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::ptr;

types_CFType!(CGImage, CGImageRef, __CGImage);
impl_TCFType!(CGImage, CGImageRef, CGImageGetTypeID);

pub type CGColorRenderingIntent = i32;
pub const kCGRenderingIntentDefault: CGColorRenderingIntent = 0;
pub const kCGRenderingIntentAbsoluteColorimetric: CGColorRenderingIntent = 1;
pub const kCGRenderingIntentRelativeColorimetric: CGColorRenderingIntent = 2;
pub const kCGRenderingIntentPerceptual: CGColorRenderingIntent = 3;
pub const kCGRenderingIntentSaturation: CGColorRenderingIntent = 4;

pub type CGImageAlphaInfo = u32;
pub const kCGImageAlphaNoneSkipFirst: CGImageAlphaInfo = 6;

impl CGImage {
  pub fn read_jpg<T>(mut reader: T) -> Result<CGImage>
    where T: Read {
    unsafe {
      let mut bytes: Vec<u8> = Vec::new();
      try!(reader.read_to_end(&mut bytes));

      let data_provider = CGDataProvider::from_buffer(&bytes);
      // TODO: Is this the right value for shouldInterpolate?
      let result = CGImageCreateWithJPEGDataProvider(data_provider.as_concrete_TypeRef(),
                                                     ptr::null(),
                                                     0,
                                                     kCGRenderingIntentDefault);
      if result != ptr::null() {
        Ok(TCFType::wrap_under_create_rule(result))
      } else {
        Err(Error::FailedToLoadAsJPEG)
      }
    }
  }

  pub fn read_png<T>(mut reader: T) -> Result<CGImage>
    where T: Read {
    unsafe {
      let mut bytes: Vec<u8> = Vec::new();
      try!(reader.read_to_end(&mut bytes));

      let data_provider = CGDataProvider::from_buffer(&bytes);
      // TODO: Is this the right value for shouldInterpolate?
      let result = CGImageCreateWithPNGDataProvider(data_provider.as_concrete_TypeRef(),
                                                    ptr::null(),
                                                    0,
                                                    kCGRenderingIntentDefault);
      if result != ptr::null() {
        Ok(TCFType::wrap_under_create_rule(result))
      } else {
        Err(Error::FailedToLoadAsPNG)
      }
    }
  }

  fn jpeg_data(&self) -> Result<Vec<u8>> {
    let data = try!(CFMutableData::new(0));
    let dest = CGImageDestination::jpg_destination_with_data(&data);
    dest.add_image(self);
    try!(dest.finalize());
    let mut vec = Vec::new();
    vec.extend_from_slice(data.bytes());
    Ok(vec)
  }

  // TODO: write write_png()

  pub fn write_jpeg<T>(&self, mut w: T) -> Result<()>
    where T: Write {
    let bytes = try!(self.jpeg_data());
    try!(w.write_all(&bytes));
    Ok(())
  }

  pub fn save_jpeg_to_file(&self, path: &Path) -> Result<()> {
    let out_file = try!(File::create(path));
    try!(self.write_jpeg(out_file));
    Ok(())
  }

  pub fn height(&self) -> size_t {
    unsafe { CGImageGetHeight(self.as_concrete_TypeRef()) }
  }

  pub fn width(&self) -> size_t {
    unsafe { CGImageGetWidth(self.as_concrete_TypeRef()) }
  }

  fn color_space(&self) -> CGColorSpace {
    unsafe {
      let cs = CGImageGetColorSpace(self.as_concrete_TypeRef());
      TCFType::wrap_under_get_rule(cs)
    }
  }

  fn draw_into_context(&self, context: &CGContext) {
    // TODO: This is just here until context.draw_image() is implemented in core_graphics.
    unsafe {
      let height = CGBitmapContextGetHeight(context.as_concrete_TypeRef());
      let width = CGBitmapContextGetWidth(context.as_concrete_TypeRef());
      CGContextDrawImage(context.as_concrete_TypeRef(),
                         CGRect::new(&CGPoint::new(0.0, 0.0),
                                     &CGSize::new(width as f64, height as f64)),
                         self.as_concrete_TypeRef())
    }
  }

  pub fn convert_to_gray(&self) -> Result<CGImage> {
    let grayscale_space = create_device_gray_color_space();
    // bytes_per_row is width * 8bit_per_component * 1component_per_pixel / 8bits_per_byte
    let context = CGContext::create_bitmap_context(self.width(),
                                                   self.height(),
                                                   8,
                                                   self.width(),
                                                   &grayscale_space,
                                                   0);
    self.draw_into_context(&context);
    CGImage::image_from_bitmap_context(&context)
  }

  pub fn shrink(&self, width: usize, height: usize) -> Result<CGImage> {
    let color_space = self.color_space();
    let num_components = number_of_components_for_color_space(&color_space);
    if num_components != 1 {
      // Figure out how to deal with alpha channels and 3 or 4 components.
      // Right now, only shrink grayscale.
      unimplemented!()
    }
    let bytes_per_row = width;

    // TODO: 0 for the bitmap_info will barf with a non-gray color space. Fix this.
    let context =
      CGContext::create_bitmap_context(width, height, 8, bytes_per_row, &color_space, 0);
    self.draw_into_context(&context);
    CGImage::image_from_bitmap_context(&context)
  }

  fn image_from_bitmap_context(context: &CGContext) -> Result<CGImage> {
    unsafe {
      let result = CGBitmapContextCreateImage(context.as_concrete_TypeRef());
      if result != ptr::null() {
        Ok(TCFType::wrap_under_create_rule(result))
      } else {
        Err(Error::BitmapConversionFailed)
      }
    }
  }

  pub fn ahash(&self) -> Result<u64> {
    // Make it grayscale, then shrink it to 8x8, then get the data.
    let data = try!(self.convert_to_gray().and_then(|i| i.shrink(8, 8)).map(|i| i.raw_data()));
    // The size should be 64 (8x8), but in some situations, CoreGraphics may allocate more memory
    // than the image strictly needs. We don't handle this case, so validate it here.
    if 64 != data.len() {
      panic!("Data len was not 64, it was {:?}.", data.len());
    }

    // Compute the ahash.
    let avg = data.iter().map(|d| *d as f32).sum::<f32>() / (data.len() as f32);
    let mut hsh = 0u64;
    for (idx, val) in data.iter().enumerate() {
      if *val as f32 > avg {
        hsh = hsh | 1 << idx;
      }
    }
    Ok(hsh)
  }

  pub fn dhash(&self) -> Result<u64> {
    // Make it grayscale, then shrink to 9x8, then get the data.
    let data = try!(self.convert_to_gray().and_then(|i| i.shrink(9, 8)).map(|i| i.raw_data()));
    // The size should be 72 (9x8), but in some situations, CoreGraphics may allocate more memory
    // than the image strictly needs. We don't handle this case, so validate it here.
    if 72 != data.len() {
      panic!("Data len was not 64, it was {:?}.", data.len());
    }
    let mut hsh = 0u64;
    let mut idx = 0u16;
    for row in 0..8 {
      for col in 0..8 {
        let v1 = data.get(row * 9 + col).unwrap();
        let v2 = data.get(row * 9 + col + 1).unwrap();
        //        println!("{} < {}", v1, v2);
        if v1 > v2 {
          hsh = hsh | 1 << idx;
        }
        idx += 1;
      }
    }
    Ok(hsh)
  }

  fn raw_data(&self) -> Vec<u8> {
    unsafe {
      let provider: CGDataProvider =
        TCFType::wrap_under_get_rule(CGImageGetDataProvider(self.as_concrete_TypeRef()));
      let data: CFData =
        TCFType::wrap_under_create_rule(CGDataProviderCopyData(provider.as_concrete_TypeRef()));
      let bytes = data.bytes();
      let mut vec = Vec::new();
      vec.extend_from_slice(bytes);
      vec
    }
  }
}

fn create_device_gray_color_space() -> CGColorSpace {
  unsafe {
    let color_space = CGColorSpaceCreateDeviceGray();
    TCFType::wrap_under_create_rule(color_space)
  }
}

fn number_of_components_for_color_space(space: &CGColorSpace) -> usize {
  unsafe { CGColorSpaceGetNumberOfComponents(space.as_concrete_TypeRef()) as usize }
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
  fn CGBitmapContextCreateImage(context: CGContextRef) -> CGImageRef;
  fn CGBitmapContextGetHeight(context: CGContextRef) -> size_t;
  fn CGBitmapContextGetWidth(context: CGContextRef) -> size_t;
  fn CGColorSpaceCreateDeviceGray() -> CGColorSpaceRef;
  fn CGColorSpaceGetNumberOfComponents(color_space: CGColorSpaceRef) -> size_t;
  fn CGContextDrawImage(context: CGContextRef, rect: CGRect, image: CGImageRef);
  fn CGDataProviderCopyData(provider: CGDataProviderRef) -> CFDataRef;
  fn CGImageCreateWithJPEGDataProvider(dataProvider: CGDataProviderRef,
                                       decode: *const c_void,
                                       shouldInterpolate: u8,
                                       intent: CGColorRenderingIntent)
      -> CGImageRef;
  fn CGImageCreateWithPNGDataProvider(dataProvider: CGDataProviderRef,
                                      decode: *const c_void,
                                      shouldInterpolate: u8,
                                      intent: CGColorRenderingIntent)
      -> CGImageRef;
  fn CGImageGetColorSpace(image: CGImageRef) -> CGColorSpaceRef;
  fn CGImageGetDataProvider(image: CGImageRef) -> CGDataProviderRef;
  fn CGImageGetHeight(image: CGImageRef) -> size_t;
  fn CGImageGetTypeID() -> CFTypeID;
  fn CGImageGetWidth(image: CGImageRef) -> size_t;
}
