// temporary disable lint for now...
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
// #![allow(unused_variables)]
// #![allow(unused_imports)]
// #![allow(unused_parens)]
#![allow(non_upper_case_globals)]

#![feature(question_mark, custom_derive, box_syntax, float_extras)]
#![feature(alloc_system)]

extern crate alloc_system;

extern crate libc;

use libc::*;


// STB_VORBIS_MAX_CHANNELS [number]
//     globally define this to the maximum number of channels you need.
//     The spec does not put a restriction on channels except that
//     the count is stored in a byte, so 255 is the hard limit.
//     Reducing this saves about 16 bytes per value, so using 16 saves
//     (255-16)*16 or around 4KB. Plus anything other memory usage
//     I forgot to account for. Can probably go as low as 8 (7.1 audio),
//     6 (5.1 audio), or 2 (stereo only).
const STB_VORBIS_MAX_CHANNELS : i32 = 16;  // enough for anyone?

// STB_VORBIS_PUSHDATA_CRC_COUNT [number]
//     after a flush_pushdata(), stb_vorbis begins scanning for the
//     next valid page, without backtracking. when it finds something
//     that looks like a page, it streams through it and verifies its
//     CRC32. Should that validation fail, it keeps scanning. But it's
//     possible that _while_ streaming through to check the CRC32 of
//     one candidate page, it sees another candidate page. This #define
//     determines how many "overlapping" candidate pages it can search
//     at once. Note that "real" pages are typically ~4KB to ~8KB, whereas
//     garbage pages could be as big as 64KB, but probably average ~16KB.
//     So don't hose ourselves by scanning an apparent 64KB page and
//     missing a ton of real ones in the interim; so minimum of 2
const STB_VORBIS_PUSHDATA_CRC_COUNT : i32 = 4;

// STB_VORBIS_FAST_HUFFMAN_LENGTH [number]
//     sets the log size of the huffman-acceleration table.  Maximum
//     supported value is 24. with larger numbers, more decodings are O(1),
//     but the table size is larger so worse cache missing, so you'll have
//     to probe (and try multiple ogg vorbis files) to find the sweet spot.
const STB_VORBIS_FAST_HUFFMAN_LENGTH : i32 = 10;


#[repr(C)]
pub struct stb_vorbis_alloc
{
   alloc_buffer: *const u8,
   alloc_buffer_length_in_bytes: i32,
}

pub type codetype = f32;

// @NOTE
//
// Some arrays below are tagged "//varies", which means it's actually
// a variable-sized piece of data, but rather than malloc I assume it's
// small enough it's better to just allocate it all together with the
// main thing
//
// Most of the variables are specified with the smallest size I could pack
// them into. It might give better performance to make them all full-sized
// integers. It should be safe to freely rearrange the structures or change
// the sizes larger--nothing relies on silently truncating etc., nor the
// order of variables.

const FAST_HUFFMAN_TABLE_SIZE : i32 =   (1 << STB_VORBIS_FAST_HUFFMAN_LENGTH);
const FAST_HUFFMAN_TABLE_MASK : i32 =   (FAST_HUFFMAN_TABLE_SIZE - 1);

// code length assigned to a value with no huffman encoding
const NO_CODE : i32 =   255;


#[repr(C)]
pub struct Codebook
{
   dimensions: c_int, entries: c_int,
   codeword_lengths: *mut u8,
   minimum_value: f32,
   delta_value: f32,
   value_bits: u8,
   lookup_type: u8,
   sequence_p: u8,
   sparse: u8,
   lookup_values: u32,
   multiplicands: *mut codetype,
   codewords: *mut u32,
//    #ifdef STB_VORBIS_FAST_HUFFMAN_SHORT
    fast_huffman: [i16; FAST_HUFFMAN_TABLE_SIZE as usize],
//    #else
    // i32  fast_huffman[FAST_HUFFMAN_TABLE_SIZE],
//    #endif
   sorted_codewords: *mut u32,
   sorted_values: *mut c_int,
   sorted_entries: c_int,
} 

#[repr(C)]
pub struct  Floor0
{
   order: u8,
   rate: u16,
   bark_map_size: u16,
   amplitude_bits: u8,
   amplitude_offset: u8,
   number_of_books: u8,
   book_list: [u8; 16], // varies
}

