extern crate stb_vorbis;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::mem;

// stb_vorbis_decode_filename: decode an entire file to interleaved shorts
fn test_decode_filename(filename: &str) -> Vec<i16> {
    // pause
    // println!("PAUSE......");
    // let mut input_text = String::new();
    // std::io::stdin()
    //     .read_line(&mut input_text)
    //     .expect("failed to read from stdin");

    println!("  stb_vorbis_decode_filename(): {}", filename);

    let mut channels = 0;
    let mut sample_rate = 0;
    let mut decoded: Vec<i16> = Vec::new();
    let path = Path::new(filename);
    let len = stb_vorbis::stb_vorbis_decode_filename(&path,
        &mut channels,
        &mut sample_rate,
        &mut decoded);

    println!("    SUCCESS, len: {} bytes, buffer length: {} elements", len, decoded.len());
    return decoded;
}

fn test_decode_memory(filename: &str) -> Vec<i16> {
    println!("  stb_vorbis_decode_memory(): {}", filename);

    //  load ogg file to memory
    let mut f = File::open(filename).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();

    let mut channels = 0;
    let mut sample_rate = 0;
    let mut decoded: Vec<i16> = Vec::new();
    let len = stb_vorbis::stb_vorbis_decode_memory(
        &buffer,
        &mut channels,
        &mut sample_rate,
        &mut decoded
    );

    println!("    SUCCESS, len: {} bytes, buffer length: {} elements",
        len,
        decoded.len());
        
    return decoded;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = &args[1];

    // let input_path = Path::new(&filename);
    let output = &args[2];
    let output = Path::new(&output);
    let mut output = File::create(output).unwrap();

    println!("{}", filename);

    let data1 = test_decode_filename(filename);
    let data2 = test_decode_memory(filename); 

    // must be same
    if data1 != data2 {
        println!("  Error: decode_filename() != decode_memory");
        std::process::exit(1);
    }

    // write to file
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            data1.as_ptr() as *const u8,
            data1.len() * mem::size_of::<i16>()
        )
    };

    output.write_all(bytes).unwrap();
    // println!("write success");

}