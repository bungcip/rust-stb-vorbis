extern crate stb_vorbis;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::mem;

// stb_vorbis_decode_filename: decode an entire file to interleaved shorts
fn test_decode_filename(mut f: File, filename: &str)
{
   let mut channels = 0;
   let mut sample_rate = 0;
   
    // pause
    // println!("PAUSE......");
    // let mut input_text = String::new();
    // std::io::stdin()
    //     .read_line(&mut input_text)
    //     .expect("failed to read from stdin");

   let mut decoded : Vec<i16> = Vec::new();
   let path = Path::new(filename);
   let len = stb_vorbis::stb_vorbis_decode_filename(
       &path, 
       &mut channels, 
       &mut sample_rate, 
       &mut decoded
    );
   
   println!("decode success, len: {} bytes, buffer length: {} elements", len, decoded.len());
   
   let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            decoded.as_ptr() as *const u8, 
            decoded.len() * mem::size_of::<i16>()
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