#[repr(C)]
pub struct Floor1
{
   partitions: u8,
   partition_class_list: [u8; 32], // varies
   class_dimensions: [u8; 16], // varies
   class_subclasses: [u8; 16], // varies
   class_masterbooks: [u8; 16], // varies
   subclass_books: [[i16; 8]; 16], // varies
   Xlist: [u16; 31*8+2], // varies
   sorted_order: [u8; 31*8+2],
   neighbors: [[u8; 2]; 31*8+2],
   floor1_multiplier: u8,
   rangebits: u8,
   values: c_int,
}

// union Floor
#[repr(C)]
pub struct Floor
{
   floor0: Floor0,
   floor1: Floor1,
}

#[repr(C)] 
pub struct Residue
{
   begin: u32, end: u32,
   part_size: u32,
   classifications: u8,
   classbook: u8,
   classdata: *mut *mut u8,
   residue_books: *mut [i16; 8],
} 

#[repr(C)]
pub struct MappingChannel
{
   magnitude: u8,
   angle: u8,
   mux: u8,
}


#[repr(C)]
pub struct Mapping
{
   coupling_steps: u16,
   chan: *mut MappingChannel,
   submaps: u8,
   submap_floor: [u8; 15], // varies
   submap_residue: [u8; 15], // varies
}


#[repr(C)]
#[derive(Copy, Clone)]
pub struct Mode
{
   blockflag: u8,
   mapping: u8,
   windowtype: u16,
   transformtype: u16,
}



#[repr(C)]
pub struct CRCscan
{
   goal_crc: u32,    // expected crc if match
   bytes_left: c_int,  // bytes left in packet
   crc_so_far: u32,  // running crc
   bytes_done: c_int,  // bytes processed in _current_ chunk
   sample_loc: u32,  // granule pos encoded in page
} 

#[repr(C)]
pub struct ProbedPage
{
   page_start: u32, page_end: u32,
   last_decoded_sample: u32
}
 


#[repr(C)]
pub struct stb_vorbis
{
  // user-accessible info
   sample_rate: c_uint,
   channels: c_int,

   setup_memory_required: c_uint,
   temp_memory_required: c_uint,
   setup_temp_memory_required: c_uint,

  // input config
// #ifndef STB_VORBIS_NO_STDIO
   f: *mut libc::FILE,
   f_start: u32,
   close_on_free: c_int,
// #endif

   stream: *mut u8,
   stream_start: *mut u8,
   stream_end: *mut u8,

   stream_len: u32,

   push_mode: u8,

   first_audio_page_offset: u32,

   p_first: ProbedPage, p_last: ProbedPage,

  // memory management
   alloc: stb_vorbis_alloc,
   setup_offset: c_int,
   temp_offset: c_int,

  // run-time results
   eof: c_int,
   error: c_int, //STBVorbisError,

  // user-useful data

  // header info
   blocksize: [c_int; 2],
   blocksize_0: c_int, blocksize_1: c_int,
   codebook_count: c_int,
   codebooks: *mut Codebook,
   floor_count: c_int,
   floor_types: [u16; 64], // varies
   floor_config: *mut Floor,
   residue_count: c_int,
   residue_types: [u16; 64], // varies
   residue_config: *mut Residue,
   mapping_count: c_int,
   mapping: *const Mapping,
   mode_count: c_int,
   mode_config: [Mode; 64],  // varies

   total_samples: u32,

  // decode buffer
   channel_buffers: [*mut f32; STB_VORBIS_MAX_CHANNELS as usize],
   outputs        : [*mut f32; STB_VORBIS_MAX_CHANNELS as usize],

   previous_window: [*mut f32; STB_VORBIS_MAX_CHANNELS as usize],
   previous_length: c_int,

//    #ifndef STB_VORBIS_NO_DEFER_FLOOR
   finalY: [*mut i16; STB_VORBIS_MAX_CHANNELS as usize],
//    #else
//    float *floor_buffers[STB_VORBIS_MAX_CHANNELS],
//    #endif

   current_loc: u32, // sample location of next frame to decode
   current_loc_valid: c_int,

  // per-blocksize precomputed data
   
   // twiddle factors
   A: [*mut f32; 2], B: [*mut f32; 2], C: [*mut f32; 2],
   window: [*mut f32; 2],
   bit_reverse: [*mut u16; 2],

  // current page/packet/segment streaming info
   serial: u32, // stream serial number for verification
   last_page: c_int,
   segment_count: c_int,
   segments: [u8; 255],
   page_flag: u8,
   bytes_in_seg: u8,
   first_decode: u8,
   next_seg: c_int,
   last_seg: c_int,  // flag that we're on the last segment
   last_seg_which: c_int, // what was the segment number of the last seg?
   acc: u32,
   valid_bits: c_int,
   packet_bytes: c_int,
   end_seg_with_known_loc: c_int,
   known_loc_for_packet: u32,
   discard_samples_deferred: c_int,
   samples_output: u32,

