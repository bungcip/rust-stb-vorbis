// temporary disable lint for now...
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(non_upper_case_globals)]

#![feature(question_mark, custom_derive, box_syntax, float_extras)]

/// straight port from stb_vorbis
/// Ogg Vorbis audio decoder - v1.09 - public domain
/// http://nothings.org/stb_vorbis/
///  

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::ptr;
use std::mem;



pub type codetype = f32;

///////////   MEMORY ALLOCATION

// normally stb_vorbis uses malloc() to allocate memory at startup,
// and alloca() to allocate temporary memory during a frame on the
// stack. (Memory consumption will depend on the amount of setup
// data in the file and how you set the compile flags for speed
// vs. size. In my test files the maximal-size usage is ~150KB.)
//
// You can modify the wrapper functions in the source (setup_malloc,
// setup_temp_malloc, temp_malloc) to change this behavior, or you
// can use a simpler allocation model: you pass in a buffer from
// which stb_vorbis will allocate _all_ its memory (including the
// temp memory). "open" may fail with a VORBIS_outofmem if you
// do not pass in enough data; there is no way to determine how
// much you do need except to succeed (at which poi32 you can
// query get_info to find the exact amount required. yes I know
// this is lame).
//
// If you pass in a non-NULL buffer of the type below, allocation
// will occur from it as described above. Otherwise just pass NULL
// to use malloc()/alloca()

#[repr(C)]
pub struct stb_vorbis_alloc
{
   alloc_buffer: *const u8,
   alloc_buffer_length_in_bytes: i32,
}

// STB_VORBIS_MAX_CHANNELS [number]
//     globally define this to the maximum number of channels you need.
//     The spec does not put a restriction on channels except that
//     the count is stored in a byte, so 255 is the hard limit.
//     Reducing this saves about 16 bytes per value, so using 16 saves
//     (255-16)*16 or around 4KB. Plus anything other memory usage
//     I forgot to account for. Can probably go as low as 8 (7.1 audio),
//     6 (5.1 audio), or 2 (stereo only).
pub const STB_VORBIS_MAX_CHANNELS: usize = 16; // enough for anyone?  

// STB_VORBIS_FAST_HUFFMAN_LENGTH [number]
//     sets the log size of the huffman-acceleration table.  Maximum
//     supported value is 24. with larger numbers, more decodings are O(1),
//     but the table size is larger so worse cache missing, so you'll have
//     to probe (and try multiple ogg vorbis files) to find the sweet spot.
pub const STB_VORBIS_FAST_HUFFMAN_LENGTH : u8 = 10;


macro_rules! CHECK {
    ($f: tt) => {
        unsafe{
            assert!($f.channel_buffers.offset(1) != ptr::null());
        }
    }
    // (f: expr) => {
        
    // }
}


fn neighbors(x: &mut [u16], n: i32, plow: &mut i32, phigh: &mut i32)
{
    unimplemented!();
//    int low = -1;
//    int high = 65536;
//    int i;
//    for (i=0; i < n; ++i) {
//       if (x[i] > low  && x[i] < x[n]) { *plow  = i; low = x[i]; }
//       if (x[i] < high && x[i] > x[n]) { *phigh = i; high = x[i]; }
//    }
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

#[repr(C)]
#[derive(Clone)]
pub struct Codebook
{
   dimensions: i32, entries: i32,
   codeword_lengths: Vec<u8>,
   minimum_value: f32,
     delta_value: f32,
     value_bits: u8,
     lookup_type: u8,
     sequence_p: u8,
     sparse: bool,
    lookup_values: u32,
    multiplicands: Vec<codetype>,
    codewords: Vec<u32>,
//    #ifdef STB_VORBIS_FAST_HUFFMAN_SHORT
//     i16  fast_huffman: [i16; FAST_HUFFMAN_TABLE_SIZE],
//    #else
//     int32  fast_huffman[i32; FAST_HUFFMAN_TABLE_SIZE],
//    #endif
   sorted_codewords: Vec<u32>,
   sorted_values: Vec<i32>,
   sorted_entries: i32,
}

impl Default for Codebook {
    fn default() -> Self {
        let mut instance : Codebook = unsafe{ mem::zeroed() };
        
        instance.codeword_lengths = Vec::new();
        instance.codewords = Vec::new();
        instance.sorted_codewords = Vec::new();
        instance.sorted_values = Vec::new();
        instance.multiplicands = Vec::new();
        
        instance
    }
}


#[repr(C)]
#[derive(Clone, Copy)]
pub struct Floor0
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
#[derive(Copy)]
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
   values: i32,
} 

