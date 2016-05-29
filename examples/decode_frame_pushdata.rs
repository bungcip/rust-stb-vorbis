extern crate stb_vorbis;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::ptr;
use std::mem;
use std::process;

use stb_vorbis::{stb_vorbis_get_info, stb_vorbis_open_pushdata, stb_vorbis_decode_frame_pushdata};
use stb_vorbis::{Vorbis, VorbisError};


fn show_info(v: &mut Vorbis) {
    let info = stb_vorbis_get_info(v);
    println!("{} channels, {} samples/sec",
             info.channels,
             info.sample_rate);
    println!("Predicted memory needed: {} ({} + {})",
             info.setup_memory_required + info.temp_memory_required,
             info.setup_memory_required,
             info.temp_memory_required);
}


fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        return min;
    } else if value > max {
        return max;
    }
    return value;
}

unsafe fn write_floats(out_file: &mut File, len: i32, left: *mut f32, right: *mut f32) {

    const SCALE: f32 = 32768.0;
    for j in 0..len {
        let x: i16 = clamp((SCALE * *left.offset(j as isize)) as i32, -32768, 32767) as i16;
        let y: i16 = clamp((SCALE * *right.offset(j as isize)) as i32, -32768, 32767) as i16;

        let x: [u8; 2] = mem::transmute(x);
        let y: [u8; 2] = mem::transmute(y);;
        out_file.write(&x).unwrap();
        out_file.write(&y).unwrap();
    }
}


// stb_vorbis_decode_frame_pushdata: decode an entire file using push mode
unsafe fn test_decode_frame_pushdata(mut out_file: File, filename: &str) {
    //  load ogg file to memory
    let mut f = File::open(filename).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    let len: i32 = buffer.len() as i32;

    // pause
    // {
    //     println!("PAUSE......");
    //     let mut input_text = String::new();
    //     io::stdin()
    //         .read_line(&mut input_text)
    //         .expect("failed to read from stdin");
    // }

    println!("run stb_vorbis_open_pushdata()");
    let data = buffer.as_mut_ptr();
    let mut used = 0;
    let mut p = 0;
    let mut q = 1;
    let mut v;
    let mut error = VorbisError::NoError;
    'retry: loop {
        v = stb_vorbis_open_pushdata(data, q, &mut used, &mut error, None);
        if v.is_none() {
            if error == VorbisError::NeedMoreData {
                q += 1;
                continue; //goto retry;
            }
            println!("Error {:?}", error);
            process::exit(1);
        }

        break;
    }
    p += used;


    let mut v = v.unwrap();
    show_info(&mut v);

    'forever: loop {
        let mut n = 0;
        let left: *mut f32;
        let right: *mut f32;

        let mut outputs: *mut *mut f32 = ptr::null_mut();
        let mut num_c: i32 = 0;
        q = 32;

        'retry3: loop {
            if q > len - p {
                q = len - p;
            }
            used = stb_vorbis_decode_frame_pushdata(&mut v,
                                                    data.offset(p as isize),
                                                    q,
                                                    &mut num_c,
                                                    &mut outputs,
                                                    &mut n);
            if used == 0 {
                if p + q == len {
                    break 'forever; // no more data, stop
                }
                if q < 128 {
                    q = 128;
                }
                q *= 2;
                continue; //  goto retry3;
            }
            break;
        }
        p += used;
        if n == 0 {
            continue;
        } // seek/error recovery
        left = *outputs.offset(0);
        right = if num_c > 1 {
            *outputs.offset(1)
        } else {
            *outputs.offset(0)
        };
        write_floats(&mut out_file, n, left, right);
    }
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = &args[1];

    let output = &args[2];
    let output = Path::new(&output);
    let output = File::create(output).unwrap();

    println!("decode frame pushdata {}", filename);
    unsafe {
        test_decode_frame_pushdata(output, filename);
        // match test_decode_frame_pushdata(output, filename){
        //     Err(why) => println!("Error: {:?}", why),
        //     Ok(_)    => println!("Sukses")
        // }
    }
}