  // push mode scanning
   page_crc_tests: c_int, // only in push_mode: number of tests active, -1 if not searching
// #ifndef STB_VORBIS_NO_PUSHDATA_API
   scan: [CRCscan; STB_VORBIS_PUSHDATA_CRC_COUNT as usize],
// #endif

  // sample-access
   channel_buffer_start: c_int,
   channel_buffer_end: c_int,
}

pub type vorb = stb_vorbis;



////////   ERROR CODES

#[repr(C, i32)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum STBVorbisError
{
   VORBIS__no_error,

   VORBIS_need_more_data=1,             // not a real error

   VORBIS_invalid_api_mixing,           // can't mix API modes
   VORBIS_outofmem,                     // not enough memory
   VORBIS_feature_not_supported,        // uses floor 0
   VORBIS_too_many_channels,            // STB_VORBIS_MAX_CHANNELS is too small
   VORBIS_file_open_failure,            // fopen() failed
   VORBIS_seek_without_length,          // can't seek in unknown-length file

   VORBIS_unexpected_eof=10,            // file is truncated?
   VORBIS_seek_invalid,                 // seek past EOF

   // decoding errors (corrupt/invalid stream) -- you probably
   // don't care about the exact details of these

   // vorbis errors:
   VORBIS_invalid_setup=20,
   VORBIS_invalid_stream,

   // ogg errors:
   VORBIS_missing_capture_pattern=30,
   VORBIS_invalid_stream_structure_version,
   VORBIS_continued_packet_flag_invalid,
   VORBIS_incorrect_stream_serial_number,
   VORBIS_invalid_first_page,
   VORBIS_bad_packet_type,
   VORBIS_cant_find_last_page,
   VORBIS_seek_failed
}


// this has been repurposed so y is now the original index instead of y
#[repr(C)]
#[derive(Copy, Clone, Eq, Ord)]
struct Point
{
   x : u16,
   y : u16
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool{
        return self.x.eq(&other.x);
    }
}

use std::cmp::Ordering;
impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>{
        return self.x.partial_cmp(&other.x);
    }
}



// Converted function is here

#[no_mangle]
pub fn error(f: &mut vorb, e: c_int) -> c_int
{
    // NOTE: e is STBVorbisError
    f.error = e;
    if f.eof == 0 && e != STBVorbisError::VORBIS_need_more_data as c_int {
        f.error = e; // breakpoint for debugging
    }
    
    return 0;
}

#[no_mangle]
pub fn include_in_sort(c: &Codebook, len: u8) -> c_int
{
   if c.sparse != 0 { 
       assert!(len as c_int != NO_CODE); 
       return 1; // true
    }
   if len as c_int == NO_CODE {
       return 0; // false
   }
   if len as c_int > STB_VORBIS_FAST_HUFFMAN_LENGTH {
       return 1; // true
   }
   return 0;
}



#[no_mangle]
pub unsafe extern fn setup_malloc(f: &mut vorb, sz: c_int) -> *mut c_void
{
   let sz = (sz+3) & !3;
   f.setup_memory_required += sz as u32;
   if f.alloc.alloc_buffer.is_null() == false {
      let p = f.alloc.alloc_buffer.offset(f.setup_offset as isize);
      if f.setup_offset + sz > f.temp_offset {
          return std::ptr::null_mut();
      }
      f.setup_offset += sz as i32;
      return p as *mut c_void;
   }
   
   if sz!= 0 {
       return libc::malloc(sz as usize);
   }else{
       return std::ptr::null_mut();
   }
}

#[no_mangle]
pub unsafe extern fn setup_free(f: &mut vorb, p: *mut c_void)
{
   if f.alloc.alloc_buffer.is_null() == false {
       return; // do nothing; setup mem is a stack
   }
   libc::free(p);
}

#[no_mangle]
pub unsafe extern fn setup_temp_malloc(f: &mut vorb, sz: c_int) -> *mut c_void
{
   let sz = (sz+3) & !3;
   f.setup_memory_required += sz as u32;
   if f.alloc.alloc_buffer.is_null() == false {
      if f.temp_offset - sz < f.setup_offset {
          return std::ptr::null_mut();
      }
      f.temp_offset -= sz;
      return f.alloc.alloc_buffer.offset(f.temp_offset as isize) as *mut c_void;
   }
   return libc::malloc(sz as usize);
}