impl Clone for Floor1 {
    fn clone(&self) -> Self {
        *self
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
// FIXME: change to union after rust support it
// pub union Floor { 
pub struct Floor {    
   floor0: Floor0,
   floor1: Floor1,
}

impl Default for Floor {
    fn default() -> Self {
        let instance : Floor = unsafe { mem::zeroed() };

        instance        
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Residue
{
   begin: u32, 
   end: u32,
   part_size: u32,
   classifications: u8,
   classbook: u8,
   classdata: *const (*const u8),
   residue_books: [*const i16; 8], 
}

impl Default for Residue {
    fn default() -> Self {
        let instance : Residue = unsafe { mem::zeroed() };
        
        instance        
    }
}

#[repr(C)]
// #[derive(Copy)]
pub struct MappingChannel
{
   magnitude: u8,
   angle: u8,
   mux: u8,
}

#[repr(C)]
// #[derive(Copy)]
pub struct Mapping
{
   coupling_steps: u16,
   chan: *const MappingChannel,
   submaps: u8,
   submap_floor: [u8; 15], // varies
   submap_residue: [u8; 15], // varies
} 

#[repr(C)]
pub struct  Mode
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
   bytes_left: i32,  // bytes left in packet
   crc_so_far: u32,  // running crc
   bytes_done: i32,  // bytes processed in _current_ chunk
   sample_loc: u32,  // granule pos encoded in page
}

#[repr(C)]
pub struct ProbedPage {
   page_start: u32, 
   page_end: u32,
   last_decoded_sample: u32
} 




#[repr(C)]
// #[derive(Default)]
pub struct stb_vorbis {
    sample_rate: u32,
    channels: i32,
    setup_memory_required: u32, // maybe usize?
    temp_memory_required: u32, // maybe usize?
    setup_temp_memory_required: u32, // maybe usize?
    
    // input config
    f: Option<File>,
    f_start: u64,
    close_on_free: bool,


    stream: *mut u8,
    stream_start: *mut u8,
    stream_end: *mut u8,
    
    stream_len: u32,
    
    push_mode: bool,
    
    first_audio_page_offset: u32,
    
    p_first: ProbedPage,
    p_last: ProbedPage,
    
    // memory management
    alloc: stb_vorbis_alloc,
    setup_offset: i32,
    temp_offset: i32,
    
    // run-time results
    eof: bool,
    error: STBVorbisError,

    // user-useful data
    // header info
   blocksize: [i32; 2],
   blocksize_0: i32,
   blocksize_1: i32,
   
   
   codebook_count: i32,
   codebooks: Vec<Codebook>,
   floor_count: i32,
   floor_types: [i32; 64],  // varies
   floor_config: Vec<Floor>,
   residue_count: i32,
   residue_types: [u16; 64], // varies
   residue_config: Vec<Residue>,
   mapping_count: i32,
   mapping: *const Mapping,
   mode_count: i32,
   mode_config: [Mode; 64],  // varies
   
   total_samples: u32,
   
   // decode buffer
   channel_buffers : *const [f32; STB_VORBIS_MAX_CHANNELS],
   outputs: *const [f32; STB_VORBIS_MAX_CHANNELS],

   previous_window: *const [f32; STB_VORBIS_MAX_CHANNELS],
   previous_length: i32,

//    #ifndef STB_VORBIS_NO_DEFER_FLOOR
//    i16 *finalY[STB_VORBIS_MAX_CHANNELS],
//    #else
//    f32 *floor_buffers[STB_VORBIS_MAX_CHANNELS],
//    #endif

    current_loc: u32, // sample location of next frame to decode
   current_loc_valid: i32,

  // per-blocksize precomputed data
   
   // twiddle factors
   A: *const [f32; 2],
   B: *const [f32; 2],
   C: *const [f32; 2],
   
   window: *const [f32; 2],
   bit_reverse: [u16; 2],
   
  // current page/packet/segment streaming info
   serial: u32, // stream serial number for verification
   last_page: i32,
    segment_count: i32,
    segments: [u8; 255],
    page_flag: u8,
    bytes_in_seg: u8,
    first_decode: u8,
    next_seg: i32,
    last_seg: bool,  // flag that we're on the last segment
    last_seg_which: i32, // what was the segment number of the last seg?
    acc: u32,
    valid_bits: i32,
    packet_bytes: i32,
    end_seg_with_known_loc: i32,
    known_loc_for_packet: u32,
    discard_samples_deferred: i32,
    samples_output: u32,

  // push mode scanning
   page_crc_tests: i32, // only in push_mode: number of tests active, -1 if not searching
// #ifndef STB_VORBIS_NO_PUSHDATA_API
//    CRCscan scan[STB_VORBIS_PUSHDATA_CRC_COUNT],
// #endif

// sample-access
   channel_buffer_start: i32,
   channel_buffer_end: i32,
}

impl Default for stb_vorbis {
    fn default() -> Self {
        let mut instance : stb_vorbis = unsafe { mem::zeroed() };
        instance.floor_config = Vec::new();
        instance.residue_config = Vec::new();
        
        instance
    }
}


// this is a weird definition of log2() for which log2(1) = 1, log2(2) = 2, log2(4) = 3
// as required by the specification. fast(?) implementation from stb.h
// @OPTIMIZE: called multiple times per-packet with "constants"; move to setup
fn ilog(n: i32) -> i32
{
//    static signed char log2_4[16] = { 0,1,2,2,3,3,3,3,4,4,4,4,4,4,4,4 };
    static log2_4: [i8; 16] = [0,1,2,2,3,3,3,3,4,4,4,4,4,4,4,4];

    let n = n as usize;

    // 2 compares if n < 16, 3 compares otherwise (4 if signed or n > 1<<29)
    let result = if (n < (1 << 14)) {
        if (n < (1 << 4)) {
            0 + log2_4[n]
        }
        else if (n < (1 << 9)) {
            5 + log2_4[n >> 5]
        }
        else {
            10 + log2_4[n >> 10]
        }
    }
    else if (n < (1 << 24)) {
        if (n < (1 << 19)) {
            15 + log2_4[n >> 15]
        }
        else {
            20 + log2_4[n >> 20]
        }
    }
    else if (n < (1 << 29)) {
        25 + log2_4[n >> 25]
    }
    else if (n < (1 << 31)) {
        30 + log2_4[n >> 30]
    }
    else {
        0 // signed n returns 0
    };
    
    result as i32
       
}

fn compute_codewords(c: &mut Codebook, len: *const u8, n: i32, values: &mut [u32]) -> bool
{
    unimplemented!();
//    int i,k,m=0;
//    uint32 available[32];

//    memset(available, 0, sizeof(available));
//    // find the first entry
//    for (k=0; k < n; ++k) if (len[k] < NO_CODE) break;
//    if (k == n) { assert(c->sorted_entries == 0); return TRUE; }
//    // add to the list
//    add_entry(c, 0, k, m++, len[k], values);
//    // add all available leaves
//    for (i=1; i <= len[k]; ++i)
//       available[i] = 1U << (32-i);
//    // note that the above code treats the first case specially,
//    // but it's really the same as the following code, so they
//    // could probably be combined (except the initial code is 0,
//    // and I use 0 in available[] to mean 'empty')
//    for (i=k+1; i < n; ++i) {
//       uint32 res;
//       int z = len[i], y;
//       if (z == NO_CODE) continue;
//       // find lowest available leaf (should always be earliest,
//       // which is what the specification calls for)
//       // note that this property, and the fact we can never have
//       // more than one free leaf at a given level, isn't totally
//       // trivial to prove, but it seems true and the assert never
//       // fires, so!
//       while (z > 0 && !available[z]) --z;
//       if (z == 0) { return FALSE; }
//       res = available[z];
//       assert(z >= 0 && z < 32);
//       available[z] = 0;
//       add_entry(c, bit_reverse(res), i, m++, len[i], values);
//       // propogate availability up the tree
//       if (z != len[i]) {
//          assert(len[i] >= 0 && len[i] < 32);
//          for (y=len[i]; y > z; --y) {
//             assert(available[y] == 0);
//             available[y] = res + (1 << (32-y));
//          }
//       }
//    }
//    return TRUE;
}

// accelerated huffman table allows fast O(1) match of all symbols
// of length <= STB_VORBIS_FAST_HUFFMAN_LENGTH
fn compute_accelerated_huffman(c: &mut Codebook)
{
    unimplemented!();
//    int i, len;
//    for (i=0; i < FAST_HUFFMAN_TABLE_SIZE; ++i)
//       c->fast_huffman[i] = -1;

//    len = c->sparse ? c->sorted_entries : c->entries;
//    #ifdef STB_VORBIS_FAST_HUFFMAN_SHORT
//    if (len > 32767) len = 32767; // largest possible value we can encode!
//    #endif
//    for (i=0; i < len; ++i) {
//       if (c->codeword_lengths[i] <= STB_VORBIS_FAST_HUFFMAN_LENGTH) {
//          uint32 z = c->sparse ? bit_reverse(c->sorted_codewords[i]) : c->codewords[i];
//          // set table entries for all bit combinations in the higher bits
//          while (z < FAST_HUFFMAN_TABLE_SIZE) {
//              c->fast_huffman[z] = i;
//              z += 1 << c->codeword_lengths[i];
//          }
//       }
//    }
}



// code length assigned to a value with no huffman encoding
const  NO_CODE : u8 = 255;

/////////////////////// LEAF SETUP FUNCTIONS //////////////////////////
//
// these functions are only called at setup, and only a few times
// per file
fn float32_unpack(x: u32) -> f32
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
   
   // FIXME: nanti check lagi
   return f64::ldexp(res, (exp-788) as isize) as f32;
}



const  CRC32_POLY : u32 = 0x04c11db7;   // from spec

//FIXME: change to lazy_init! macro
static mut crc_table: [u32; 256] = [0; 256];

fn crc32_init()
{
    unsafe {
        for i in 0 .. 256 {
            let mut s : u32 = i << 24;
            let mut j = 0;
            while j < 8 {
                s = (s << 1) ^ (if s >= (1u32<<31) {CRC32_POLY }else{0} );
                j += 1;
            }
            crc_table[i as usize] = s;
            
        }
    }
}

fn error(f: &mut stb_vorbis, e: STBVorbisError) -> bool
{
   f.error = e;
   if !f.eof && e != STBVorbisError::VORBIS_need_more_data {
      f.error = e; // breakpoint for debugging
   }
   return false;
}

fn get8(z: &mut stb_vorbis) -> u8
{
    use std::io::Read;
    
    unsafe {
        if z.stream.is_null() == false {
            if z.stream >= z.stream_end { z.eof = true; return 0; }
            z.stream = z.stream.offset(1);
            return *(z.stream);
        }
    }
    
    let mut f = match z.f {
        None => return 0,
        Some(ref f) => f
    };

    let mut buf : [u8; 1] = [0u8; 1];
    match f.read(&mut buf){
        Err(why) => {
            // debug
            // println!("why: {}", why);
            
            z.eof = true;
        },
        Ok(_) => ()
    }
    
    // println!("ch: {}", buf[0]);
    
    return buf[0];
}

fn get32(f: &mut stb_vorbis) -> u32
{
    let mut x : u32;
    x = get8(f) as u32;
    x += (get8(f) as u32) << 8;
    x += (get8(f)  as u32) << 16;
    x += (get8(f) as u32) << 24 ;
    x
}

fn getn(z: &mut stb_vorbis, data: &mut [u8], n: i32) -> bool
{    
    unsafe {
        if z.stream.is_null() == false {
            if z.stream >= z.stream_end { z.eof = true; return false; }
            ptr::copy_nonoverlapping(z.stream, data.as_mut_ptr(), n as usize);
            z.stream = z.stream.offset(n as isize);
            return true;
        }
    }
    
    let n : usize = n as usize;
    match z.f {
        None => return false,
        Some(ref mut f) => {
            match f.read(&mut data[..n]){
                Err(_) => return false,
                Ok(_) => {
                    z.eof = true;
                    return true;
                }
            }
        }
    }
}

fn skip(z: &mut stb_vorbis, n: i32)
{
    unsafe {
        if z.stream.is_null() == false {
            z.stream = z.stream.offset(n as isize);
            if z.stream >= z.stream_end { z.eof = true; }
            return;
        }
    }
    
    if let Some(ref mut f) = z.f {
        use std::io::SeekFrom;
        f.seek(SeekFrom::Current(n as i64)).unwrap();
    }    
}


fn next_segment(f: &mut stb_vorbis) -> i32
{
    // unreachable!();
//    int len;
   if f.last_seg {return 0;}
   if f.next_seg == -1 {
      f.last_seg_which = f.segment_count-1; // in case start_page fails
      if !start_page(f) {
        f.last_seg = true;
        return 0;
      }
      if (f.page_flag & PAGEFLAG_continued_packet) == 0 {
          error(f, STBVorbisError::VORBIS_continued_packet_flag_invalid);
          return 0;
      }
   }

   let len = f.segments[f.next_seg as usize];
   f.next_seg += 1;
   
   if len < 255 {
      f.last_seg = true;
      f.last_seg_which = f.next_seg-1;
   }
   if f.next_seg >= f.segment_count {
      f.next_seg = -1;
   }
   
   assert!(f.bytes_in_seg == 0);
   
   f.bytes_in_seg = len;
   return len as i32;
}

const EOP : i32 = (-1);
const INVALID_BITS : i32 = (-1);

fn get8_packet_raw(f: &mut stb_vorbis) -> i32
{
   if f.bytes_in_seg == 0 {  // CLANG!
      if f.last_seg {
        return EOP;  
      } else if next_segment(f) == 0 {
          return EOP;
      }
   }
   
   debug_assert!(f.bytes_in_seg > 0);
   
   f.bytes_in_seg -= 1;
   f.packet_bytes += 1;
   
   return get8(f) as i32;
}

fn get8_packet(f: &mut stb_vorbis) -> i32
{
   let x = get8_packet_raw(f);
   f.valid_bits = 0;
   return x;
}

fn flush_packet(f: &mut stb_vorbis)
{
   while get8_packet_raw(f) != EOP {}
}

// @OPTIMIZE: this is the secondary bit decoder, so it's probably not as important
// as the huffman decoder?
fn get_bits(f: &mut stb_vorbis, n: i32) -> u32
{
   let mut z : u32;

   if f.valid_bits < 0 { return 0};
   if f.valid_bits < n {
      if n > 24 {
         // the accumulator technique below would not work correctly in this case
         z = get_bits(f, 24);
         z += get_bits(f, n-24) << 24;
         return z;
      }
      if f.valid_bits == 0 {
          f.acc = 0;
      } 
      while f.valid_bits < n {
         let z = get8_packet_raw(f);
         if z == EOP {
            f.valid_bits = INVALID_BITS;
            return 0;
         }
         f.acc += (z << f.valid_bits) as u32;
         f.valid_bits += 8;
      }
   }
   if f.valid_bits < 0 { return 0; }
   z = f.acc & ((1 << n)-1);
   f.acc >>= n;
   f.valid_bits -= n;
   return z;
}


fn capture_pattern(f: &mut stb_vorbis) -> bool
{
    // for debugging....
    // let pattern : [u8; 4] = [0x4f, 0x67, 0x67, 0x53];
    // for &i in pattern.iter() {
    //     let ch = get8(f);
    //     if ch != i {
    //         // println!("capture_pattern(): wrong, {:#x} != {:#x}", ch, i);
    //         return false;
    //     }
    // }
   if 0x4f != get8(f) {return false};
   if 0x67 != get8(f) {return false};
   if 0x67 != get8(f) {return false};
   if 0x53 != get8(f) {return false};
    // println!("capture_pattern(): true");
   return true;
}

const PAGEFLAG_continued_packet : u8 = 1;
const PAGEFLAG_first_page       : u8 = 2;
const PAGEFLAG_last_page        : u8 = 4;

const   VORBIS_packet_id : u8 = 1;
const   VORBIS_packet_comment : u8 = 3;
const   VORBIS_packet_setup : u8 = 5;


// if the fast table above doesn't work, we want to binary
// search them... need to reverse the bits
fn compute_sorted_huffman(c: &mut Codebook, lengths: *const u8, values: &[u32])
{
    unimplemented!();
//    int i, len;
//    // build a list of all the entries
//    // OPTIMIZATION: don't include the short ones, since they'll be caught by FAST_HUFFMAN.
//    // this is kind of a frivolous optimization--I don't see any performance improvement,
//    // but it's like 4 extra lines of code, so.
//    if (!c->sparse) {
//       int k = 0;
//       for (i=0; i < c->entries; ++i)
//          if (include_in_sort(c, lengths[i])) 
//             c->sorted_codewords[k++] = bit_reverse(c->codewords[i]);
//       assert(k == c->sorted_entries);
//    } else {
//       for (i=0; i < c->sorted_entries; ++i)
//          c->sorted_codewords[i] = bit_reverse(c->codewords[i]);
//    }

//    qsort(c->sorted_codewords, c->sorted_entries, sizeof(c->sorted_codewords[0]), uint32_compare);
//    c->sorted_codewords[c->sorted_entries] = 0xffffffff;

//    len = c->sparse ? c->sorted_entries : c->entries;
//    // now we need to indicate how they correspond; we could either
//    //   #1: sort a different data structure that says who they correspond to
//    //   #2: for each sorted entry, search the original list to find who corresponds
//    //   #3: for each original entry, find the sorted entry
//    // #1 requires extra storage, #2 is slow, #3 can use binary search!
//    for (i=0; i < len; ++i) {
//       int huff_len = c->sparse ? lengths[values[i]] : lengths[i];
//       if (include_in_sort(c,huff_len)) {
//          uint32 code = bit_reverse(c->codewords[i]);
//          int x=0, n=c->sorted_entries;
//          while (n > 1) {
//             // invariant: sc[x] <= code < sc[x+n]
//             int m = x + (n >> 1);
//             if (c->sorted_codewords[m] <= code) {
//                x = m;
//                n -= (n>>1);
//             } else {
//                n >>= 1;
//             }
//          }
//          assert(c->sorted_codewords[x] == code);
//          if (c->sparse) {
//             c->sorted_values[x] = values[i];
//             c->codeword_lengths[x] = huff_len;
//          } else {
//             c->sorted_values[x] = i;
//          }
//       }
//    }
}


// only run while parsing the header (3 times)
fn vorbis_validate(data: &[u8]) -> bool
{
    return data[0..6] == [b'v', b'o', b'r', b'b', b'i', b's'];
}

// called from setup only, once per code book
// (formula implied by specification)
fn lookup1_values(entries: i32, dim: i32) -> i32
{
   let mut r : i32 = f64::floor(f64::exp(f64::ln(entries as f64) / dim as f64)) as i32;
   if f64::floor(f64::powi((r+1) as f64, dim)) as i32 <= entries{
      r += 1; // floor() to avoid _ftol() when non-CRT
   }   // (int) cast for MinGW warning;

   assert!(f64::powi((r+1) as f64, dim) as i32 > entries);
   assert!(f64::floor(f64::powi(r as f64, dim)) as i32 <= entries); // (int),floor() as above
   return r;
}


fn start_page_no_capturepattern(f: &mut stb_vorbis) -> bool {
    use STBVorbisError::*;

   let loc0: u32;
   let loc1: u32;
   let n: u32;
   
   // stream structure version
   if 0 != get8(f) {
       return error(f, VORBIS_invalid_stream_structure_version);
   }
   
   // header flag
   f.page_flag = get8(f);
   // absolute granule position
   loc0 = get32(f); 
   loc1 = get32(f);
   // @TODO: validate loc0,loc1 as valid positions?
   // stream serial number -- vorbis doesn't interleave, so discard
   get32(f);
   //if (f.serial != get32(f)) return error(f, VORBIS_incorrect_stream_serial_number);
   // page sequence number
   n = get32(f);
   f.last_page = n as i32; // NOTE: maybe change f.last_page to u32?
   // CRC32
   get32(f);
   // page_segments
   f.segment_count = get8(f) as i32; // NOTE: maybe change f.last_page to u8?
   {
        let segment_count = f.segment_count;
        let mut segments: [u8; 255] = [0; 255];
        if !getn(f, &mut segments, segment_count){
            return error(f, VORBIS_unexpected_eof);
        }
        f.segments.copy_from_slice(&segments);
   }
   
   
   // assume we _don't_ know any the sample position of any segments
   f.end_seg_with_known_loc = -2;
   if loc0 != !0 || loc1 != !0 {
      // determine which packet is the last one that will complete
      let mut i: i32 = f.segment_count-1;
      while i >= 0 {
          if f.segments[i as usize] < 255{
              break;
          }          
          i -= 1;
      }
      
      // 'i' is now the index of the _last_ segment of a packet that ends
      if i >= 0 {
         f.end_seg_with_known_loc = i;
         f.known_loc_for_packet   = loc0;
      }
   }
   
   if f.first_decode != 0 {
      let mut len: u32 = 0;
      for i in 0 .. f.segment_count {
          len += f.segments[i as usize] as u32;
      }
      len += 27u32 + f.segment_count as u32;
      f.p_first = ProbedPage {
          page_start: f.first_audio_page_offset,
          page_end: f.first_audio_page_offset + len as u32,
          last_decoded_sample: loc0
      };
   }
   f.next_seg = 0;
   return true;
}


fn start_page(f: &mut stb_vorbis) -> bool
{
   if !capture_pattern(f) {
       return error(f, STBVorbisError::VORBIS_missing_capture_pattern);
   }
   return start_page_no_capturepattern(f);
}

fn start_packet(f: &mut stb_vorbis) -> bool
{
   while f.next_seg == -1 {
      if !start_page(f) {return false;}
      if (f.page_flag & PAGEFLAG_continued_packet) != 0{
         return error(f, STBVorbisError::VORBIS_continued_packet_flag_invalid);
      }
   }
   f.last_seg = false;
   f.valid_bits = 0;
   f.packet_bytes = 0;
   f.bytes_in_seg = 0;
   // f.next_seg is now valid
   return true;
}

fn is_whole_packet_present(f: &mut stb_vorbis, end_page: bool) -> bool
{
   // make sure that we have the packet available before continuing...
   // this requires a full ogg parse, but we know we can fetch from f->stream

   // instead of coding this out explicitly, we could save the current read state,
   // read the next packet with get8() until end-of-packet, check f->eof, then
   // reset the state? but that would be slower, esp. since we'd have over 256 bytes
   // of state to restore (primarily the page segment table)

   unimplemented!();

//    int s = f->next_seg, first = TRUE;
//    uint8 *p = f->stream;

//    if (s != -1) { // if we're not starting the packet with a 'continue on next page' flag
//       for (; s < f->segment_count; ++s) {
//          p += f->segments[s];
//          if (f->segments[s] < 255)               // stop at first short segment
//             break;
//       }
//       // either this continues, or it ends it...
//       if (end_page)
//          if (s < f->segment_count-1)             return error(f, VORBIS_invalid_stream);
//       if (s == f->segment_count)
//          s = -1; // set 'crosses page' flag
//       if (p > f->stream_end)                     return error(f, VORBIS_need_more_data);
//       first = FALSE;
//    }
//    for (; s == -1;) {
//       uint8 *q; 
//       int n;

//       // check that we have the page header ready
//       if (p + 26 >= f->stream_end)               return error(f, VORBIS_need_more_data);
//       // validate the page
//       if (memcmp(p, ogg_page_header, 4))         return error(f, VORBIS_invalid_stream);
//       if (p[4] != 0)                             return error(f, VORBIS_invalid_stream);
//       if (first) { // the first segment must NOT have 'continued_packet', later ones MUST
//          if (f->previous_length)
//             if ((p[5] & PAGEFLAG_continued_packet))  return error(f, VORBIS_invalid_stream);
//          // if no previous length, we're resynching, so we can come in on a continued-packet,
//          // which we'll just drop
//       } else {
//          if (!(p[5] & PAGEFLAG_continued_packet)) return error(f, VORBIS_invalid_stream);
//       }
//       n = p[26]; // segment counts
//       q = p+27;  // q points to segment table
//       p = q + n; // advance past header
//       // make sure we've read the segment table
//       if (p > f->stream_end)                     return error(f, VORBIS_need_more_data);
//       for (s=0; s < n; ++s) {
//          p += q[s];
//          if (q[s] < 255)
//             break;
//       }
//       if (end_page)
//          if (s < n-1)                            return error(f, VORBIS_invalid_stream);
//       if (s == n)
//          s = -1; // set 'crosses page' flag
//       if (p > f->stream_end)                     return error(f, VORBIS_need_more_data);
//       first = FALSE;
//    }
//    return TRUE;
}


fn start_decoder(f: &mut stb_vorbis) -> bool {
    use STBVorbisError::*;
    
    let mut max_submaps : i32 = 0;
    let mut longest_floorlist : i32 =0;

   // first page, first packet
    
    if !start_page(f) {
        return false;
    }
    
    let mut header : [u8; 6] = [0u8; 6];
    
   // validate page flag
   if (f.page_flag & PAGEFLAG_first_page) == 0       {return error(f, VORBIS_invalid_first_page);}
   if (f.page_flag & PAGEFLAG_last_page) != 0          {return error(f, VORBIS_invalid_first_page);}
   if (f.page_flag & PAGEFLAG_continued_packet) != 0    {return error(f, VORBIS_invalid_first_page);}
   // check for expected packet length
   if f.segment_count != 1                       {return error(f, VORBIS_invalid_first_page);}
   if f.segments[0] != 30                        {return error(f, VORBIS_invalid_first_page);}
   // read packet
   // check packet header
   if get8(f) != VORBIS_packet_id                 {return error(f, VORBIS_invalid_first_page);}
   if !getn(f, &mut header, 6)                {return error(f, VORBIS_unexpected_eof);}
   if !vorbis_validate(&header)                    {return error(f, VORBIS_invalid_first_page);}
   // vorbis_version
   if get32(f) != 0                               {return error(f, VORBIS_invalid_first_page);}
   f.channels = get8(f) as i32; // FIXME: current channel is i32
   if f.channels == 0       {return error(f, VORBIS_invalid_first_page);}
   if f.channels > STB_VORBIS_MAX_CHANNELS as i32       {return error(f, VORBIS_too_many_channels);}
   f.sample_rate = get32(f); 
   if f.sample_rate == 0{return error(f, VORBIS_invalid_first_page);}
   get32(f); // bitrate_maximum
   get32(f); // bitrate_nominal
   get32(f); // bitrate_minimum

   let x = get8(f) as i32;
    
   {
      let log0: i32 = x & 15;
      let log1: i32 = x >> 4;
      f.blocksize_0 = 1 << log0;
      f.blocksize_1 = 1 << log1;
      if log0 < 6 || log0 > 13   {return error(f, VORBIS_invalid_setup);}
      if log1 < 6 || log1 > 13   {return error(f, VORBIS_invalid_setup);}
      if log0 > log1             {return error(f, VORBIS_invalid_setup);}
   };
   
   // framing_flag
   let x = get8(f);
   if (x & 1) == 0 { return error(f, VORBIS_invalid_first_page);}
   
   // second packet!
   if !start_page(f){ return false;}

   if !start_packet(f) {return false;}
   
   let mut len;
   // do-while
   while {
      len = next_segment(f);
      skip(f, len);
      f.bytes_in_seg = 0;
      
      len != 0
   }{}
   
   // third packet!
   if !start_packet(f){return false;}
   if f.push_mode {
      if !is_whole_packet_present(f, true) {
         // convert error in ogg header to write type
         if f.error == VORBIS_invalid_stream {
            f.error = VORBIS_invalid_setup;
         }
         return false;
      }
   }
   
   crc32_init(); // always init it, to avoid multithread race conditions

   if get8_packet(f) != VORBIS_packet_setup as i32 {
       return error(f, VORBIS_invalid_setup);
   }
   for i in 0 .. 6 {
     header[i] = get8_packet(f) as u8;  
   } 
   if !vorbis_validate(&header){
       return error(f, VORBIS_invalid_setup);
   }                    

   // codebooks

   f.codebook_count = (get_bits(f,8) + 1) as i32;
   f.codebooks.resize(f.codebook_count as usize, Codebook::default());
//    memset(f->codebooks, 0, sizeof(*f->codebooks) * f->codebook_count);
   
   for i in 0 .. f.codebook_count {
    //   uint32 *values;
    //   int ordered, sorted_count;
    //   int total=0;
    let mut total = 0;
    
    //   Codebook *c = f->codebooks+i;
    
      CHECK!(f);
      let mut c : Codebook = Codebook::default();
    // let c = &mut f.codebooks[i as usize];
      
    //   println!("sinikah");
      let x = get_bits(f, 8); if x != 0x42            {return error(f, VORBIS_invalid_setup);}
      let x = get_bits(f, 8); if x != 0x43            {return error(f, VORBIS_invalid_setup);}
      let x = get_bits(f, 8); if x != 0x56            {return error(f, VORBIS_invalid_setup);}
      let x = get_bits(f, 8);
    //   println!("lol");
      
      c.dimensions = ((get_bits(f, 8)<<8) + x) as i32;
      let x = get_bits(f, 8);
      let y = get_bits(f, 8);
      c.entries = ((get_bits(f, 8)<<16) + (y<<8) + x) as i32;
      let ordered = get_bits(f,1);
      c.sparse = if ordered != 0 { 
          false
       }else{
          get_bits(f,1) != 0
       };

      
      if c.dimensions == 0 && c.entries != 0    {return error(f, VORBIS_invalid_setup);}

    //   uint8 *lengths;
      let mut length_in_stacks : Vec<u8> = Vec::new();
      let lengths: *mut u8;
      if c.sparse {
          length_in_stacks.resize(c.entries as usize, 0);
          lengths = length_in_stacks.as_mut_slice().as_mut_ptr();
      }else{
         c.codeword_lengths.resize(c.entries as usize, 0);
         lengths = c.codeword_lengths.as_mut_slice().as_mut_ptr();
      }

      // NOTE: rust dont have OOM
    //   if (!lengths) return error(f, VORBIS_outofmem);

      if ordered != 0 {
         let mut current_entry = 0;
         let mut current_length = get_bits(f,5) + 1;
         while current_entry < c.entries {
            let limit = c.entries - current_entry;
            let n = get_bits(f, ilog(limit));
            if current_entry + n as i32 > c.entries as i32 { return error(f, VORBIS_invalid_setup); }
            
            unsafe {
                // NOTE: maybe order is wrong??? 
                ptr::write_bytes(lengths.offset(current_entry as isize), current_length as u8, n as usize);
                // memset(lengths + current_entry, current_length, n);
                
            }
            
            current_entry += n as i32;
            current_length += 1 ;
         }
      } else {
         for j in 0 .. c.entries {
             let j = j as isize;
             
            let present = if c.sparse  { get_bits(f,1) } else {1 } ;
            if present != 0 {
                unsafe {
                   *lengths.offset(j) = (get_bits(f, 5) + 1) as u8;
                    total += 1;
                    if *lengths.offset(j) == 32 {
                        return error(f, VORBIS_invalid_setup);
                    }
                }
            } else {
                unsafe {
                   *lengths.offset(j) = NO_CODE;
                    
                }
            }
         }
      }
      
      if c.sparse  && (total >= c.entries >> 2) {
         // convert sparse items to non-sparse!
         if c.entries > f.setup_temp_memory_required as i32 {
            f.setup_temp_memory_required = c.entries as u32;
         }

         c.codeword_lengths.resize(c.entries as usize, 0);
         c.sparse = false;

        //  c.codeword_lengths = (uint8 *) setup_malloc(f, c.entries);
        //  if (c.codeword_lengths == NULL) return error(f, VORBIS_outofmem);
        //  memcpy(c.codeword_lengths, lengths, c.entries);
        //  setup_temp_free(f, lengths, c.entries); // note this is only safe if there have been no intervening temp mallocs!
        //  lengths = c.codeword_lengths;
        //  c.sparse = 0;
      }

      // compute the size of the sorted tables
      let mut sorted_count : i32;
      if c.sparse  {
         sorted_count = total;
      } else {
         sorted_count = 0;
        //  #ifndef STB_VORBIS_NO_HUFFMAN_BINARY_SEARCH
        for j in 0 .. c.entries {
            unsafe {
                let l = *lengths.offset(j as isize);
                if l > STB_VORBIS_FAST_HUFFMAN_LENGTH && l != NO_CODE {
                    sorted_count += 1;
                }
            }
        }
        //  #endif
      }

      c.sorted_entries = sorted_count;
      let mut values : Vec<u32> = Vec::new();

      CHECK!(f);
      if c.sparse == false {
          c.codewords.resize(c.entries as usize, 0);
         // NOTE: rust don't have OOM
        //  if (!c.codewords)                  return error(f, VORBIS_outofmem);
      } else {
         if c.sorted_entries != 0 {
             c.codeword_lengths.resize(c.sorted_entries as usize, 0);
            // if (!c.codeword_lengths)           return error(f, VORBIS_outofmem);
            c.codewords.resize(c.sorted_entries as usize, 0);
            // c.codewords = (uint32 *) setup_temp_malloc(f, sizeof(*c.codewords) * c.sorted_entries);
            // if (!c.codewords)                  return error(f, VORBIS_outofmem);
            values = vec![0; c.sorted_entries as usize];
            // values = (uint32 *) setup_temp_malloc(f, sizeof(*values) * c.sorted_entries);
            // if (!values)                        return error(f, VORBIS_outofmem);
         }
         
         let size = c.entries as usize + (mem::size_of::<u32>() + mem::size_of::<u32>()) * c.sorted_entries as usize;
         if (size > f.setup_temp_memory_required as usize){
            f.setup_temp_memory_required = size as u32;
         }
      }

      let c_entries = c.entries;
      if !compute_codewords(&mut c, lengths, c_entries, &mut values) {
         if c.sparse {
            //  setup_temp_free(f, values, 0);
         }
         return error(f, VORBIS_invalid_setup);
      }

      if c.sorted_entries != 0 {
         // allocate an extra slot for sentinels
         c.sorted_codewords.resize(c.sorted_entries as usize + 1, 0);
        
         // allocate an extra slot at the front so that c.sorted_values[-1] is defined
         // so that we can catch that case without an extra if
         c.sorted_values.resize( c.sorted_entries as usize + 1, 0);
         
         // NOTE: index -1 pindah ke terakhir...., cek nanti apakah bisa seperti ini atau tidak
         c.sorted_values[ c.sorted_entries as usize] = -1;
        //  ++c.sorted_values;
        //  c.sorted_values[-1] = -1;
         compute_sorted_huffman(&mut c, lengths, &values);
      }

      if c.sparse {
          // NOTE: no need 
        //  setup_temp_free(f, values, sizeof(*values)*c.sorted_entries);
        //  setup_temp_free(f, c.codewords, sizeof(*c.codewords)*c.sorted_entries);
        //  setup_temp_free(f, lengths, c.entries);
        //  c.codewords = NULL;
        c.codewords.clear();
      }

      compute_accelerated_huffman(&mut c);
      
      CHECK!(f);
      c.lookup_type = get_bits(f, 4) as u8;
      if c.lookup_type > 2 {return error(f, VORBIS_invalid_setup);}
      
      if c.lookup_type > 0 {
//          uint16 *mults;
         c.minimum_value = float32_unpack(get_bits(f, 32));
         c.delta_value = float32_unpack(get_bits(f, 32));
         c.value_bits = (get_bits(f, 4)+1) as u8;
         c.sequence_p = get_bits(f,1) as u8;
         if c.lookup_type == 1 {
            c.lookup_values = lookup1_values(c.entries, c.dimensions) as u32;
         } else {
            c.lookup_values = (c.entries * c.dimensions) as u32;
         }
         
         if c.lookup_values == 0 { return error(f, VORBIS_invalid_setup); }
         
         let mut mults : Vec<u16> =  vec![0; c.lookup_values as usize];
//          if (mults == NULL) return error(f, VORBIS_outofmem);
         for j in 0 .. c.lookup_values {
             let q = get_bits(f, c.value_bits as i32) as i32;
             if q == EOP {
                // setup_temp_free(f,mults,sizeof(mults[0])*c.lookup_values); 
                return error(f, VORBIS_invalid_setup);                  
             }
             mults[j as usize] = q as u16;
         }

// #ifndef STB_VORBIS_DIVIDES_IN_CODEBOOK
        'outer: loop{
         if c.lookup_type == 1 {
            let len : i32;
            let sparse = c.sparse;
            let mut last : f32 = 0.0;
            
            // pre-expand the lookup1-style multiplicands, to avoid a divide in the inner loop
            if sparse {
               if (c.sorted_entries == 0){
                   break 'outer;
               }
               c.multiplicands.resize((c.sorted_entries * c.dimensions) as usize, 0.0);
            } else{
               c.multiplicands.resize((c.entries        * c.dimensions) as usize, 0.0);
            }
            // if c.multiplicands == NULL { setup_temp_free(f,mults,sizeof(mults[0])*c.lookup_values); return error(f, VORBIS_outofmem); }
            len = if sparse { c.sorted_entries } else { c.entries } ;
            
            for j in 0 .. len {
               let z = if sparse {c.sorted_values[j as usize]}else{j} ;
               let mut div = 1;
               for k in 0 .. c.dimensions {
                  let off : usize = ((z / div) % c.lookup_values as i32) as usize;
                  let val : f32 = mults[off] as f32;
                  let val : f32 = mults[off] as f32 * c.delta_value + c.minimum_value + last;
                  c.multiplicands[(j*c.dimensions + k) as usize] = val;
                  if c.sequence_p != 0{
                     last = val;
                  }
                  if k+1 < c.dimensions {
                     if div > (u32::max_value() / c.lookup_values) as i32 {
                        // setup_temp_free(f, mults,sizeof(mults[0])*c.lookup_values);
                        return error(f, VORBIS_invalid_setup);
                     }
                     div *= c.lookup_values as i32;
                  }
               }
            }
            c.lookup_type = 2;
         }
         else
// #endif
         {
            let mut last : f32 = 0.0;
            CHECK!(f);
            c.multiplicands.resize(c.lookup_values as usize, 0.0);
            // c.multiplicands = (codetype *) setup_malloc(f, sizeof(c.multiplicands[0]) * c.lookup_values);
            // if (c.multiplicands == NULL) { setup_temp_free(f, mults,sizeof(mults[0])*c.lookup_values); return error(f, VORBIS_outofmem); }
            
            for j in 0 .. c.lookup_values {
               let val : f32 = mults[j as usize] as f32 * c.delta_value as f32 + c.minimum_value as f32 + last as f32;
               c.multiplicands[j as usize] = val;
               if c.sequence_p != 0{
                  last = val;
               }
            }
         }
// #ifndef STB_VORBIS_DIVIDES_IN_CODEBOOK
        } // NOTE: replaced with labeled break
// #endif
//          setup_temp_free(f, mults, sizeof(mults[0])*c.lookup_values);

         CHECK!(f);
      }
      CHECK!(f);

        // NOTE: tambahan kode
        f.codebooks[i as usize] = c.clone();
   }

   // time domain transfers (notused)

   let x = get_bits(f, 6) + 1;
   for i in 0 .. x {
      let z = get_bits(f, 16);
      if z != 0 {
          return error(f, VORBIS_invalid_setup);
      }
   }


   // Floors
   f.floor_count = (get_bits(f, 6)+1) as i32;
   f.floor_config = vec![Floor::default(); f.floor_count as usize];
//    if (f.floor_config == NULL) return error(f, VORBIS_outofmem);
   for i in 0 .. f.floor_count {
       let i = i as usize;
       
      f.floor_types[i] = get_bits(f, 16) as i32;
      if f.floor_types[i] > 1 {return error(f, VORBIS_invalid_setup);}
      if f.floor_types[i] == 0 {
          {
            // let mut g = &mut f.floor_config[i].floor0;
            f.floor_config[i].floor0.order = get_bits(f,8) as u8;
            f.floor_config[i].floor0.rate = get_bits(f,16) as u16;
            f.floor_config[i].floor0.bark_map_size = get_bits(f,16) as u16;
            f.floor_config[i].floor0.amplitude_bits = get_bits(f,6) as u8;
            f.floor_config[i].floor0.amplitude_offset = get_bits(f,8) as u8;
            f.floor_config[i].floor0.number_of_books = (get_bits(f,4) + 1) as u8;
            for j in 0 .. f.floor_config[i].floor0.number_of_books {
                f.floor_config[i].floor0.book_list[j as usize] = get_bits(f,8) as u8;
            }
          }
         return error(f, VORBIS_feature_not_supported);
      } else {
         let mut p : [Point; 31*8+2] = [Point{x:0, y:0}; 31*8+2];
        //  let g = &f.floor_config[i].floor1;
         let mut max_class : i32 = -1; 
         f.floor_config[i].floor1.partitions = get_bits(f, 5) as u8;
         for j in 0 .. f.floor_config[i].floor1.partitions {
             let j = j as usize;
            f.floor_config[i].floor1.partition_class_list[j] = get_bits(f, 4) as u8;
            if f.floor_config[i].floor1.partition_class_list[j] as i32 > max_class{
               max_class = f.floor_config[i].floor1.partition_class_list[j] as i32;
            }
         }
         for j in 0 .. max_class {
             let j = j as usize;
             
            f.floor_config[i].floor1.class_dimensions[j] = (get_bits(f, 3)+1) as u8;
            f.floor_config[i].floor1.class_subclasses[j] = get_bits(f, 2) as u8;
            if f.floor_config[i].floor1.class_subclasses[j] != 0 {
               f.floor_config[i].floor1.class_masterbooks[j] = get_bits(f, 8) as u8;
               if f.floor_config[i].floor1.class_masterbooks[j] as i32 >= f.codebook_count{
                   return error(f, VORBIS_invalid_setup);
               }
            }
            for k in 0 .. (1 << f.floor_config[i].floor1.class_subclasses[j]) {
                let k = k as usize;
               f.floor_config[i].floor1.subclass_books[j][k] = (get_bits(f,8)-1) as i16;
               if f.floor_config[i].floor1.subclass_books[j][k] >= f.codebook_count as i16 {
                   return error(f, VORBIS_invalid_setup);
               }
            }
         }
         
         f.floor_config[i].floor1.floor1_multiplier = (get_bits(f,2)+1) as u8;
         f.floor_config[i].floor1.rangebits = get_bits(f,4) as u8;
         f.floor_config[i].floor1.Xlist[0] = 0;
         f.floor_config[i].floor1.Xlist[1] = 1 << f.floor_config[i].floor1.rangebits;
         f.floor_config[i].floor1.values = 2;
         for j in 0 .. f.floor_config[i].floor1.partitions {
             let j = j as usize;
            let c = f.floor_config[i].floor1.partition_class_list[j] as usize;
            for k in 0 .. f.floor_config[i].floor1.class_dimensions[c] {
                let index = f.floor_config[i].floor1.values as usize;
                let rangebits = f.floor_config[i].floor1.rangebits as i32;
               f.floor_config[i].floor1.Xlist[index] = get_bits(f, rangebits) as u16;
               
               f.floor_config[i].floor1.values += 1;
            }
         }
         // precompute the sorting
         for j in 0 .. f.floor_config[i].floor1.values {
             let j = j as usize;
            p[j].x = f.floor_config[i].floor1.Xlist[j];
            p[j].y = j as u16;
         }
         
         {
             let mut p = &mut p[0 .. f.floor_config[i].floor1.values as usize];
             p.sort();
         }
         
         for j in 0 .. f.floor_config[i].floor1.values {
             let j = j as usize;
            f.floor_config[i].floor1.sorted_order[j] = p[j].y as u8;
         }
         // precompute the neighbors
         for j in 2 .. f.floor_config[i].floor1.values {
            let mut low : i32 = 0;
            let mut hi : i32 = 0;
            neighbors(&mut f.floor_config[i].floor1.Xlist, j, &mut low, &mut hi);

            let j = j as usize;
            f.floor_config[i].floor1.neighbors[j][0] = low as u8;
            f.floor_config[i].floor1.neighbors[j][1] = hi as u8;
         }

         if (f.floor_config[i].floor1.values > longest_floorlist){
            longest_floorlist = f.floor_config[i].floor1.values;
         }
      }
   }

   // Residue
   f.residue_count = (get_bits(f, 6)+1) as i32;
   f.residue_config.resize(f.residue_count as usize, Residue::default());
//    if (f.residue_config == NULL) return error(f, VORBIS_outofmem);
//    memset(f.residue_config, 0, f.residue_count * sizeof(f.residue_config[0]));
   for i in 0 .. f.residue_count {
//       uint8 residue_cascade[64];
//       Residue *r = f.residue_config+i;
//       f.residue_types[i] = get_bits(f, 16);
//       if (f.residue_types[i] > 2) return error(f, VORBIS_invalid_setup);
//       r.begin = get_bits(f, 24);
//       r.end = get_bits(f, 24);
//       if (r.end < r.begin) return error(f, VORBIS_invalid_setup);
//       r.part_size = get_bits(f,24)+1;
//       r.classifications = get_bits(f,6)+1;
//       r.classbook = get_bits(f,8);
//       if (r.classbook >= f.codebook_count) return error(f, VORBIS_invalid_setup);
//       for (j=0; j < r.classifications; ++j) {
//          uint8 high_bits=0;
//          uint8 low_bits=get_bits(f,3);
//          if (get_bits(f,1))
//             high_bits = get_bits(f,5);
//          residue_cascade[j] = high_bits*8 + low_bits;
//       }
//       r.residue_books = (short (*)[8]) setup_malloc(f, sizeof(r.residue_books[0]) * r.classifications);
//       if (r.residue_books == NULL) return error(f, VORBIS_outofmem);
//       for (j=0; j < r.classifications; ++j) {
//          for (k=0; k < 8; ++k) {
//             if (residue_cascade[j] & (1 << k)) {
//                r.residue_books[j][k] = get_bits(f, 8);
//                if (r.residue_books[j][k] >= f.codebook_count) return error(f, VORBIS_invalid_setup);
//             } else {
//                r.residue_books[j][k] = -1;
//             }
//          }
//       }
//       // precompute the classifications[] array to avoid inner-loop mod/divide
//       // call it 'classdata' since we already have r.classifications
//       r.classdata = (uint8 **) setup_malloc(f, sizeof(*r.classdata) * f.codebooks[r.classbook].entries);
//       if (!r.classdata) return error(f, VORBIS_outofmem);
//       memset(r.classdata, 0, sizeof(*r.classdata) * f.codebooks[r.classbook].entries);
//       for (j=0; j < f.codebooks[r.classbook].entries; ++j) {
//          int classwords = f.codebooks[r.classbook].dimensions;
//          int temp = j;
//          r.classdata[j] = (uint8 *) setup_malloc(f, sizeof(r.classdata[j][0]) * classwords);
//          if (r.classdata[j] == NULL) return error(f, VORBIS_outofmem);
//          for (k=classwords-1; k >= 0; --k) {
//             r.classdata[j][k] = temp % r.classifications;
//             temp /= r.classifications;
//          }
//       }
   }

   unimplemented!();
   
}

//    uint8 x,y;
//    int len,i,j,k, max_submaps = 0;
//    int longest_floorlist=0;



//    f->mapping_count = get_bits(f,6)+1;
//    f->mapping = (Mapping *) setup_malloc(f, f->mapping_count * sizeof(*f->mapping));
//    if (f->mapping == NULL) return error(f, VORBIS_outofmem);
//    memset(f->mapping, 0, f->mapping_count * sizeof(*f->mapping));
//    for (i=0; i < f->mapping_count; ++i) {
//       Mapping *m = f->mapping + i;      
//       int mapping_type = get_bits(f,16);
//       if (mapping_type != 0) return error(f, VORBIS_invalid_setup);
//       m->chan = (MappingChannel *) setup_malloc(f, f->channels * sizeof(*m->chan));
//       if (m->chan == NULL) return error(f, VORBIS_outofmem);
//       if (get_bits(f,1))
//          m->submaps = get_bits(f,4)+1;
//       else
//          m->submaps = 1;
//       if (m->submaps > max_submaps)
//          max_submaps = m->submaps;
//       if (get_bits(f,1)) {
//          m->coupling_steps = get_bits(f,8)+1;
//          for (k=0; k < m->coupling_steps; ++k) {
//             m->chan[k].magnitude = get_bits(f, ilog(f->channels-1));
//             m->chan[k].angle = get_bits(f, ilog(f->channels-1));
//             if (m->chan[k].magnitude >= f->channels)        return error(f, VORBIS_invalid_setup);
//             if (m->chan[k].angle     >= f->channels)        return error(f, VORBIS_invalid_setup);
//             if (m->chan[k].magnitude == m->chan[k].angle)   return error(f, VORBIS_invalid_setup);
//          }
//       } else
//          m->coupling_steps = 0;

//       // reserved field
//       if (get_bits(f,2)) return error(f, VORBIS_invalid_setup);
//       if (m->submaps > 1) {
//          for (j=0; j < f->channels; ++j) {
//             m->chan[j].mux = get_bits(f, 4);
//             if (m->chan[j].mux >= m->submaps)                return error(f, VORBIS_invalid_setup);
//          }
//       } else
//          // @SPECIFICATION: this case is missing from the spec
//          for (j=0; j < f->channels; ++j)
//             m->chan[j].mux = 0;

//       for (j=0; j < m->submaps; ++j) {
//          get_bits(f,8); // discard
//          m->submap_floor[j] = get_bits(f,8);
//          m->submap_residue[j] = get_bits(f,8);
//          if (m->submap_floor[j] >= f->floor_count)      return error(f, VORBIS_invalid_setup);
//          if (m->submap_residue[j] >= f->residue_count)  return error(f, VORBIS_invalid_setup);
//       }
//    }

//    // Modes
//    f->mode_count = get_bits(f, 6)+1;
//    for (i=0; i < f->mode_count; ++i) {
//       Mode *m = f->mode_config+i;
//       m->blockflag = get_bits(f,1);
//       m->windowtype = get_bits(f,16);
//       m->transformtype = get_bits(f,16);
//       m->mapping = get_bits(f,8);
//       if (m->windowtype != 0)                 return error(f, VORBIS_invalid_setup);
//       if (m->transformtype != 0)              return error(f, VORBIS_invalid_setup);
//       if (m->mapping >= f->mapping_count)     return error(f, VORBIS_invalid_setup);
//    }

//    flush_packet(f);

//    f->previous_length = 0;

//    for (i=0; i < f->channels; ++i) {
//       f->channel_buffers[i] = (float *) setup_malloc(f, sizeof(float) * f->blocksize_1);
//       f->previous_window[i] = (float *) setup_malloc(f, sizeof(float) * f->blocksize_1/2);
//       f->finalY[i]          = (int16 *) setup_malloc(f, sizeof(int16) * longest_floorlist);
//       if (f->channel_buffers[i] == NULL || f->previous_window[i] == NULL || f->finalY[i] == NULL) return error(f, VORBIS_outofmem);
//       #ifdef STB_VORBIS_NO_DEFER_FLOOR
//       f->floor_buffers[i]   = (float *) setup_malloc(f, sizeof(float) * f->blocksize_1/2);
//       if (f->floor_buffers[i] == NULL) return error(f, VORBIS_outofmem);
//       #endif
//    }

//    if (!init_blocksize(f, 0, f->blocksize_0)) return FALSE;
//    if (!init_blocksize(f, 1, f->blocksize_1)) return FALSE;
//    f->blocksize[0] = f->blocksize_0;
//    f->blocksize[1] = f->blocksize_1;

// #ifdef STB_VORBIS_DIVIDE_TABLE
//    if (integer_divide_table[1][1]==0)
//       for (i=0; i < DIVTAB_NUMER; ++i)
//          for (j=1; j < DIVTAB_DENOM; ++j)
//             integer_divide_table[i][j] = i / j;
// #endif

//    // compute how much temporary memory is needed

//    // 1.
//    {
//       uint32 imdct_mem = (f->blocksize_1 * sizeof(float) >> 1);
//       uint32 classify_mem;
//       int i,max_part_read=0;
//       for (i=0; i < f->residue_count; ++i) {
//          Residue *r = f->residue_config + i;
//          int n_read = r->end - r->begin;
//          int part_read = n_read / r->part_size;
//          if (part_read > max_part_read)
//             max_part_read = part_read;
//       }
//       #ifndef STB_VORBIS_DIVIDES_IN_RESIDUE
//       classify_mem = f->channels * (sizeof(void*) + max_part_read * sizeof(uint8 *));
//       #else
//       classify_mem = f->channels * (sizeof(void*) + max_part_read * sizeof(int *));
//       #endif

//       f->temp_memory_required = classify_mem;
//       if (imdct_mem > f->temp_memory_required)
//          f->temp_memory_required = imdct_mem;
//    }

//    f->first_decode = TRUE;

//    if (f->alloc.alloc_buffer) {
//       assert(f->temp_offset == f->alloc.alloc_buffer_length_in_bytes);
//       // check if there's enough temp memory so we don't error later
//       if (f->setup_offset + sizeof(*f) + f->temp_memory_required > (unsigned) f->temp_offset)
//          return error(f, VORBIS_outofmem);
//    }

//    f->first_audio_page_offset = stb_vorbis_get_file_offset(f);

//    return TRUE;
// }


// FIXME: rename function name to more rust friendly
// FIXME: remove alloc param
// FIXME: change to return stb_vorbis
// FIXME: remove unsafe
// static void vorbis_init(stb_vorbis *p, const stb_vorbis_alloc *z)
fn vorbis_init(z: *const stb_vorbis_alloc) -> stb_vorbis
{
    let mut p : stb_vorbis = stb_vorbis::default();
    p.eof = false;
    p.error = STBVorbisError::VORBIS__no_error;
    p.stream = ptr::null_mut();
    p.codebooks = Vec::new();
    p.page_crc_tests = -1;
    p.close_on_free = false;
    p.f = None;
    
    if z != std::ptr::null() {
        unreachable!();
    //   p.alloc = *z;
    //   p.alloc.alloc_buffer_length_in_bytes = (p.alloc.alloc_buffer_length_in_bytes+3) & !3;
    //   p.temp_offset = p.alloc.alloc_buffer_length_in_bytes;
    }    
    
    p
}

fn vorbis_deinit(p: stb_vorbis){
//    if p.residue_config != ptr::null() {
//        for i in 0.. p.residue_count {
//            unsafe {
//                 let ref r = *p.residue_config.offset(i as isize);
//                 if r.classdata != ptr::null() {
//                     let ref codebook =  *p.codebooks.offset(r.classbook as isize);
//                     for j in 0 .. codebook.entries {
//                         //FIXME: check it again later...
//                         //    drop(r.classdata[j]);
//                     }
//                 }
//                     //FIXME: check it again later...
//                     // drop(r.residue_books);
                
//            }
//        }
//    }
   
//    if p.codebooks != ptr::null() {
//         unsafe {
//             debug_assert!(p.channel_buffers.offset(1) != ptr::null());
//         }
       
//         for i in 0 .. p.codebook_count {
//             unsafe {
//                 let ref c = *p.codebooks.offset(i as isize);
//                 //FIXME: check it again later...
//                 // drop(c.codeword_lengths);
//                 // drop(c.multiplicands);
//                 // drop(c.codewords);
//                 // drop(c.sorted_codewords);
//                 // // c.sorted_values[-1] is the first entry in the array
//                 // if c.sorted_values { 
//                 //     drop(c.sorted_values.offset(-1));
//                 // }
//             }
//         }
//         //FIXME: check it again later...
//         // drop(p.codebooks);
//    }
   
    // unimplemented!();


//    setup_free(p, p.floor_config);
//    setup_free(p, p.residue_config);
//    if (p.mapping) {
//       for (i=0; i < p.mapping_count; ++i)
//          setup_free(p, p.mapping[i].chan);
//       setup_free(p, p.mapping);
//    }
//    CHECK(p);
//    for (i=0; i < p.channels && i < STB_VORBIS_MAX_CHANNELS; ++i) {
//       setup_free(p, p.channel_buffers[i]);
//       setup_free(p, p.previous_window[i]);
//       #ifdef STB_VORBIS_NO_DEFER_FLOOR
//       setup_free(p, p.floor_buffers[i]);
//       #endif
//       setup_free(p, p.finalY[i]);
//    }
//    for (i=0; i < 2; ++i) {
//       setup_free(p, p.A[i]);
//       setup_free(p, p.B[i]);
//       setup_free(p, p.C[i]);
//       setup_free(p, p.window[i]);
//       setup_free(p, p.bit_reverse[i]);
//    }
//    #ifndef STB_VORBIS_NO_STDIO
//    if (p.close_on_free) fclose(p.f);
//    #endif
    
}

// FIXME: remove this?
fn stb_vorbis_close(p: stb_vorbis)
{
   vorbis_deinit(p);
//    setup_free(p,p);
}


fn vorbis_pump_first_frame(f: &mut stb_vorbis)
{
   let mut len : i32 = 0;
   let mut right : i32 = 0;
   let mut left : i32 = 0;
   
   if vorbis_decode_packet(f, &mut len, &mut left, &mut right) > 0 {
      vorbis_finish_frame(f, len, left, right);
   }
}

fn vorbis_decode_packet(f: &mut stb_vorbis, len: &mut i32, p_left: &mut i32, p_right: &mut i32) -> i32
{
    unreachable!();
//    int mode, left_end, right_end;
//    if (!vorbis_decode_initial(f, p_left, &left_end, p_right, &right_end, &mode)) return 0;
//    return vorbis_decode_packet_rest(f, len, f->mode_config + mode, *p_left, left_end, *p_right, right_end, p_left);
}

fn vorbis_finish_frame(f: &mut stb_vorbis, len: i32, left: i32, right: i32) -> i32
{
    unreachable!();
}

// decode the next frame and return the number of *samples* per channel.
// Note that for interleaved data, you pass in the number of shorts (the
// size of your array), but the return value is the number of samples per
// channel, not the total number of samples.
//
// The data is coerced to the number of channels you request according to the
// channel coercion rules (see below). You must pass in the size of your
// buffer(s) so that stb_vorbis will not overwrite the end of the buffer.
// The maximum buffer size needed can be gotten from get_info(); however,
// the Vorbis I specification implies an absolute maximum of 4096 samples
// per channel.

// Channel coercion rules:
//    Let M be the number of channels requested, and N the number of channels present,
//    and Cn be the nth channel; let stereo L be the sum of all L and center channels,
//    and stereo R be the sum of all R and center channels (channel assignment from the
//    vorbis spec).
//        M    N       output
//        1    k      sum(Ck) for all k
//        2    *      stereo L, stereo R
//        k    l      k > l, the first l channels, then 0s
//        k    l      k <= l, the first k channels
//    Note that this is not _good_ surround etc. mixing at all! It's just so
//    you get something useful.


pub fn stb_vorbis_get_frame_short_interleaved(f: &mut stb_vorbis, num_c: i32, buffer: &mut [i16]) -> i32{
//    int len;
   if num_c == 1 {
       return stb_vorbis_get_frame_short(f,num_c, &buffer, buffer.len() as i32);
   }
   
   unreachable!();
   
//    let output: [*const f32];
//    let mut len = stb_vorbis_get_frame_float(f, 0, &mut output);
//    if len {
//       if (len*num_c > num_shorts){
//         len = num_shorts / num_c;  
//       } 
//       convert_channels_short_interleaved(num_c, buffer, f->channels, output, 0, len);
//    }
//    return len;

}

pub fn stb_vorbis_get_frame_short(f: &mut stb_vorbis, num_c: i32, buffer: &&mut [i16], num_samples: i32) -> i32{
    unreachable!();    
}




// create an ogg vorbis decoder from an open FILE *, looking for a stream at
// the _current_ seek point (ftell); the stream will be of length 'len' bytes.
// on failure, returns NULL and sets *error. note that stb_vorbis must "own"
// this stream; if you seek it in between calls to stb_vorbis, it will become
// confused.

// FIXME: rename function name to more rust friendly
// FIXME: remove alloc param
// FIXME: remove error param
pub fn stb_vorbis_open_file_section(mut file: File, close_on_free: bool, error: &mut i32, alloc: *const stb_vorbis_alloc, len: u64) -> Result<stb_vorbis, STBVorbisError>
{
    
    use std::io::SeekFrom;
    
    let mut p = vorbis_init(alloc);
    p.f_start = file.seek(SeekFrom::Current(0)).unwrap();
    p.stream_len = len as u32; // FIXME: check if convertion is right or not...
    p.close_on_free = close_on_free;

    p.f = Some(file);
    
    println!("soto?");
    if start_decoder(&mut p) {
        println!("bakso?");
        vorbis_pump_first_frame(&mut p);
        println!("mie ayam?");
        return Ok(p);
    }
    println!("sini lancar?");
    
    
    let e = p.error;
    
    *error = p.error as i32;
    vorbis_deinit(p);
    
    return Err(e);
}


// create an ogg vorbis decoder from an open FILE *, looking for a stream at
// the _current_ seek point (ftell). on failure, returns NULL and sets *error.
// note that stb_vorbis must "own" this stream; if you seek it in between
// calls to stb_vorbis, it will become confused. Morever, if you attempt to
// perform stb_vorbis_seek_*() operations on this file, it will assume it
// owns the _entire_ rest of the file after the start point. Use the next
// function, stb_vorbis_open_file_section(), to limit it.

// FIXME: rename function name to more rust friendly
// FIXME: remove alloc param
// FIXME: remove error param
pub fn stb_vorbis_open_file(mut file: File, close_on_free: bool, error: &mut i32, alloc: *const stb_vorbis_alloc) -> Result<stb_vorbis, STBVorbisError>
{
    use std::io::SeekFrom;

    let start = file.seek(SeekFrom::Current(0)).unwrap();
    let len = file.seek(SeekFrom::End(0)).unwrap() - start;
    
    file.seek(SeekFrom::Start(start)).unwrap();
    
    return stb_vorbis_open_file_section(file, close_on_free, error, alloc, len);
}


// create an ogg vorbis decoder from a filename via fopen(). on failure,
// returns NULL and sets *error (possibly to VORBIS_file_open_failure).

// FIXME: rename function name to more rust friendly
// FIXME: remove alloc param
// FIXME: remove error param
pub fn stb_vorbis_open_filename(filename: &Path, error: &mut i32, alloc: *const stb_vorbis_alloc) -> Result<stb_vorbis, STBVorbisError> {
    let f = match File::open(filename){
        Err(_) => {
            return Err(STBVorbisError::VORBIS_file_open_failure)
        },
        Ok(f) => f
    };
    
    return stb_vorbis_open_file(f, true, error, alloc);    
}

// decode an entire file and output the data interleaved into a malloc()ed
// buffer stored in *output. The return value is the number of samples
// decoded, or -1 if the file could not be opened or was not an ogg vorbis file.
// When you're done with it, just free() the pointer returned in *output.

// FIXME: rename function name to more rust friendly
// FIXME: use u32 for param
// NOTE: different from stb_vorbis c, sample_rate is required
pub fn stb_vorbis_decode_filename(filename: &Path, channels: &mut i32, sample_rate: &mut u32) -> Result<Vec<i16>, STBVorbisError>
{
    let mut error: i32 = 0;
    
    let v = stb_vorbis_open_filename(filename, &mut error, ptr::null());
    let mut v = v?;
    
    let limit : usize = (v.channels * 4096) as usize;
    *channels = v.channels;
    *sample_rate = v.sample_rate;
    
    let mut offset : usize = 0;
    let mut data_len : usize = 0;
    let mut total : usize = limit;
    
    let mut data : Vec<i16> = Vec::with_capacity(total);
    data.resize(total, 0);
    
    
    loop {
        let n = {
            let channels = v.channels;
            stb_vorbis_get_frame_short_interleaved(&mut v, channels, &mut data[offset..])
        };
        
        if n == 0 {
            break;
        }
        
        data_len += n as usize;
        offset += (n * v.channels) as usize;
        
        if offset + limit > total {
            total *= 2;
            data.resize(total, 0);
        }
        
    }
    
    // *output = data;
    data.resize(data_len, 0);
    stb_vorbis_close(v);
    
    return Ok(data);
}