extern crate stb_vorbis;

// use stb_vorbis::*;

// use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
// use std::ptr;
use std::mem;



// stb_vorbis_decode_filename: decode an entire file to interleaved shorts
fn test_decode_filename(mut f: File, filename: &Path)
{
   let mut channels : i32 = 0;
   let mut sample_rate : u32 = 0;
   let decoded = stb_vorbis::stb_vorbis_decode_filename(filename, &mut channels, &mut sample_rate).unwrap();
   
   let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            decoded.as_ptr() as *const u8, 
            decoded.len() * mem::size_of::<u16>()
        )
    };
   
   f.write_all(bytes).unwrap();
}


fn main(){
    let output_path = Path::new("output.bin");
    let output = File::create(output_path).unwrap();
    
    let mut args = std::env::args();
    let filename = args.nth(1).unwrap();
    let input_path = Path::new(&filename);
    
    println!("decode filename {}", filename);
    test_decode_filename(output, input_path);
}