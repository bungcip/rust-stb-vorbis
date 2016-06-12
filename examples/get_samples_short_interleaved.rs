extern crate stb_vorbis;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
// use std::ptr;
use std::mem;
use std::process;


use stb_vorbis::{
    stb_vorbis_get_info, stb_vorbis_get_error, 
    stb_vorbis_open_filename, stb_vorbis_seek,
    stb_vorbis_get_samples_short_interleaved, stb_vorbis_get_samples_short
};
use stb_vorbis::{Vorbis, VorbisError, AudioBufferSlice};


fn show_info(v: &mut Vorbis) {
    let info = stb_vorbis_get_info(v);
    println!("{} channels, {} samples/sec", info.channels, info.sample_rate);
}

fn test_get_samples_short_interleaved(v: &mut Vorbis) -> Vec<u8> {
    // pause
    // {
    //     println!("PAUSE......");
    //     let mut input_text = String::new();
    //     std::io::stdin()
    //         .read_line(&mut input_text)
    //         .expect("failed to read from stdin");
    // }
    let mut result : Vec<u8> = Vec::with_capacity(4096);
    loop {
        let mut sbuffer: [i16; 333] = [0; 333];
        let n = stb_vorbis_get_samples_short_interleaved(v, 2, &mut sbuffer);
        if n == 0 {
            break;
        }

        // save it result
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(sbuffer.as_ptr() as *const u8,
                                       (n*2) as usize * mem::size_of::<i16>())
        };
        result.extend_from_slice(&bytes);
    }
    return result;
}

fn test_get_samples_short(v: &mut Vorbis) -> Vec<Vec<i16>> {
    let mut result : Vec<Vec<i16>> = vec![Vec::new(), Vec::new()];
    let mut sbuffer: Vec<Vec<i16>> = vec![Vec::new(), Vec::new()];
    sbuffer[0].resize(333, 0);
    sbuffer[1].resize(333, 0);
    loop {
        let mut audio_slice = unsafe {
            AudioBufferSlice::from(&mut sbuffer)
        };
        let n = unsafe {
            stb_vorbis_get_samples_short(v, 2, &mut audio_slice)
        };
        if n == 0 {
            break;
        }

        for i in 0 .. 2 {
            result[i].extend_from_slice(&sbuffer[i][.. n as usize]);
        }

        // save it result
        // let bytes: &[u8] = unsafe {
        //     std::slice::from_raw_parts(sbuffer.as_ptr() as *const u8,
        //                                (n*2) as usize * mem::size_of::<i16>())
        // };
        // result.extend_from_slice(&bytes);
    }
    return result;
    
}




fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = &args[1];

    let filename = Path::new(filename);
    let v = stb_vorbis_open_filename(&filename);
    let mut v = match v {
        Err(why) => {
            println!("Couldn't open {}. Error: {:?}'", filename.display(), why);
            process::exit(why as i32);
        },
        Ok(v) => v,
    };

    println!("test_stb_vorbis_get_info()");
    show_info(&mut v);

    println!("test_get_samples_short_interleaved(). {}", filename.display());
    let bytes = test_get_samples_short_interleaved(&mut v);

    // test stb_vorbis_get_error
    let error = stb_vorbis_get_error(&mut v);
    if error != VorbisError::NoError {
        println!("Error: {:?}'", error);
        process::exit(error as i32);
    }

    println!("test_get_samples_short(). {}", filename.display());
    stb_vorbis_seek(&mut v, 0);
    let shorts = test_get_samples_short(&mut v);
    use std::ops::Add;
    let size : usize = shorts.iter().map(|x| x.len() ).fold(0, Add::add);

    // must have same size!
    assert_eq!(size * 2, bytes.len());

    let output = &args[2];
    let output = Path::new(&output);
    let mut output = File::create(output).unwrap();
    output.write_all(&bytes).unwrap();

}