#[no_mangle]
pub unsafe extern fn setup_temp_free(f: &mut vorb, p: *mut c_void, sz: c_int)
{
   if f.alloc.alloc_buffer.is_null() == false {
      f.temp_offset += (sz+3)&!3;
      return;
   }
   libc::free(p);
}

const  CRC32_POLY  : u32 =  0x04c11db7;   // from spec

#[no_mangle]
pub unsafe extern fn crc32_init()
{
   for i in 0 .. 256 {
       let mut s : u32 = i << 24;
       for _ in 0 .. 8 {
           s = (s << 1) ^ (if s >= (1u32<<31) {CRC32_POLY} else {0});
       }
       crc_table[i as usize] = s;
   }
   
}



// used in setup, and for huffman that doesn't go fast path
#[no_mangle]
pub extern fn bit_reverse(n: c_uint) -> c_uint 
{
  let n = ((n & 0xAAAAAAAA) >>  1) | ((n & 0x55555555) << 1);
  let n = ((n & 0xCCCCCCCC) >>  2) | ((n & 0x33333333) << 2);
  let n = ((n & 0xF0F0F0F0) >>  4) | ((n & 0x0F0F0F0F) << 4);
  let n = ((n & 0xFF00FF00) >>  8) | ((n & 0x00FF00FF) << 8);
  return (n >> 16) | (n << 16);
}


#[no_mangle]
pub extern fn square(x: f32) -> f32{
    x * x
}

/////////////////////// LEAF SETUP FUNCTIONS //////////////////////////
//
// these functions are only called at setup, and only a few times
// per file
#[no_mangle]
pub extern fn float32_unpack(x: u32) -> f32
{
   // from the specification
   let mantissa : u32 = x & 0x1fffff;
   let sign : u32 = x & 0x80000000;
   let exp : u32 = (x & 0x7fe00000) >> 21;
   let res: f64 = if sign != 0 {
     -(mantissa as f64)
   }else{
       mantissa as f64
   };
   
   return f64::ldexp(res, (exp as i32 - 788 ) as isize) as f32;
}

// zlib & jpeg huffman tables assume that the output symbols
// can either be arbitrarily arranged, or have monotonically
// increasing frequencies--they rely on the lengths being sorted;
// this makes for a very simple generation algorithm.
// vorbis allows a huffman table with non-sorted lengths. This
// requires a more sophisticated construction, since symbols in
// order do not map to huffman codes "in order".
#[no_mangle]
pub unsafe extern fn add_entry(c: &Codebook, huff_code: u32, symbol: c_int, count: c_int, len: c_int, values: *mut u32)
{
    // TODO(bungcip): maybe change len as u8?
    // TODO(bungcip): maybe symbol len as u32?
    
   if c.sparse == 0 {
      *c.codewords.offset(symbol as isize) = huff_code;
   } else {
      let count = count as isize;
      *c.codewords.offset(count) = huff_code;
      *c.codeword_lengths.offset(count) = len as u8;
      *values.offset(count) = symbol as u32;
   }
}


// this is a weird definition of log2() for which log2(1) = 1, log2(2) = 2, log2(4) = 3
// as required by the specification. fast(?) implementation from stb.h
// @OPTIMIZE: called multiple times per-packet with "constants"; move to setup
#[no_mangle]
pub extern fn ilog(n: i32) -> i32
{
    static log2_4: [i8; 16] = [0,1,2,2,3,3,3,3,4,4,4,4,4,4,4,4];

    let n = n as usize;

    // 2 compares if n < 16, 3 compares otherwise (4 if signed or n > 1<<29)
    let result = if n < (1 << 14) {
        if n < (1 << 4) {
            0 + log2_4[n]
        } else if n < (1 << 9) {
            5 + log2_4[n >> 5]
        } else {
            10 + log2_4[n >> 10]
        }
    }
    else if n < (1 << 24) {
        if n < (1 << 19) {
            15 + log2_4[n >> 15]
        }
        else {
            20 + log2_4[n >> 20]
        }
    }
    else if n < (1 << 29) {
        25 + log2_4[n >> 25]
    }
    else if n < (1 << 31) {
        30 + log2_4[n >> 30]
    }
    else {
        0 // signed n returns 0
    };
    
    result as i32
       
}

#[no_mangle]
pub extern fn get_window(f: &vorb, len: c_int) -> *mut f32
{
   let len = len << 1;
   if len == f.blocksize_0 { return f.window[0]; }
   if len == f.blocksize_1 { return f.window[1]; }

   unreachable!();
}

