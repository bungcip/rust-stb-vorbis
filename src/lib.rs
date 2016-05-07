// temporary disable lint for now...
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(non_upper_case_globals)]

#![feature(question_mark, custom_derive, box_syntax)]
//  float_extras

extern crate libc;

use libc::*;

// Converted function is here

#[no_mangle]
pub extern fn square(x: f32) -> f32{
    x * x
}



// Below is function that still live in C code
extern {
    pub fn stb_vorbis_decode_filename(
        filename: *const i8, 
        channels: *mut c_int, 
        sample_rate: *mut c_int, 
        output: *mut *mut i16) -> c_int;
}