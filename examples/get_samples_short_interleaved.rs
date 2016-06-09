extern crate stb_vorbis;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
// use std::ptr;
use std::mem;
// use std::process;

use stb_vorbis::{stb_vorbis_get_info, stb_vorbis_open_filename, stb_vorbis_get_samples_short_interleaved};
use stb_vorbis::{Vorbis/*, VorbisError*/};


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


fn test_get_samples_short_interleaved(mut out_file: File, filename: &str) {
    let filename = Path::new(filename);
    let v = stb_vorbis_open_filename(&filename, None);
    let mut v = match v {
        Err(why) => {
            println!("Couldn't open {}. Error: {:?}'", filename.display(), why);
            return;
        },
        Ok(v) => v,
    };

    show_info(&mut v);

    // pause
    // {
    //     println!("PAUSE......");
    //     let mut input_text = String::new();
    //     std::io::stdin()
    //         .read_line(&mut input_text)
    //         .expect("failed to read from stdin");
    // }

    loop {
        let mut sbuffer: [i16; 333] = [0; 333];
        let n = stb_vorbis_get_samples_short_interleaved(&mut v, 2, &mut sbuffer, 333);
        if n == 0 {
            break;
        }

        // write to file
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(sbuffer.as_ptr() as *const u8,
                                       (n*2) as usize * mem::size_of::<i16>())
        };

        out_file.write_all(bytes).unwrap();
    }
}




fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = &args[1];

    let output = &args[2];
    let output = Path::new(&output);
    let output = File::create(output).unwrap();

    println!("get samples short interleaved {}", filename);
    test_get_samples_short_interleaved(output, filename);
}