#[no_mangle]
pub unsafe extern fn compute_bitreverse(n: c_int, rev: *mut u16)
{
   let ld = ilog(n) - 1; // ilog is off-by-one from normal definitions
   let n8 = n >> 3;
   
   for i in 0 .. n8 {
       *rev.offset(i as isize) = ((bit_reverse(i as c_uint) >> (32-ld+3)) << 2) as u16;
   }
}

#[no_mangle]
pub extern fn uint32_compare(p: *const c_void, q: *const c_void) -> c_int
{
    unsafe {
        let x = std::ptr::read(p as *const u32);        
        let y = std::ptr::read(q as *const u32);
        if x < y {
            return -1;
        }else{
            if x > y {
                return 1;
            }else{
                return 0
            }
        }
    }
}


// only run while parsing the header (3 times)
#[no_mangle]
pub extern fn vorbis_validate(data: *const u8) -> c_int
{
    static vorbis: &'static [u8; 6] = b"vorbis";
    unsafe {
        let result = libc::memcmp(data as *const c_void, vorbis.as_ptr() as *const c_void, 6) == 0;    
        if result {
            return 1;
        }else{
            return 0;
        }
    }
}

// called from setup only, once per code book
// (formula implied by specification)
#[no_mangle]
pub extern fn lookup1_values(entries: c_int, dim: c_int) -> c_int
{
    let mut r =  f64::floor(f64::exp(f64::ln(entries as f64) / dim as f64)) as c_int;
    if f64::floor(f64::powi( (r+1) as f64, dim)) as c_int <= entries {
       r += 1;
    }
    assert!(f64::powi((r+1) as f64, dim) > entries as f64);
    assert!(f64::powi(r as f64, dim) as c_int <= entries);
    return r;
}


#[no_mangle]
pub extern fn neighbors(x: *mut u16, n: c_int, plow: *mut c_int, phigh: *mut c_int)
{
    let mut low : i32 = -1;
    let mut high : i32 = 65536;
    
    for i in 0 .. n {
        unsafe {
            if (*x.offset(i as isize) as i32) > low && (*x.offset(i as isize) as i32) < (*x.offset(n as isize) as i32) { 
                *plow = i;
                low = *x.offset(i as isize) as i32; 
            }
            if (*x.offset(i as isize) as i32) < high && (*x.offset(i as isize) as i32) > (*x.offset(n as isize) as i32) { 
                *phigh = i; 
                high = *x.offset(i as isize) as i32;
            }
        }
    }
}

#[no_mangle]
pub unsafe extern fn point_compare(p: *const c_void, q: *const c_void) -> c_int
{
   let a : &Point = std::mem::transmute(p as *const Point);
   let b : &Point = std::mem::transmute(q as *const Point);
   
   if a.x < b.x {
       return -1;
   }else if a.x > b.x {
       return 1;
   }else {
       return 0;
   }
}

macro_rules! USE_MEMORY {
    ($z: expr) => {
        $z.stream.is_null() == false
    }
}

macro_rules! IS_PUSH_MODE {
    ($f: expr) => {
        $f.push_mode != 0
    }
}

#[no_mangle]
pub unsafe extern fn get8(z: &mut vorb) -> u8
{
   if USE_MEMORY!(z) {
      if z.stream >= z.stream_end { 
          z.eof = 1;
          return 0;
      }
      z.stream = z.stream.offset(1);
      return *z.stream;
   }

   let c = libc::fgetc(z.f);
   if c == libc::EOF { 
       z.eof = 1; return 0; 
    }
   return c as u8;
}


#[no_mangle]
pub unsafe extern fn get32(f: &mut vorb) -> u32
{
   let mut x : u32 = get8(f) as u32;
   x += (get8(f) as u32) << 8;
   x += (get8(f) as u32) << 16;
   x += (get8(f) as u32) << 24;
   return x;
}

#[no_mangle]
pub unsafe extern fn getn(z: &mut vorb, data: *mut u8, n: c_int) -> c_int
{
   if USE_MEMORY!(z) {
      if z.stream.offset(n as isize) > z.stream_end { z.eof = 1; return 0; }
      std::ptr::copy_nonoverlapping(z.stream, data, n as usize);
    //   libc::memcpy(data, z.stream, n);
      z.stream = z.stream.offset(n as isize);
      return 1;
   }

   if libc::fread(data as *mut c_void, n as usize, 1, z.f) == 1 {
      return 1;
   } else {
      z.eof = 1;
      return 0;
   }
}

