#![feature(alloc_system)]

extern crate alloc_system;
extern crate stb_vorbis;

// use stb_vorbis::*;

// use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::ptr;
use std::mem;
use std::ffi::CString;


// stb_vorbis_decode_filename: decode an entire file to interleaved shorts
fn test_decode_filename(mut f: File, filename: &str)
{
   let mut channels : i32 = 0;
   let mut sample_rate : i32 = 0;
   let mut decoded: *mut i16 = ptr::null_mut();
   
   let filename = CString::new(filename).unwrap();
   let len = unsafe { stb_vorbis::stb_vorbis_decode_filename(
       filename.as_ptr(), 
       &mut channels as *mut i32, 
       &mut sample_rate as *mut i32, 
       &mut decoded as *mut *mut i16) };
   
   
   println!("decode success, len: {} bytes", len);
   
   let d = decoded;
   let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            d as *const u8, 
            len as usize * channels as usize * mem::size_of::<u16>()
        )
    };
   
   f.write_all(bytes).unwrap();
   println!("write success");
}


fn main(){
    let args : Vec<String> = std::env::args().collect();
    let filename = &args[1];

    // let input_path = Path::new(&filename);
    let output = &args[2];
    let output = Path::new(&output);
    let output = File::create(output).unwrap();
    
    println!("decode filename {}", filename);
    test_decode_filename(output, filename);
}