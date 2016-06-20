extern crate stb_vorbis;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::mem;
use std::process;

use stb_vorbis::{
    stb_vorbis_get_info, stb_vorbis_get_error, 
    stb_vorbis_open_filename, stb_vorbis_seek,
    stb_vorbis_get_samples_float_interleaved, stb_vorbis_get_samples_float
};
use stb_vorbis::{Vorbis, VorbisError, AudioBufferSlice};


fn show_info(v: &mut Vorbis) {
    let info = stb_vorbis_get_info(v);
    println!("{} channels, {} samples/sec", info.channels, info.sample_rate);
}

fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        return min;
    } else if value > max {
        return max;
    }
    return value;
}

fn write_floats_interleaved(out_file: &mut File, buffer: &[f32]) {
    const SCALE: f32 = 32768.0;
    for j in buffer {
        let x: i16 = clamp((SCALE * j) as i32, -32768, 32767) as i16;
        let x: [u8; 2] = unsafe { mem::transmute(x) };
        out_file.write(&x).unwrap();
    }
}

fn test_get_samples_float_interleaved(v: &mut Vorbis) -> Vec<f32> {
    // pause
    // {
    //     println!("PAUSE......");
    //     let mut input_text = String::new();
    //     std::io::stdin()
    //         .read_line(&mut input_text)
    //         .expect("failed to read from stdin");
    // }
    let mut result = Vec::with_capacity(4096);
    loop {
        let mut sbuffer: [f32; 333] = [0.0; 333];
        let n = stb_vorbis_get_samples_float_interleaved(v, 2, &mut sbuffer);
        if n == 0 {
            break;
        }

        result.extend_from_slice(&sbuffer[.. n as usize]);
    }
    return result;
}

fn test_get_samples_float(v: &mut Vorbis) -> Vec<Vec<f32>> {
    let mut result : Vec<Vec<f32>> = vec![Vec::with_capacity(10000), Vec::with_capacity(10000)];
    let mut sbuffer: Vec<Vec<f32>> = vec![Vec::new(), Vec::new()];
    sbuffer[0].resize(333, 0.0);
    sbuffer[1].resize(333, 0.0);
    loop {
        let mut audio_slice = unsafe {
            AudioBufferSlice::from(&mut sbuffer)
        };
        let n = stb_vorbis_get_samples_float(v, 2, &mut audio_slice);
        if n == 0 {
            break;
        }

        for i in 0 .. 2 {
            result[i].extend_from_slice(&sbuffer[i][.. n as usize]);
        }

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

    println!("test_get_samples_float_interleaved(): {}", filename.display());
    let floats_interleaved = test_get_samples_float_interleaved(&mut v);

    // test stb_vorbis_get_error
    let error = stb_vorbis_get_error(&mut v);
    if error != VorbisError::NoError {
        println!("Error: {:?}'", error);
        process::exit(error as i32);
    }

    println!("test_get_samples_float(): {}", filename.display());
    stb_vorbis_seek(&mut v, 0);
    let floats = test_get_samples_float(&mut v);
    use std::ops::Add;
    let size : usize = floats.iter().map(|x| x.len() ).fold(0, Add::add);

    // must have same size!
    assert_eq!(size, floats_interleaved.len() * 2);

    let output = &args[2];
    let output = Path::new(&output);
    let mut output = File::create(output).unwrap();
    write_floats_interleaved(&mut output, &floats_interleaved);
}