#[no_mangle]
pub unsafe extern fn skip(z: &mut vorb, n: c_int)
{
   if USE_MEMORY!(z) {
      z.stream = z.stream.offset(n as isize);
      if z.stream >= z.stream_end {z.eof = 1;}
      return;
   }

   let x = libc::ftell(z.f);
   libc::fseek(z.f, x+n, libc::SEEK_SET);
}

#[no_mangle]
pub unsafe extern fn capture_pattern(f: &mut vorb) -> c_int
{
   if 0x4f != get8(f) {return 0;}
   if 0x67 != get8(f) {return 0;}
   if 0x67 != get8(f) {return 0;}
   if 0x53 != get8(f) {return 0;}
   return 1;
}


const EOP : i32 = -1;
const INVALID_BITS : i32 = -1;

#[no_mangle]
pub unsafe extern fn get8_packet_raw(f: *mut vorb) -> c_int
{
    let f : &mut vorb = std::mem::transmute(f as *mut vorb); 
    if f.bytes_in_seg == 0 {
        if f.last_seg != 0 {
            return EOP;
        }else if next_segment(f) == 0 {
            return EOP;
        }
    }
    
    assert!(f.bytes_in_seg > 0);
    
    f.bytes_in_seg -= 1;
    f.packet_bytes += 1;
    
    return get8(f) as c_int;
}

#[no_mangle]
pub unsafe extern fn get8_packet(f: *mut vorb) -> c_int
{
    let x = get8_packet_raw(f);
    
    let f : &mut vorb = std::mem::transmute(f as *mut vorb); 
    f.valid_bits = 0;
    
    return x;
}

#[no_mangle]
pub unsafe extern fn flush_packet(f: *mut vorb)
{
    while get8_packet_raw(f) != EOP {}
}


// @OPTIMIZE: this is the secondary bit decoder, so it's probably not as important
// as the huffman decoder?
#[no_mangle]
pub unsafe extern fn get_bits(f: &mut vorb, n: c_int) -> u32
{
   let mut z : u32;

   if f.valid_bits < 0 {return 0;}
   if f.valid_bits < n {
      if n > 24 {
         // the accumulator technique below would not work correctly in this case
         z = get_bits(f, 24);
         z += get_bits(f, n-24) << 24;
         return z;
      }
      if f.valid_bits == 0 {f.acc = 0;}
      while f.valid_bits < n {
         let z = get8_packet_raw(f);
         if z == EOP {
            f.valid_bits = INVALID_BITS;
            return 0;
         }
         f.acc += (z as u32) << f.valid_bits;
         f.valid_bits += 8;
      }
   }
   if f.valid_bits < 0 {return 0;}
   z = f.acc & ((1 << n)-1);
   f.acc >>= n;
   f.valid_bits -= n;
   return z;
}


#[no_mangle]
pub unsafe extern fn start_page(f: &mut vorb) -> c_int
{
   if capture_pattern(f) == 0 {
       return error(f, STBVorbisError::VORBIS_missing_capture_pattern as i32);
   } 
   return start_page_no_capturepattern(f);
}


const PAGEFLAG_continued_packet : c_int =   1;
const PAGEFLAG_first_page       : c_int =   2;
const PAGEFLAG_last_page        : c_int =   4;


#[no_mangle]
pub unsafe extern fn start_packet(f: &mut vorb) -> c_int
{
   while f.next_seg == -1 {
      if start_page(f) == 0 { return 0; } // false
      if (f.page_flag & PAGEFLAG_continued_packet as u8) != 0 {
         return error(f, STBVorbisError::VORBIS_continued_packet_flag_invalid as i32);
      }
   }
   f.last_seg = 0; // false
   f.valid_bits = 0;
   f.packet_bytes = 0;
   f.bytes_in_seg = 0;
   // f.next_seg is now valid
   return 1; // true
}

#[no_mangle]
pub unsafe extern fn maybe_start_packet(f: &mut vorb) -> c_int
{
    use STBVorbisError::{VORBIS_missing_capture_pattern, VORBIS_continued_packet_flag_invalid};
    
   if f.next_seg == -1 {
      let x = get8(f) as i32;
      if f.eof != 0 { return 0; } // EOF at page boundary is not an error!
      if 0x4f != x       { return error(f, VORBIS_missing_capture_pattern as c_int); }
      if 0x67 != get8(f) { return error(f, VORBIS_missing_capture_pattern as c_int); }
      if 0x67 != get8(f) { return error(f, VORBIS_missing_capture_pattern as c_int); }
      if 0x53 != get8(f) { return error(f, VORBIS_missing_capture_pattern as c_int); }
      if start_page_no_capturepattern(f) == 0 { return 0; }
      if (f.page_flag & PAGEFLAG_continued_packet as u8) != 0 {
         // set up enough state that we can read this packet if we want,
         // e.g. during recovery
         f.last_seg = 0;
         f.bytes_in_seg = 0;
         return error(f, VORBIS_continued_packet_flag_invalid as c_int);
      }
   }
   return start_packet(f);
}

#[no_mangle]
pub unsafe extern fn next_segment(f: &mut vorb) -> c_int
{
    use STBVorbisError::VORBIS_continued_packet_flag_invalid;
//    int len;
   if f.last_seg != 0 {return 0;}
   if f.next_seg == -1 {
      f.last_seg_which = f.segment_count-1; // in case start_page fails
      if start_page(f) == 0 { f.last_seg = 1; return 0; }
      if (f.page_flag & PAGEFLAG_continued_packet as u8) == 0 {return error(f, VORBIS_continued_packet_flag_invalid as c_int); }
   }
   
   let len = f.segments[f.next_seg as usize];
   f.next_seg += 1;
   
   if len < 255 {
      f.last_seg = 1; // true
      f.last_seg_which = f.next_seg-1;
   }
   if f.next_seg >= f.segment_count{
      f.next_seg = -1;
   }
   assert!(f.bytes_in_seg == 0);
   f.bytes_in_seg = len;
   return len as i32;
}



#[no_mangle]
pub unsafe extern fn vorbis_decode_packet(f: &mut vorb, len: &mut c_int, p_left: &mut c_int, p_right: &mut c_int) -> c_int
{
    let mut mode : c_int = 0;
    let mut left_end: c_int = 0;
    let mut right_end: c_int = 0;
    
    if vorbis_decode_initial(f, p_left, &mut left_end, p_right, &mut right_end, &mut mode) == 0{
        return 0;
    }
    
    return vorbis_decode_packet_rest(
        f, len, &mut f.mode_config[mode as usize], 
        *p_left, left_end, *p_right, right_end, p_left
    );
}


#[no_mangle]
pub unsafe extern fn vorbis_pump_first_frame(f: &mut stb_vorbis)
{
    let mut len: c_int = 0;
    let mut right: c_int = 0;
    let mut left: c_int = 0;
    
    if vorbis_decode_packet(f, &mut len, &mut left, &mut right) != 0 {
        vorbis_finish_frame(f, len, left, right);
    }
}

#[no_mangle]
pub unsafe extern fn stb_vorbis_close(p: *mut stb_vorbis)
{
   if p.is_null(){
       return;
   }
   
   vorbis_deinit(p);
   setup_free(std::mem::transmute(p),p as *mut c_void);
}

#[no_mangle]
pub unsafe extern fn stb_vorbis_open_file_section(file: *mut libc::FILE, close_on_free: c_int, error: *mut c_int, alloc: *const stb_vorbis_alloc, length: c_uint) -> *mut stb_vorbis
{
    let mut p : stb_vorbis = std::mem::zeroed();
    
    vorbis_init(&mut p, alloc);
   p.f = file;
   p.f_start = ftell(file) as u32;
   p.stream_len   = length;
   p.close_on_free = close_on_free;
    
   if start_decoder(&mut p) != 0 {
      let mut f = vorbis_alloc(&mut p);
      if f.is_null() == false {
         *f = p;
         vorbis_pump_first_frame(std::mem::transmute(f));
         return f;
      }
   }
   
   if error.is_null() == false {
       *error = p.error;
   } 
   vorbis_deinit(&mut p);
   
   return std::ptr::null_mut();
}


#[no_mangle]
pub unsafe extern fn stb_vorbis_open_file(file: *mut FILE,  close_on_free: c_int, error: *mut c_int, alloc: *const stb_vorbis_alloc) -> *mut stb_vorbis
{
    let start = libc::ftell(file);
    libc::fseek(file, 0, libc::SEEK_END);
    
    let len = libc::ftell(file) - start;
    libc::fseek(file, start, libc::SEEK_SET);
    
    return stb_vorbis_open_file_section(file, close_on_free, error, alloc, len as c_uint);
}


#[no_mangle]
pub unsafe extern fn stb_vorbis_open_filename(filename: *const i8, error: *mut c_int, alloc: *const stb_vorbis_alloc) -> *mut stb_vorbis
{
   let  mode: &'static [u8; 3] = b"rb\0";
   let f = libc::fopen(filename, mode.as_ptr() as *const i8);
   if f.is_null() == false {
      return stb_vorbis_open_file(f, 1, error, alloc);
   } 
   
   if error.is_null() == false {
     *error = STBVorbisError::VORBIS_file_open_failure as i32;  
   } 
   return std::ptr::null_mut();
}


// The meaning of "left" and "right"
//
// For a given frame:
//     we compute samples from 0..n
//     window_center is n/2
//     we'll window and mix the samples from left_start to left_end with data from the previous frame
//     all of the samples from left_end to right_start can be output without mixing; however,
//        this interval is 0-length except when transitioning between short and long frames
//     all of the samples from right_start to right_end need to be mixed with the next frame,
//        which we don't have, so those get saved in a buffer
//     frame N's right_end-right_start, the number of samples to mix with the next frame,
//        has to be the same as frame N+1's left_end-left_start (which they are by
//        construction)

#[no_mangle]
pub unsafe extern fn vorbis_decode_initial(f: &mut vorb, p_left_start: *mut c_int, p_left_end: *mut c_int, p_right_start: *mut c_int, p_right_end: *mut c_int, mode: *mut c_int) -> c_int
{
   f.channel_buffer_start = 0;
   f.channel_buffer_end = 0;

   loop {
        if f.eof != 0 {return 0;} // false
        if maybe_start_packet(f) == 0 {
            return 0; // false
        }
        // check packet type
        if get_bits(f,1) != 0 {
            if IS_PUSH_MODE!(f) {
                return error(f, STBVorbisError::VORBIS_bad_packet_type as c_int);
            }
            while EOP != get8_packet(f){}
            continue;
        }
        
       break;
   }

   if f.alloc.alloc_buffer.is_null() == false {
      assert!(f.alloc.alloc_buffer_length_in_bytes == f.temp_offset);
   }

   let x = ilog(f.mode_count-1);
   let i : c_int = get_bits(f, x) as c_int;
   if i == EOP {return 0;} // false
   if i >= f.mode_count {return 0;} // false
   
   *mode = i;

   // NOTE: hack to forget borrow
   let &mut m = {
       let _borrow = &mut f.mode_config[i as usize];
       let _borrow = _borrow as *mut _;
       let _borrow : &mut Mode = std::mem::transmute(_borrow);
       _borrow
   };
   let n : c_int;
   let prev: c_int;
   let next: c_int;
   
   if m.blockflag != 0 {
      n = f.blocksize_1;
      prev = get_bits(f,1) as c_int;
      next = get_bits(f,1) as c_int;
   } else {
      prev = 0;
      next = 0;
      n = f.blocksize_0;
   }

// WINDOWING

   let window_center = n >> 1;
   if m.blockflag != 0 && prev == 0 {
      *p_left_start = (n - f.blocksize_0) >> 2;
      *p_left_end   = (n + f.blocksize_0) >> 2;
   } else {
      *p_left_start = 0;
      *p_left_end   = window_center;
   }
   if m.blockflag != 0 && next == 0 {
      *p_right_start = (n*3 - f.blocksize_0) >> 2;
      *p_right_end   = (n*3 + f.blocksize_0) >> 2;
   } else {
      *p_right_start = window_center;
      *p_right_end   = n;
   }

   return 1; // true
}


// Below is function that still live in C code
extern {
    static mut crc_table: [u32; 256];
        
    pub fn vorbis_finish_frame(f: *mut stb_vorbis, len: c_int, left: c_int, right: c_int) -> c_int;
    
    // pub fn vorbis_decode_initial(f: *mut vorb, p_left_start: *mut c_int, p_left_end: *mut c_int, p_right_start: *mut c_int, p_right_end: *mut c_int, mode: *mut c_int) -> c_int;
    pub fn vorbis_decode_packet_rest(f: *mut vorb, len: *mut c_int, m: *mut Mode, left_start: c_int, left_end: c_int, right_start: c_int, right_end: c_int, p_left: *mut c_int) -> c_int;

    pub fn start_page_no_capturepattern(f: *mut vorb) -> c_int;

    pub fn vorbis_deinit(f: *mut stb_vorbis);
    pub fn vorbis_init(f: *mut stb_vorbis, z: *const stb_vorbis_alloc);
    pub fn vorbis_alloc(f: *mut stb_vorbis) -> *mut stb_vorbis;

    pub fn start_decoder(f: *mut vorb) -> c_int;

    /// Real API

    pub fn stb_vorbis_decode_filename(
        filename: *const i8, 
        channels: *mut c_int, 
        sample_rate: *mut c_int, 
        output: *mut *mut i16) -> c_int;
}