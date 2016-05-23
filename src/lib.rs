// temporary disable lint for now...
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]

#![feature(question_mark, custom_derive, box_syntax, float_extras)]
#![feature(alloc_system)]

/**
 * Rust Stb Vorbis
 * 
 * Ogg vorbis audio decoder in pure rust.
 * This is ported from stb_vorbis (http://nothings.org/stb_vorbis) 
 * v1.09 by Sean Barrett
 *
 * MIT License
 * bungcip (gigih aji ibrahim)
 * 2016
 */


extern crate alloc_system;
extern crate libc;

use libc::*;
use std::mem;


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


static mut crc_table: [u32; 256] = [0; 256];


// the following table is block-copied from the specification
static inverse_db_table: [f32; 256] =
[
  1.0649863e-07, 1.1341951e-07, 1.2079015e-07, 1.2863978e-07, 
  1.3699951e-07, 1.4590251e-07, 1.5538408e-07, 1.6548181e-07, 
  1.7623575e-07, 1.8768855e-07, 1.9988561e-07, 2.1287530e-07, 
  2.2670913e-07, 2.4144197e-07, 2.5713223e-07, 2.7384213e-07, 
  2.9163793e-07, 3.1059021e-07, 3.3077411e-07, 3.5226968e-07, 
  3.7516214e-07, 3.9954229e-07, 4.2550680e-07, 4.5315863e-07, 
  4.8260743e-07, 5.1396998e-07, 5.4737065e-07, 5.8294187e-07, 
  6.2082472e-07, 6.6116941e-07, 7.0413592e-07, 7.4989464e-07, 
  7.9862701e-07, 8.5052630e-07, 9.0579828e-07, 9.6466216e-07, 
  1.0273513e-06, 1.0941144e-06, 1.1652161e-06, 1.2409384e-06, 
  1.3215816e-06, 1.4074654e-06, 1.4989305e-06, 1.5963394e-06, 
  1.7000785e-06, 1.8105592e-06, 1.9282195e-06, 2.0535261e-06, 
  2.1869758e-06, 2.3290978e-06, 2.4804557e-06, 2.6416497e-06, 
  2.8133190e-06, 2.9961443e-06, 3.1908506e-06, 3.3982101e-06, 
  3.6190449e-06, 3.8542308e-06, 4.1047004e-06, 4.3714470e-06, 
  4.6555282e-06, 4.9580707e-06, 5.2802740e-06, 5.6234160e-06, 
  5.9888572e-06, 6.3780469e-06, 6.7925283e-06, 7.2339451e-06, 
  7.7040476e-06, 8.2047000e-06, 8.7378876e-06, 9.3057248e-06, 
  9.9104632e-06, 1.0554501e-05, 1.1240392e-05, 1.1970856e-05, 
  1.2748789e-05, 1.3577278e-05, 1.4459606e-05, 1.5399272e-05, 
  1.6400004e-05, 1.7465768e-05, 1.8600792e-05, 1.9809576e-05, 
  2.1096914e-05, 2.2467911e-05, 2.3928002e-05, 2.5482978e-05, 
  2.7139006e-05, 2.8902651e-05, 3.0780908e-05, 3.2781225e-05, 
  3.4911534e-05, 3.7180282e-05, 3.9596466e-05, 4.2169667e-05, 
  4.4910090e-05, 4.7828601e-05, 5.0936773e-05, 5.4246931e-05, 
  5.7772202e-05, 6.1526565e-05, 6.5524908e-05, 6.9783085e-05, 
  7.4317983e-05, 7.9147585e-05, 8.4291040e-05, 8.9768747e-05, 
  9.5602426e-05, 0.00010181521, 0.00010843174, 0.00011547824, 
  0.00012298267, 0.00013097477, 0.00013948625, 0.00014855085, 
  0.00015820453, 0.00016848555, 0.00017943469, 0.00019109536, 
  0.00020351382, 0.00021673929, 0.00023082423, 0.00024582449, 
  0.00026179955, 0.00027881276, 0.00029693158, 0.00031622787, 
  0.00033677814, 0.00035866388, 0.00038197188, 0.00040679456, 
  0.00043323036, 0.00046138411, 0.00049136745, 0.00052329927, 
  0.00055730621, 0.00059352311, 0.00063209358, 0.00067317058, 
  0.00071691700, 0.00076350630, 0.00081312324, 0.00086596457, 
  0.00092223983, 0.00098217216, 0.0010459992,  0.0011139742, 
  0.0011863665,  0.0012634633,  0.0013455702,  0.0014330129, 
  0.0015261382,  0.0016253153,  0.0017309374,  0.0018434235, 
  0.0019632195,  0.0020908006,  0.0022266726,  0.0023713743, 
  0.0025254795,  0.0026895994,  0.0028643847,  0.0030505286, 
  0.0032487691,  0.0034598925,  0.0036847358,  0.0039241906, 
  0.0041792066,  0.0044507950,  0.0047400328,  0.0050480668, 
  0.0053761186,  0.0057254891,  0.0060975636,  0.0064938176, 
  0.0069158225,  0.0073652516,  0.0078438871,  0.0083536271, 
  0.0088964928,  0.009474637,   0.010090352,   0.010746080, 
  0.011444421,   0.012188144,   0.012980198,   0.013823725, 
  0.014722068,   0.015678791,   0.016697687,   0.017782797, 
  0.018938423,   0.020169149,   0.021479854,   0.022875735, 
  0.024362330,   0.025945531,   0.027631618,   0.029427276, 
  0.031339626,   0.033376252,   0.035545228,   0.037855157, 
  0.040315199,   0.042935108,   0.045725273,   0.048696758, 
  0.051861348,   0.055231591,   0.058820850,   0.062643361, 
  0.066714279,   0.071049749,   0.075666962,   0.080584227, 
  0.085821044,   0.091398179,   0.097337747,   0.10366330, 
  0.11039993,    0.11757434,    0.12521498,    0.13335215, 
  0.14201813,    0.15124727,    0.16107617,    0.17154380, 
  0.18269168,    0.19456402,    0.20720788,    0.22067342, 
  0.23501402,    0.25028656,    0.26655159,    0.28387361, 
  0.30232132,    0.32196786,    0.34289114,    0.36517414, 
  0.38890521,    0.41417847,    0.44109412,    0.46975890, 
  0.50028648,    0.53279791,    0.56742212,    0.60429640, 
  0.64356699,    0.68538959,    0.72993007,    0.77736504, 
  0.82788260,    0.88168307,    0.9389798,     1.0
];


const CP_L : i8 = (PLAYBACK_LEFT  | PLAYBACK_MONO) as i8;
const CP_C : i8 = (PLAYBACK_LEFT  | PLAYBACK_RIGHT | PLAYBACK_MONO)  as i8;
const CP_R : i8 = (PLAYBACK_RIGHT | PLAYBACK_MONO) as i8;

static channel_position: [[i8; 6]; 7] = [
   [ 0, 0, 0, 0, 0, 0 ],
   [ CP_C, CP_C, CP_C, CP_C, CP_C, CP_C ],
   [ CP_L, CP_R, CP_R, CP_R, CP_R, CP_R ],
   [ CP_L, CP_C, CP_R, CP_R, CP_R, CP_R ],
   [ CP_L, CP_R, CP_L, CP_R, CP_R, CP_R ],
   [ CP_L, CP_C, CP_R, CP_L, CP_R, CP_R ],
   [ CP_L, CP_C, CP_R, CP_L, CP_R, CP_C ],
];

static ogg_page_header: [u8; 4] = [ 0x4f, 0x67, 0x67, 0x53 ];

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
// much you do need except to succeed (at which point you can
// query get_info to find the exact amount required. yes I know
// this is lame).
//
// If you pass in a non-NULL buffer of the type below, allocation
// will occur from it as described above. Otherwise just pass NULL
// to use malloc()/alloca()

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy)]
pub struct Codebook
{
   dimensions: i32, entries: i32,
   codeword_lengths: *mut u8,
   minimum_value: f32,
   delta_value: f32,
   value_bits: u8,
   lookup_type: u8,
   sequence_p: u8,
   sparse: bool,
   lookup_values: u32,
   multiplicands: *mut codetype,
   codewords: *mut u32,
   fast_huffman: [i16; FAST_HUFFMAN_TABLE_SIZE as usize],
   sorted_codewords: *mut u32,
   sorted_values: *mut i32,
   sorted_entries: i32,
} 

impl Clone for Codebook {
    fn clone(&self) -> Self {
        *self
    }
}

#[repr(C)]
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

// union Floor
#[repr(C)]
pub struct Floor
{
//    floor0: Floor0,
   floor1: Floor1,
}

#[repr(C)] 
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct MappingChannel
{
   magnitude: u8,
   angle: u8,
   mux: u8,
}


#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct CRCscan
{
   goal_crc: u32,    // expected crc if match
   bytes_left: i32,  // bytes left in packet
   crc_so_far: u32,  // running crc
   bytes_done: i32,  // bytes processed in _current_ chunk
   sample_loc: u32,  // granule pos encoded in page
} 

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProbedPage
{
   page_start: u32, page_end: u32,
   last_decoded_sample: u32
}
 


#[repr(C)]
pub struct stb_vorbis
{
  // user-accessible info
   sample_rate: u32,
   channels: i32,

   setup_memory_required: u32,
   temp_memory_required: u32,
   setup_temp_memory_required: u32,

  // input config
   f: *mut libc::FILE,
   f_start: u32,
   close_on_free: bool,

   stream: *mut u8,
   stream_start: *mut u8,
   stream_end: *mut u8,

   stream_len: u32,

   push_mode: bool,

   first_audio_page_offset: u32,

   p_first: ProbedPage, p_last: ProbedPage,

  // memory management
   alloc: stb_vorbis_alloc,
   setup_offset: i32,
   temp_offset: i32,

  // run-time results
   pub eof: bool,
   pub error: i32, //STBVorbisError,

  // user-useful data

  // header info
   blocksize: [i32; 2],
   blocksize_0: i32, blocksize_1: i32,
   codebook_count: i32,
   codebooks: *mut Codebook,
   floor_count: i32,
   floor_types: [u16; 64], // varies
   floor_config: *mut Floor,
   residue_count: i32,
   residue_types: [u16; 64], // varies
   residue_config: *mut Residue,
   mapping_count: i32,
   mapping: *mut Mapping,
   mode_count: i32,
   mode_config: [Mode; 64],  // varies

   total_samples: u32,

  // decode buffer
   channel_buffers: [*mut f32; STB_VORBIS_MAX_CHANNELS as usize],
   outputs        : [*mut f32; STB_VORBIS_MAX_CHANNELS as usize],

   previous_window: [*mut f32; STB_VORBIS_MAX_CHANNELS as usize],
   previous_length: i32,

   finalY: [*mut i16; STB_VORBIS_MAX_CHANNELS as usize],

   current_loc: u32, // sample location of next frame to decode
   current_loc_valid: i32,

  // per-blocksize precomputed data
   
   // twiddle factors
   A: [*mut f32; 2], B: [*mut f32; 2], C: [*mut f32; 2],
   window: [*mut f32; 2],
   bit_reverse: [*mut u16; 2],

  // current page/packet/segment streaming info
   serial: u32, // stream serial number for verification
   last_page: i32,
   segment_count: i32,
   segments: [u8; 255],
   page_flag: u8,
   bytes_in_seg: u8,
   first_decode: u8,
   next_seg: i32,
   last_seg: i32,  // flag that we're on the last segment
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
   scan: [CRCscan; STB_VORBIS_PUSHDATA_CRC_COUNT as usize],

  // sample-access
   channel_buffer_start: i32,
   channel_buffer_end: i32,
}

pub type vorb = stb_vorbis;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stb_vorbis_info
{
   pub sample_rate: u32,
   pub channels: i32,

   pub setup_memory_required: u32,
   pub setup_temp_memory_required: u32,
   pub temp_memory_required: u32,

   pub max_frame_size: i32,
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

fn error(f: &mut vorb, e: i32) -> i32
{
    // NOTE: e is STBVorbisError
    f.error = e;
    if f.eof == false && e != STBVorbisError::VORBIS_need_more_data as i32 {
        f.error = e; // breakpoint for debugging
    }
    
    if e == STBVorbisError::VORBIS_invalid_stream as i32 {
        panic!("Cek error nya!");
    }
    
    return 0;
}

fn include_in_sort(c: &Codebook, len: u8) -> i32
{
   if c.sparse == true { 
       assert!(len as i32 != NO_CODE); 
       return 1; // true
    }
   if len as i32 == NO_CODE {
       return 0; // false
   }
   if len as i32 > STB_VORBIS_FAST_HUFFMAN_LENGTH {
       return 1; // true
   }
   return 0;
}



unsafe fn setup_malloc(f: &mut vorb, sz: i32) -> *mut c_void
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


unsafe fn setup_free(f: &mut vorb, p: *mut c_void)
{
   if f.alloc.alloc_buffer.is_null() == false {
       return; // do nothing; setup mem is a stack
   }
   libc::free(p);
}


unsafe fn setup_temp_malloc(f: &mut vorb, sz: i32) -> *mut c_void
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


unsafe fn setup_temp_free(f: &mut vorb, p: *mut c_void, sz: i32)
{
   if f.alloc.alloc_buffer.is_null() == false {
      f.temp_offset += (sz+3)&!3;
      return;
   }
   libc::free(p);
}

const  CRC32_POLY  : u32 =  0x04c11db7;   // from spec


unsafe fn crc32_init()
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
fn bit_reverse(n: u32) -> u32 
{
  let n = ((n & 0xAAAAAAAA) >>  1) | ((n & 0x55555555) << 1);
  let n = ((n & 0xCCCCCCCC) >>  2) | ((n & 0x33333333) << 2);
  let n = ((n & 0xF0F0F0F0) >>  4) | ((n & 0x0F0F0F0F) << 4);
  let n = ((n & 0xFF00FF00) >>  8) | ((n & 0x00FF00FF) << 8);
  return (n >> 16) | (n << 16);
}


fn square(x: f32) -> f32{
    x * x
}

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
   
   return f64::ldexp(res, (exp as i32 - 788 ) as isize) as f32;
}

// zlib & jpeg huffman tables assume that the output symbols
// can either be arbitrarily arranged, or have monotonically
// increasing frequencies--they rely on the lengths being sorted;
// this makes for a very simple generation algorithm.
// vorbis allows a huffman table with non-sorted lengths. This
// requires a more sophisticated construction, since symbols in
// order do not map to huffman codes "in order".
unsafe fn add_entry(c: &Codebook, huff_code: u32, symbol: i32, count: i32, len: i32, values: *mut u32)
{
    // TODO(bungcip): maybe change len as u8?
    // TODO(bungcip): maybe symbol len as u32?
    
   if c.sparse == false {
      *c.codewords.offset(symbol as isize) = huff_code;
   } else {
      let count = count as isize;
      *c.codewords.offset(count) = huff_code;
      *c.codeword_lengths.offset(count) = len as u8;
      *values.offset(count) = symbol as u32;
   }
}



unsafe fn compute_codewords(c: &mut Codebook, len: *mut u8, n: i32, values: *mut u32) -> i32
{
   let mut m=0;
   let mut available: [u32; 32] = std::mem::zeroed();

//    memset(available, 0, sizeof(available));
   // find the first entry
   let mut k = 0;
   while k < n {
       if (*len.offset(k as isize) as i32) < NO_CODE {
           break;
       }
       k += 1;
   }
   
   if k == n { assert!(c.sorted_entries == 0); return 1; } // true
   
   // add to the list
   add_entry(c, 0, k, m, *len.offset(k as isize) as i32, values);
   m += 1;
   
   // add all available leaves
   let mut i = 1;
   while i <= *len.offset(k as isize) {
      available[i as usize] = 1u32 << (32-i);
      i += 1;
   }
   
   // note that the above code treats the first case specially,
   // but it's really the same as the following code, so they
   // could probably be combined (except the initial code is 0,
   // and I use 0 in available[] to mean 'empty')
   for i in k+1 .. n {
      let res : u32;
      let mut z = *len.offset(i as isize);
      if z as i32 == NO_CODE {
          continue;
      }
      // find lowest available leaf (should always be earliest,
      // which is what the specification calls for)
      // note that this property, and the fact we can never have
      // more than one free leaf at a given level, isn't totally
      // trivial to prove, but it seems true and the assert never
      // fires, so!
      while z > 0 && available[z as usize]  == 0{
          z -= 1;
      }
      if z == 0 { return 0; } // false
      res = available[z as usize];
    //   assert!(z >= 0 && z < 32);
      assert!(z < 32); // NOTE(z is u8 so negative is impossible)
      available[z as usize] = 0;
      add_entry(c, bit_reverse(res), i, m, *len.offset(i as isize) as i32, values);
      m += 1;
      
      // propogate availability up the tree
      if z != *len.offset(i as isize) {
        //  assert!(*len.offset(i as isize) >= 0 && *len.offset(i as isize) < 32);
         assert!(*len.offset(i as isize) < 32); // NOTE (len[x] is already unsigned)
         
         let mut y = *len.offset(i as isize);
         while y > z {
            assert!(available[y as usize] == 0);
            available[y as usize] = res + (1 << (32-y));
             
             y -= 1;
         }         
      }
   }
   
   return 1; // true
}


// this is a weird definition of log2() for which log2(1) = 1, log2(2) = 2, log2(4) = 3
// as required by the specification. fast(?) implementation from stb.h
// @OPTIMIZE: called multiple times per-packet with "constants"; move to setup

fn ilog(n: i32) -> i32
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

fn get_window(f: &vorb, len: i32) -> *mut f32
{
   let len = len << 1;
   if len == f.blocksize_0 { return f.window[0]; }
   if len == f.blocksize_1 { return f.window[1]; }

   unreachable!();
}

unsafe fn compute_bitreverse(n: i32, rev: *mut u16)
{
   let ld = ilog(n) - 1; // ilog is off-by-one from normal definitions
   let n8 = n >> 3;
   
   for i in 0 .. n8 {
       *rev.offset(i as isize) = ((bit_reverse(i as u32) >> (32-ld+3)) << 2) as u16;
   }
}

fn uint32_compare(p: *const c_void, q: *const c_void) -> i32
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

fn vorbis_validate(data: *const u8) -> i32
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

fn lookup1_values(entries: i32, dim: i32) -> i32
{
    let mut r =  f64::floor(f64::exp(f64::ln(entries as f64) / dim as f64)) as i32;
    if f64::floor(f64::powi( (r+1) as f64, dim)) as i32 <= entries {
       r += 1;
    }
    assert!(f64::powi((r+1) as f64, dim) > entries as f64);
    assert!(f64::powi(r as f64, dim) as i32 <= entries);
    return r;
}

// from CRC
const M_PI : f32 = 3.14159265358979323846264;

// called twice per file
fn compute_twiddle_factors(n: i32, A: *mut f32, B: *mut f32, C: *mut f32)
{
    use std::f32;
    
    let n4 = n >> 2;
    let n8 = n >> 3;

    let mut k = 0;
    let mut k2 = 0;
    
    while k < n4 {
        unsafe {
            let x1 = (4*k) as f32 * M_PI / n as f32;
            *A.offset(k2 as isize)     = f32::cos(x1) as f32;
            *A.offset((k2+1) as isize) =  -f32::sin(x1) as f32;
            
            let x2 = (k2+1) as f32 * M_PI / n as f32 / 2.0;
            *B.offset(k2 as isize)     = (f32::cos(x2) * 0.5) as f32;
            *B.offset((k2+1) as isize) = (f32::sin(x2) * 0.5) as f32;
        }
        
        k += 1; k2 += 2;
    }

    let mut k = 0;
    let mut k2 = 0;
    
    while k < n8 {
        unsafe {
            let x1 = (2*(k2+1)) as f32 * M_PI / n as f32;
            *C.offset(k2) = f32::cos(x1) as f32;
            
            let x2 = (2*(k2+1)) as f32 * M_PI / n as f32;
            *C.offset(k2+1) = -f32::sin(x2) as f32;
        }
        
        k += 1; k2 += 2;
    }

}




fn neighbors(x: *mut u16, n: i32, plow: *mut i32, phigh: *mut i32)
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


unsafe fn point_compare(p: *const c_void, q: *const c_void) -> i32
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
        $f.push_mode == true
    }
}

macro_rules! MAGIC {
    ($SHIFT: expr) => { (1.5f32 * (1 << (23-$SHIFT)) as f32 + 0.5f32/(1 << $SHIFT) as f32) }
}

macro_rules! ADDEND {
    ($SHIFT: expr) => { (((150-$SHIFT) << 23) + (1 << 22) ) }
}

macro_rules! FAST_SCALED_FLOAT_TO_INT {
    ($x: expr, $s: expr) => {{
        let temp : i32 = $crate::std::mem::transmute($x + MAGIC!($s));
        temp - ADDEND!($s)        
    }}
}

macro_rules! CHECK {
    ($f: expr) => {
        // assert!( $f.channel_buffers[1].is_null() == false );
    }
}

// @OPTIMIZE: if you want to replace this bresenham line-drawing routine,
// note that you must produce bit-identical output to decode correctly;
// this specific sequence of operations is specified in the spec (it's
// drawing integer-quantized frequency-space lines that the encoder
// expects to be exactly the same)
//     ... also, isn't the whole point of Bresenham's algorithm to NOT
// have to divide in the setup? sigh
macro_rules! LINE_OP {
    ($a: expr, $b: expr) => {
        $a *= $b
    }
}




unsafe fn get8(z: &mut vorb) -> u8
{
   if USE_MEMORY!(z) {
      if z.stream >= z.stream_end { 
          z.eof = true;
          return 0;
      }
      
      let c = *z.stream;
      z.stream = z.stream.offset(1);
      return c;
   }

   let c = libc::fgetc(z.f);
   if c == libc::EOF { 
       z.eof = true; return 0; 
    }
   return c as u8;
}



unsafe fn get32(f: &mut vorb) -> u32
{
   let mut x : u32 = get8(f) as u32;
   x += (get8(f) as u32) << 8;
   x += (get8(f) as u32) << 16;
   x += (get8(f) as u32) << 24;
   return x;
}


unsafe fn getn(z: &mut vorb, data: *mut u8, n: i32) -> i32
{
   if USE_MEMORY!(z) {
      if z.stream.offset(n as isize) > z.stream_end { z.eof = true; return 0; }
      std::ptr::copy_nonoverlapping(z.stream, data, n as usize);
    //   libc::memcpy(data, z.stream, n);
      z.stream = z.stream.offset(n as isize);
      return 1;
   }

   if libc::fread(data as *mut c_void, n as usize, 1, z.f) == 1 {
      return 1;
   } else {
      z.eof = true;
      return 0;
   }
}


unsafe fn skip(z: &mut vorb, n: i32)
{
   if USE_MEMORY!(z) {
      z.stream = z.stream.offset(n as isize);
      if z.stream >= z.stream_end {z.eof = true;}
      return;
   }

   let x = libc::ftell(z.f);
   libc::fseek(z.f, x+n, libc::SEEK_SET);
}

unsafe fn capture_pattern(f: &mut vorb) -> i32
{
   if 0x4f != get8(f) {return 0;}
   if 0x67 != get8(f) {return 0;}
   if 0x67 != get8(f) {return 0;}
   if 0x53 != get8(f) {return 0;}
   return 1;
}


const EOP : i32 = -1;
const INVALID_BITS : i32 = -1;

unsafe fn get8_packet_raw(f: *mut vorb) -> i32
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
    
    return get8(f) as i32;
}


unsafe fn get8_packet(f: *mut vorb) -> i32
{
    let x = get8_packet_raw(f);
    
    let f : &mut vorb = std::mem::transmute(f as *mut vorb); 
    f.valid_bits = 0;
    
    return x;
}


unsafe fn flush_packet(f: *mut vorb)
{
    while get8_packet_raw(f) != EOP {}
}


// @OPTIMIZE: this is the secondary bit decoder, so it's probably not as important
// as the huffman decoder?

unsafe fn get_bits(f: &mut vorb, n: i32) -> u32
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



unsafe fn start_page(f: &mut vorb) -> i32
{
   if capture_pattern(f) == 0 {
       return error(f, STBVorbisError::VORBIS_missing_capture_pattern as i32);
   } 
   return start_page_no_capturepattern(f);
}


const PAGEFLAG_continued_packet : i32 =   1;
const PAGEFLAG_first_page       : i32 =   2;
const PAGEFLAG_last_page        : i32 =   4;



unsafe fn start_packet(f: &mut vorb) -> i32
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

unsafe fn maybe_start_packet(f: &mut vorb) -> i32
{
    use STBVorbisError::{VORBIS_missing_capture_pattern, VORBIS_continued_packet_flag_invalid};
    
   if f.next_seg == -1 {
      let x = get8(f) as i32;
      if f.eof == true { return 0; } // EOF at page boundary is not an error!
      if 0x4f != x       { return error(f, VORBIS_missing_capture_pattern as i32); }
      if 0x67 != get8(f) { return error(f, VORBIS_missing_capture_pattern as i32); }
      if 0x67 != get8(f) { return error(f, VORBIS_missing_capture_pattern as i32); }
      if 0x53 != get8(f) { return error(f, VORBIS_missing_capture_pattern as i32); }
      if start_page_no_capturepattern(f) == 0 { return 0; }
      if (f.page_flag & PAGEFLAG_continued_packet as u8) != 0 {
         // set up enough state that we can read this packet if we want,
         // e.g. during recovery
         f.last_seg = 0;
         f.bytes_in_seg = 0;
         return error(f, VORBIS_continued_packet_flag_invalid as i32);
      }
   }
   return start_packet(f);
}


unsafe fn next_segment(f: &mut vorb) -> i32
{
    use STBVorbisError::VORBIS_continued_packet_flag_invalid;
//    int len;
   if f.last_seg != 0 {return 0;}
   if f.next_seg == -1 {
      f.last_seg_which = f.segment_count-1; // in case start_page fails
      if start_page(f) == 0 { f.last_seg = 1; return 0; }
      if (f.page_flag & PAGEFLAG_continued_packet as u8) == 0 {return error(f, VORBIS_continued_packet_flag_invalid as i32); }
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



unsafe fn vorbis_decode_packet(f: &mut vorb, len: &mut i32, p_left: &mut i32, p_right: &mut i32) -> i32
{
    let mut mode_index : i32 = 0;
    let mut left_end: i32 = 0;
    let mut right_end: i32 = 0;
    
    if vorbis_decode_initial(f, p_left, &mut left_end, p_right, &mut right_end, &mut mode_index) == 0{
        return 0;
    }
    
    // hack to fight borrow checker
    let mode : &Mode = {
        let p = &f.mode_config[mode_index as usize];
        let p = p as *const _;
        let p : &Mode = std::mem::transmute(p);
        p
    };
    
    return vorbis_decode_packet_rest(
        f, len, mode, 
        *p_left, left_end, *p_right, right_end, p_left
    );
}


unsafe fn vorbis_pump_first_frame(f: &mut stb_vorbis)
{
    let mut len: i32 = 0;
    let mut right: i32 = 0;
    let mut left: i32 = 0;
    
    if vorbis_decode_packet(f, &mut len, &mut left, &mut right) != 0 {
        vorbis_finish_frame(f, len, left, right);
    }
}

// NOTE(bungcip): p must be zeroed before using it
unsafe fn vorbis_init(p: &mut stb_vorbis, z: *const stb_vorbis_alloc)
{
   
   if z.is_null() == false {
      p.alloc = *z;
      p.alloc.alloc_buffer_length_in_bytes = (p.alloc.alloc_buffer_length_in_bytes+3) & !3;
      p.temp_offset = p.alloc.alloc_buffer_length_in_bytes;
   }
   p.eof = false;
   p.error = STBVorbisError::VORBIS__no_error as i32;
   p.stream = std::ptr::null_mut();
   p.codebooks = std::ptr::null_mut();
   p.page_crc_tests = -1;

   p.close_on_free = false;
   p.f = std::ptr::null_mut();
}

// close an ogg vorbis file and free all memory in use
pub unsafe fn stb_vorbis_close(p: *mut stb_vorbis)
{
   if p.is_null(){
       return;
   }
   
   vorbis_deinit(std::mem::transmute(p));
   setup_free(std::mem::transmute(p),p as *mut c_void);
}

// create an ogg vorbis decoder from an open FILE *, looking for a stream at
// the _current_ seek point (ftell); the stream will be of length 'len' bytes.
// on failure, returns NULL and sets *error. note that stb_vorbis must "own"
// this stream; if you seek it in between calls to stb_vorbis, it will become
// confused.
pub unsafe fn stb_vorbis_open_file_section(file: *mut libc::FILE, close_on_free: bool, error: *mut i32, alloc: *const stb_vorbis_alloc, length: u32) -> *mut stb_vorbis
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


// create an ogg vorbis decoder from an open FILE *, looking for a stream at
// the _current_ seek point (ftell). on failure, returns NULL and sets *error.
// note that stb_vorbis must "own" this stream; if you seek it in between
// calls to stb_vorbis, it will become confused. Morever, if you attempt to
// perform stb_vorbis_seek_*() operations on this file, it will assume it
// owns the _entire_ rest of the file after the start point. Use the next
// function, stb_vorbis_open_file_section(), to limit it.
pub unsafe fn stb_vorbis_open_file(file: *mut FILE,  close_on_free: bool, error: *mut i32, alloc: *const stb_vorbis_alloc) -> *mut stb_vorbis
{
    let start = libc::ftell(file);
    libc::fseek(file, 0, libc::SEEK_END);
    
    let len = libc::ftell(file) - start;
    libc::fseek(file, start, libc::SEEK_SET);
    
    return stb_vorbis_open_file_section(file, close_on_free, error, alloc, len as u32);
}


// create an ogg vorbis decoder from a filename via fopen(). on failure,
// returns NULL and sets *error (possibly to VORBIS_file_open_failure).
pub unsafe fn stb_vorbis_open_filename(filename: *const i8, error: *mut i32, alloc: *const stb_vorbis_alloc) -> *mut stb_vorbis
{
   let  mode: &'static [u8; 3] = b"rb\0";
   let f = libc::fopen(filename, mode.as_ptr() as *const i8);
   if f.is_null() == false {
      return stb_vorbis_open_file(f, true, error, alloc);
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

unsafe fn vorbis_decode_initial(f: &mut vorb, p_left_start: *mut i32, p_left_end: *mut i32, p_right_start: *mut i32, p_right_end: *mut i32, mode: *mut i32) -> i32
{
   f.channel_buffer_start = 0;
   f.channel_buffer_end = 0;

   loop {
        if f.eof == true {return 0;} // false
        if maybe_start_packet(f) == 0 {
            return 0; // false
        }
        // check packet type
        if get_bits(f,1) != 0 {
            if IS_PUSH_MODE!(f) {
                return error(f, STBVorbisError::VORBIS_bad_packet_type as i32);
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
   let i : i32 = get_bits(f, x) as i32;
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
   let n : i32;
   let prev: i32;
   let next: i32;
   
   if m.blockflag != 0 {
      n = f.blocksize_1;
      prev = get_bits(f,1) as i32;
      next = get_bits(f,1) as i32;
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

unsafe fn vorbis_finish_frame(f: &mut stb_vorbis, len: i32, left: i32, right: i32) -> i32
{
//    int prev,i,j;
   // we use right&left (the start of the right- and left-window sin()-regions)
   // to determine how much to return, rather than inferring from the rules
   // (same result, clearer code); 'left' indicates where our sin() window
   // starts, therefore where the previous window's right edge starts, and
   // therefore where to start mixing from the previous buffer. 'right'
   // indicates where our sin() ending-window starts, therefore that's where
   // we start saving, and where our returned-data ends.

   // mixin from previous window
   if f.previous_length != 0 {
    //   int i,j, 
      let n = f.previous_length;
      let w = get_window(f, n);
      for i in 0 .. f.channels {
         let i = i as usize;
         for j in 0 .. n {
            *f.channel_buffers[i].offset( (left + j) as isize ) =
               *f.channel_buffers[i].offset((left + j) as isize) * *w.offset(    j as isize) +
               *f.previous_window[i].offset(        j  as isize) * *w.offset(n as isize - 1 - (j as isize));
         }
      }
   }

   let prev = f.previous_length;

   // last half of this data becomes previous window
   f.previous_length = len - right;

   // @OPTIMIZE: could avoid this copy by double-buffering the
   // output (flipping previous_window with channel_buffers), but
   // then previous_window would have to be 2x as large, and
   // channel_buffers couldn't be temp mem (although they're NOT
   // currently temp mem, they could be (unless we want to level
   // performance by spreading out the computation))
   for i in 0 .. f.channels {
       let i = i as usize;
      let mut j = 0;
      while right + j < len {
         *f.previous_window[i].offset(j as isize) = *f.channel_buffers[i].offset( (right+j) as isize);
          j += 1;
      }           
   }

   if prev == 0 {
      // there was no previous packet, so this data isn't valid...
      // this isn't entirely true, only the would-have-overlapped data
      // isn't valid, but this seems to be what the spec requires
      return 0;
   }

   // truncate a short frame
   let right = if len < right { len } else { right };

   f.samples_output += (right-left) as u32;

   return right - left;
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
pub unsafe fn stb_vorbis_get_frame_short_interleaved(f: &mut stb_vorbis, num_c: i32, buffer: *mut i16, num_shorts: i32) -> i32
{
   let mut output: *mut *mut f32 = std::ptr::null_mut();
   let mut buffer = buffer;
   
   if num_c == 1 {
       return stb_vorbis_get_frame_short(f, num_c, &mut buffer, num_shorts);
   }
   let mut len = stb_vorbis_get_frame_float(f, std::ptr::null_mut(), &mut output);
   if len != 0 {
      if len*num_c > num_shorts {
        len = num_shorts / num_c;  
      } 
      convert_channels_short_interleaved(num_c, buffer, f.channels, output, 0, len);
   }
   return len;
}

// decode an entire file and output the data interleaved into a malloc()ed
// buffer stored in *output. The return value is the number of samples
// decoded, or -1 if the file could not be opened or was not an ogg vorbis file.
// When you're done with it, just free() the pointer returned in *output.

pub unsafe fn stb_vorbis_decode_filename(filename: *const i8, channels: *mut i32, sample_rate: *mut i32, output: *mut *mut i16) -> i32
{
//    int data_len, offset, total, limit, error;
//    short *data;
   let mut error : i32 = 0;
   let v: *mut stb_vorbis = stb_vorbis_open_filename(filename, &mut error, std::ptr::null_mut());
   if v == std::ptr::null_mut(){
       return -1;
   }
   
   let v: &mut stb_vorbis = std::mem::transmute(v);
    
   let limit = v.channels * 4096;
   *channels = v.channels;
   if sample_rate.is_null() == false {
      *sample_rate = v.sample_rate as i32;
   }
   
   let mut offset = 0;
   let mut data_len = 0;
   let mut total = limit;
   let mut data : *mut i16 = libc::malloc(total as usize * std::mem::size_of::<i16>()) as *mut i16;
   if data == std::ptr::null_mut() {
      stb_vorbis_close(v);
      return -2;
   }

   loop {
       let ch = v.channels;
      let  n = stb_vorbis_get_frame_short_interleaved(v, ch, data.offset(offset as isize), total-offset);
      if n == 0{
        break;  
      } 
      data_len += n;
      offset += n * v.channels;
      if offset + limit > total {
         total *= 2;
         let data2 = libc::realloc(data as *mut c_void, total as usize * std::mem::size_of::<i16>()) as *mut i16;
         if data2 == std::ptr::null_mut() {
            libc::free(data as *mut c_void);
            stb_vorbis_close(v);
            return -2;
         }
         data = data2;
      }
   }
   *output = data;
   stb_vorbis_close(v);
   return data_len;
}


// decode the next frame and return the number of samples. the number of
// channels returned are stored in *channels (which can be NULL--it is always
// the same as the number of channels reported by get_info). *output will
// contain an array of float* buffers, one per channel. These outputs will
// be overwritten on the next call to stb_vorbis_get_frame_*.
//
// You generally should not intermix calls to stb_vorbis_get_frame_*()
// and stb_vorbis_get_samples_*(), since the latter calls the former.
pub unsafe fn stb_vorbis_get_frame_float(f: &mut stb_vorbis, channels: *mut i32, output: *mut *mut *mut f32) -> i32
{
//    int len, right,left,i;
   if IS_PUSH_MODE!(f){
       return error(f, STBVorbisError::VORBIS_invalid_api_mixing as i32);
   } 

    let mut len = 0;
    let mut left = 0;
    let mut right = 0;
   if vorbis_decode_packet(f, &mut len, &mut left, &mut right) == 0 {
      f.channel_buffer_start = 0;
      f.channel_buffer_end = 0;
      return 0;
   }

   let len = vorbis_finish_frame(f, len, left, right);
   for i in 0 .. f.channels {
      f.outputs[i as usize] = f.channel_buffers[i as usize].offset(left as isize);
   }

   f.channel_buffer_start = left;
   f.channel_buffer_end   = left+len;

   if channels.is_null() == false {*channels = f.channels;}
   if output.is_null() == false   {
       let o = f.outputs.as_ptr();
       let o = o as *mut *mut f32;
       *output = o;
    }
   return len;
}

pub unsafe fn stb_vorbis_get_frame_short(f: &mut vorb, num_c: i32, buffer: *mut *mut i16, num_samples: i32) -> i32
{
    let mut output: *mut *mut f32 = std::ptr::null_mut();
   let mut len = stb_vorbis_get_frame_float(f, std::ptr::null_mut(), &mut output);
   if len > num_samples{
     len = num_samples;
   }
   
    if len != 0 {
      convert_samples_short(num_c, buffer, 0, f.channels, output, 0, len);
    }
   return len;
}


const PLAYBACK_MONO  : i32 =   1;
const PLAYBACK_LEFT  : i32 =   2;
const PLAYBACK_RIGHT : i32 =   4;


unsafe fn convert_samples_short(buf_c: i32, buffer: *mut *mut i16, b_offset: i32, data_c: i32, data: *mut *mut f32, d_offset: i32, samples: i32)
{
   if buf_c != data_c && buf_c <= 2 && data_c <= 6 {
    //   static int channel_selector[3][2] = { {0}, {PLAYBACK_MONO}, {PLAYBACK_LEFT, PLAYBACK_RIGHT} };
      
      static channel_selector: [[i32; 2]; 3] = [
          [0, 0],
          [PLAYBACK_MONO, PLAYBACK_MONO],
          [PLAYBACK_LEFT, PLAYBACK_RIGHT]
      ];
      
      for i in 0 .. buf_c {
         compute_samples(channel_selector[buf_c as usize][i as usize], 
            (*buffer.offset(i as isize)).offset(b_offset as isize), data_c, data, d_offset, samples as i32);
      }
   } else {
      let limit = if buf_c < data_c { buf_c } else { data_c };
      
      let mut i = 0;
      while i < limit {
         copy_samples((*buffer.offset(i as isize)).offset(b_offset as isize),
             (*data.offset(i as isize)).offset(d_offset as isize), samples);
          i += 1;
      }
      
      while i < buf_c {
          std::ptr::write_bytes(
              (*buffer.offset(i as isize)).offset(b_offset as isize), 0, samples as usize);
          i += 1;
      }
   }
}


unsafe fn convert_channels_short_interleaved(buf_c: i32, buffer: *mut i16, data_c: i32, data: *mut *mut f32, d_offset: i32, len: i32)
{
   if buf_c != data_c && buf_c <= 2 && data_c <= 6 {
       assert!(buf_c == 2);
       for _ in 0 .. buf_c {
         compute_stereo_samples(buffer, data_c, data, d_offset, len);     
       }
   } else {
       let limit = if buf_c < data_c { buf_c } else { data_c };
       let mut buffer = buffer;
       for j in 0 .. len {
           let mut i = 0;
           while i < limit {
               let f : f32 = *(*data.offset(i as isize)).offset( (d_offset+j) as isize );
               let mut v : i32 = FAST_SCALED_FLOAT_TO_INT!(f, 15);
               if ( (v + 32768) as u32) > 65535 {
                   v = if v < 0 {  -32768 } else { 32767 };
               }
               
               *buffer = v as i16;
               buffer = buffer.offset(1);
               
               i += 1;
           }
           
           while i < buf_c {
               *buffer = 0;
               buffer = buffer.offset(1);

               i += 1;
           }
       }
       
   }
}

unsafe fn copy_samples(dest: *mut i16, src: *mut f32, len: i32)
{
   for i in 0 .. len  {
      let mut v : i32 = FAST_SCALED_FLOAT_TO_INT!(*src.offset(i as isize), 15);
      if ((v + 32768) as u32) > 65535 {
         v = if v < 0 { -32768 } else { 32767 };
      }
      *dest.offset(i as isize) = v as i16;
   }
}

// these functions seek in the Vorbis file to (approximately) 'sample_number'.
// after calling seek_frame(), the next call to get_frame_*() will include
// the specified sample. after calling stb_vorbis_seek(), the next call to
// stb_vorbis_get_samples_* will start with the specified sample. If you
// do not need to seek to EXACTLY the target sample when using get_samples_*,
// you can also use seek_frame().
pub unsafe fn stb_vorbis_seek(f: &mut stb_vorbis, sample_number: u32) -> i32
{
   if stb_vorbis_seek_frame(f, sample_number) == 0 {
      return 0;
   }

   if sample_number != f.current_loc {
      let mut n = 0;
      let frame_start = f.current_loc;
      stb_vorbis_get_frame_float(f, &mut n, std::ptr::null_mut());
      assert!(sample_number > frame_start);
      assert!(f.channel_buffer_start + (sample_number-frame_start) as i32 <= f.channel_buffer_end);
      f.channel_buffer_start += (sample_number - frame_start) as i32;
   }

   return 1;
}



unsafe fn init_blocksize(f: &mut vorb, b: i32, n: i32) -> i32
{
    use STBVorbisError::*;
    
   let n2 = n >> 1;
   let n4 = n >> 2;
   let n8 = n >> 3;
   
   let b = b as usize;
   f.A[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n2) as *mut f32;
   f.B[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n2) as *mut f32;
   f.C[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n4) as *mut f32;
   
   if f.A[b].is_null() || f.B[b].is_null() || f.C[b].is_null() {
     return error(f, VORBIS_outofmem as i32);  
   } 
   
   compute_twiddle_factors(n, f.A[b], f.B[b], f.C[b]);
   f.window[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n2) as *mut f32;
   
   if f.window[b].is_null() {return error(f, VORBIS_outofmem as i32);}
   compute_window(n, f.window[b]);

   f.bit_reverse[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n8) as *mut u16;
   if f.bit_reverse[b].is_null() { return error(f, VORBIS_outofmem as i32); }
   
   compute_bitreverse(n, f.bit_reverse[b]);
   return 1; // true
}

// accelerated huffman table allows fast O(1) match of all symbols
// of length <= STB_VORBIS_FAST_HUFFMAN_LENGTH

unsafe fn compute_accelerated_huffman(c: &mut Codebook)
{
//    int i, len;
   for i in 0 .. FAST_HUFFMAN_TABLE_SIZE {
       c.fast_huffman[i as usize] = -1;
   }

   let mut len = if c.sparse == true { c.sorted_entries } else  {c.entries};
   
   if len > 32767 {len = 32767;} // largest possible value we can encode!
   
   for i in 0 .. len {
      if *c.codeword_lengths.offset(i as isize) <= STB_VORBIS_FAST_HUFFMAN_LENGTH as u8 {
         let mut z : u32 = if c.sparse == true { 
             bit_reverse(*c.sorted_codewords.offset(i as isize)) 
         } else { 
             *c.codewords.offset(i as isize) 
        };
         // set table entries for all bit combinations in the higher bits
         while z < FAST_HUFFMAN_TABLE_SIZE as u32 {
             c.fast_huffman[z as usize] = i as i16;
             z += 1 << *c.codeword_lengths.offset(i as isize);
         }
      }
   }
}

// returns the current seek point within the file, or offset from the beginning
// of the memory buffer. In pushdata mode it returns 0.

pub unsafe fn stb_vorbis_get_file_offset(f: &stb_vorbis) -> u32
{
   if f.push_mode == true {return 0;}
   if USE_MEMORY!(f) {return (f.stream as usize - f.stream_start as usize) as u32;}
   return (libc::ftell(f.f) - f.f_start as i32) as u32;
}

unsafe fn start_page_no_capturepattern(f: &mut vorb) -> i32
{
    use STBVorbisError::*;
    
   // stream structure version
   if 0 != get8(f) {return error(f, VORBIS_invalid_stream_structure_version as i32);}
   // header flag
   f.page_flag = get8(f);
   // absolute granule position
   let loc0 = get32(f); 
   let loc1 = get32(f);
   // @TODO: validate loc0,loc1 as valid positions?
   // stream serial number -- vorbis doesn't interleave, so discard
   get32(f);
   // page sequence number
   let n = get32(f);
   f.last_page = n as i32;
   // CRC32
   get32(f);
   // page_segments
   f.segment_count = get8(f) as i32;
   let sc = f.segment_count;
   let segments_ptr = (&mut f.segments).as_mut_ptr();
   if getn(f, segments_ptr, sc) == 0 {
      return error(f, VORBIS_unexpected_eof as i32);
   }
   // assume we _don't_ know any the sample position of any segments
   f.end_seg_with_known_loc = -2;
   if loc0 != !0 || loc1 != !0 {
      let mut i;
      // determine which packet is the last one that will complete
      i = f.segment_count - 1;
      while i >= 0 {
         if f.segments[i as usize] < 255 {
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
   if f.first_decode != 0{
      let mut p : ProbedPage = std::mem::zeroed();
      let mut len : i32 = 0;
      for i in 0 .. f.segment_count {
         len += f.segments[i as usize] as i32;
      }
      len += 27 + f.segment_count as i32;
      p.page_start = f.first_audio_page_offset;
      p.page_end = p.page_start + len as u32;
      p.last_decoded_sample = loc0;
      f.p_first = p;
   }
   f.next_seg = 0;
   return 1; // true
}

unsafe fn predict_point(x: i32, x0: i32 , x1: i32 , y0: i32 , y1: i32 ) -> i32
{
   let dy = y1 - y0;
   let adx = x1 - x0;
   // @OPTIMIZE: force int division to round in the right direction... is this necessary on x86?
   let err = libc::abs(dy) * (x - x0);
   let off = err / adx;
   return if dy < 0  {y0 - off} else {y0 + off};
}

pub type YTYPE = i16;

unsafe fn do_floor(f: &mut vorb, map: &Mapping, i: i32, n: i32 , target: *mut f32, finalY: *mut YTYPE, _: *mut u8) -> i32
{
   let n2 = n >> 1;

   let s : &MappingChannel = std::mem::transmute(map.chan.offset(i as isize));
   let s : i32 = s.mux as i32;
   let floor = map.submap_floor[s as usize];
   
   if f.floor_types[floor as usize] == 0 {
      return error(f, STBVorbisError::VORBIS_invalid_stream as i32);
   } else {
      let g : &Floor1 = &(*f.floor_config.offset(floor as isize)).floor1;
      let mut j : i32;
      let mut lx : i32 = 0;
      let mut ly : i32 = *finalY.offset(0) as i32 * g.floor1_multiplier as i32;
      for q in 1 .. g.values {
         j = g.sorted_order[q as usize] as i32;
         if *finalY.offset(j as isize) >= 0
         {
            let hy : i32 = *finalY.offset(j as isize) as i32 * g.floor1_multiplier as i32;
            let hx : i32 = g.Xlist[j as usize] as i32;
            if lx != hx as i32{
               draw_line(target, lx,ly, hx, hy, n2);
            }
            CHECK!(f);
            lx = hx;
            ly = hy;
         }
      }
      if lx < n2 {
         // optimization of: draw_line(target, lx,ly, n,ly, n2);
         for j in lx .. n2{
            LINE_OP!(*target.offset(j as isize), inverse_db_table[ly as usize]);
         }
         CHECK!(f);
      }
   }
   return 1; // true
}

#[inline(always)]
unsafe fn draw_line(output: *mut f32, x0: i32, y0: i32, mut x1: i32, y1: i32, n: i32)
{
   let dy = y1 - y0;
   let adx = x1 - x0;
   let mut ady = libc::abs(dy);
   let base : i32;
   let mut x: i32 = x0;
   let mut y: i32 = y0;
   let mut err = 0;
   let sy;

   base = dy / adx;
   if dy < 0 {
      sy = base - 1;
   }   else{
      sy = base+1;
   }


   ady -= abs(base) * adx;
   
   if x1 > n {x1 = n;}
   if x < x1 {
      LINE_OP!(*output.offset(x as isize), inverse_db_table[y as usize]);
      
      x += 1;
      while x < x1 {
         err += ady;
          
         if err >= adx {
            err -= adx;
            y += sy;
         } else{
            y += base;
         }
         LINE_OP!(*output.offset(x as isize), inverse_db_table[y as usize]);
         
         x += 1;
      }      
   }
}


unsafe fn residue_decode(f: &mut vorb, book: &Codebook, target: *mut f32, mut offset: i32, n: i32, rtype: i32) -> i32
{
   if rtype == 0 {
      let step = n / book.dimensions;
      for k in 0 .. step {
         if codebook_decode_step(f, book, target.offset((offset+k) as isize), n-offset-k, step) == 0{
            return 0; // false
         }
      }
   } else {
       let mut k = 0;
       while k < n {
         if codebook_decode(f, book, target.offset(offset as isize), n-k) == 0{
            return 0; // FALSE
         }
         k += book.dimensions;
         offset += book.dimensions;
       }
       
   }
   return 1; // true
}

// CODEBOOK_ELEMENT_FAST is an optimization for the CODEBOOK_FLOATS case
// where we avoid one addition
macro_rules! CODEBOOK_ELEMENT {
    ($c: expr, $off: expr) => {
        *$c.multiplicands.offset($off as isize)
    }
}

macro_rules! CODEBOOK_ELEMENT_FAST {
    ($c: expr, $off: expr) => {
        *$c.multiplicands.offset($off as isize)
    }
}

macro_rules! CODEBOOK_ELEMENT_BASE {
    ($c: expr) => {
        0.0
    }
}


unsafe fn codebook_decode(f: &mut vorb, c: &Codebook, output: *mut f32, mut len: i32 ) -> i32
{
   let mut z = codebook_decode_start(f,c);
   if z < 0 {return 0;} // false
   if len > c.dimensions {len = c.dimensions;}

   z *= c.dimensions;
   if c.sequence_p != 0 {
      let mut last : f32 = CODEBOOK_ELEMENT_BASE!(c);
      for i in 0 .. len  {
         let val : f32 = CODEBOOK_ELEMENT_FAST!(c, z+i) + last;
         *output.offset(i as isize) += val;
         last = val + c.minimum_value;
      }
   } else {
      let last : f32 = CODEBOOK_ELEMENT_BASE!(c);
      for i in 0 .. len  {
         *output.offset(i as isize) += CODEBOOK_ELEMENT_FAST!(c,z+i) + last;
      }
   }

   return 1; // true
}

macro_rules! DECODE {
    ($var: expr, $f: expr, $c: expr) => {
        DECODE_RAW!($var, $f, $c);
        if $c.sparse == true {
            $var = *$c.sorted_values.offset($var as isize);
        }
    }
}


macro_rules! DECODE_RAW {
    ($var: expr, $f: expr, $c: expr) => {
        $var = codebook_decode_scalar($f, $c);
    }
}

macro_rules! DECODE_VQ {
    ($var: expr, $f: expr, $c: expr) => {
        DECODE_RAW!($var, $f, $c)
    }
}


unsafe fn codebook_decode_start(f: &mut vorb, c: &Codebook) -> i32
{
   let mut z = -1;

   // type 0 is only legal in a scalar context
   if c.lookup_type == 0 {
      error(f, STBVorbisError::VORBIS_invalid_stream as i32);
   } else {
      DECODE_VQ!(z,f,c);
      if c.sparse == true {assert!(z < c.sorted_entries);}
      if z < 0 {  // check for EOP
         if f.bytes_in_seg == 0 {
            if f.last_seg != 0 {
               return z;
            }
         }
         error(f,  STBVorbisError::VORBIS_invalid_stream as i32);
      }
   }
   return z;
}

#[inline(always)]
unsafe fn codebook_decode_scalar(f: &mut vorb, c: &Codebook) -> i32
{
   let mut i : i32;
   if f.valid_bits < STB_VORBIS_FAST_HUFFMAN_LENGTH {
      prep_huffman(f);
   }
   // fast huffman table lookup
   i = (f.acc & FAST_HUFFMAN_TABLE_MASK as u32) as i32;
   i = c.fast_huffman[i as usize] as i32;
   if i >= 0 {
      f.acc >>= *c.codeword_lengths.offset(i as isize);
      f.valid_bits -= *c.codeword_lengths.offset(i as isize) as i32;
      if f.valid_bits < 0 { f.valid_bits = 0; return -1; }
      return i;
   }
   return codebook_decode_scalar_raw(f,c);
}

// @OPTIMIZE: primary accumulator for huffman
// expand the buffer to as many bits as possible without reading off end of packet
// it might be nice to allow f->valid_bits and f->acc to be stored in registers,
// e.g. cache them locally and decode locally
#[inline(always)]
unsafe fn prep_huffman(f: &mut vorb)
{
   if f.valid_bits <= 24 {
      if f.valid_bits == 0 {f.acc = 0;}
      
      while {
         if f.last_seg != 0 && f.bytes_in_seg == 0 {return;}
         let z : i32 = get8_packet_raw(f);
         if z == EOP {return;}
         f.acc += (z as u32) << f.valid_bits;
         f.valid_bits += 8;
          
         // condition
         f.valid_bits <= 24
      }{/* do nothing */}
   }
}

unsafe fn codebook_decode_scalar_raw(f: &mut vorb, c: &Codebook) -> i32
{
//    int i;
   prep_huffman(f);

   if c.codewords.is_null()  && c.sorted_codewords.is_null() {
      return -1;
   }

   // cases to use binary search: sorted_codewords && !c.codewords
   //                             sorted_codewords && c.entries > 8
   let case = if c.entries > 8 {
       c.sorted_codewords.is_null() == false
   }else{
       c.codewords.is_null()
   };
   if case {
      // binary search
      let code : u32 = bit_reverse(f.acc) as u32;
      let mut x : i32 = 0;
      let mut n : i32 = c.sorted_entries;
      let len : i32;

      while n > 1 {
         // invariant: sc[x] <= code < sc[x+n]
         let m = x + (n >> 1);
         if *c.sorted_codewords.offset(m as isize) <= code {
            x = m;
            n -= n>>1;
         } else {
            n >>= 1;
         }
      }
      // x is now the sorted index
      if c.sparse == false {
          x = *c.sorted_values.offset(x as isize);
      }
      // x is now sorted index if sparse, or symbol otherwise
      len = *c.codeword_lengths.offset(x as isize) as i32;
      if f.valid_bits >= len {
         f.acc >>= len;
         f.valid_bits -= len;
         return x;
      }

      f.valid_bits = 0;
      return -1;
   }

   // if small, linear search
   assert!(c.sparse == false);
   for i in 0 .. c.entries  {
      if *c.codeword_lengths.offset(i as isize) as i32 == NO_CODE {continue;}
      if *c.codewords.offset(i as isize) == (f.acc & ((1 << *c.codeword_lengths.offset(i as isize))-1)) {
         if f.valid_bits >= *c.codeword_lengths.offset(i as isize) as i32 {
            f.acc >>= *c.codeword_lengths.offset(i as isize);
            f.valid_bits -= *c.codeword_lengths.offset(i as isize) as i32;
            return i;
         }
         f.valid_bits = 0;
         return -1;
      }
   }

   error(f, STBVorbisError::VORBIS_invalid_stream as i32);
   f.valid_bits = 0;
   return -1;
}

unsafe fn codebook_decode_step(f: &mut vorb, c: &Codebook, output: *mut f32, mut len: i32 , step: i32 ) -> i32
{
    // NOTE(bungcip): convert return type to bool?
    
   let mut z = codebook_decode_start(f,c);
   let mut last : f32 = CODEBOOK_ELEMENT_BASE!(c);
   if z < 0 {return 0;} // false
   if len > c.dimensions { len = c.dimensions; }

   z *= c.dimensions;
   for i in 0 .. len  {
      let val : f32 = CODEBOOK_ELEMENT_FAST!(c, z+i) + last;
      *output.offset( (i*step) as isize) += val;
      if c.sequence_p != 0 {last = val;}
   }

   return 1; // true
}

unsafe fn codebook_decode_deinterleave_repeat(f: &mut vorb, c: &Codebook, outputs: *mut *mut f32, ch: i32, i32er_p: &mut i32, p_inter_p: &mut i32, len: i32, mut total_decode: i32) -> i32
{
   let mut i32er = *i32er_p;
   let mut p_inter = *p_inter_p;
   let mut effective = c.dimensions;
   let mut z : i32;

   // type 0 is only legal in a scalar context
   if c.lookup_type == 0 {
     return error(f, STBVorbisError::VORBIS_invalid_stream as i32);
   } 
   while total_decode > 0 {
      let mut last : f32 = CODEBOOK_ELEMENT_BASE!(c);
      DECODE_VQ!(z,f,c);
      assert!(c.sparse == false || z < c.sorted_entries);
      if z < 0 {
         if f.bytes_in_seg == 0{
            if f.last_seg != 0{
              return 0; // false
            } 
         }
         return error(f, STBVorbisError::VORBIS_invalid_stream as i32);
      }

      // if this will take us off the end of the buffers, stop short!
      // we check by computing the length of the virtual interleaved
      // buffer (len*ch), our current offset within it (p_inter*ch)+(i32er),
      // and the length we'll be using (effective)
      if i32er + p_inter*ch + effective > len * ch {
         effective = len*ch - (p_inter*ch - i32er);
      }

      {
         z *= c.dimensions;
         if c.sequence_p != 0 {
            for i in 0 .. effective {
               let val : f32 = CODEBOOK_ELEMENT_FAST!(c,z+i) + last;
               if (*outputs.offset(i32er as isize)).is_null() == false {
                  *(*outputs.offset(i32er as isize)).offset(p_inter as isize) += val;
               }
               i32er += 1;
               if i32er == ch { i32er = 0; p_inter += 1; }
               last = val;
            }
         } else {
            for i in 0 .. effective {
               let val : f32 = CODEBOOK_ELEMENT_FAST!(c,z+i) + last;
               if (*outputs.offset(i32er as isize)).is_null() == false {
                  *(*outputs.offset(i32er as isize)).offset(p_inter as isize) += val;
               }
               i32er += 1;
               if i32er == ch { i32er = 0; p_inter += 1; }
            }
         }
      }

      total_decode -= effective;
   }
   *i32er_p = i32er;
   *p_inter_p = p_inter;

   return 1; // true
}

unsafe fn compute_window(n: i32, window: *mut f32)
{
   let n2 : i32 = n >> 1;
   for i in 0 .. n2 {
      *window.offset(i as isize) = 
            f64::sin(
                0.5 as f64 * 
                M_PI as f64 * 
                square(
                    f64::sin((i as f64 - 0 as f64 + 0.5) / n2 as f64 * 0.5 * M_PI as f64) as f32
                ) as f64
            ) as f32;
   }
}

unsafe fn vorbis_alloc(f: &mut stb_vorbis) -> *mut stb_vorbis
{
   let p : *mut stb_vorbis = setup_malloc(f, std::mem::size_of::<stb_vorbis>() as i32)  as *mut stb_vorbis;
   return p;
}

unsafe fn vorbis_deinit(p: &mut stb_vorbis)
{
   if p.residue_config.is_null() == false {
      for i in 0 .. p.residue_count {
         let r: &mut Residue = std::mem::transmute(p.residue_config.offset(i as isize));
         if r.classdata.is_null() == false {
            for j in 0 .. (*p.codebooks.offset(r.classbook as isize)).entries {
               setup_free(p, (*r.classdata.offset(j as isize)) as *mut c_void);
            }
            setup_free(p, r.classdata as *mut c_void);
         }
         setup_free(p, r.residue_books as *mut c_void);
      }
   }
   if p.codebooks.is_null() == false {
      CHECK!(p);
      for i in 0 .. p.codebook_count {
         let c: &mut Codebook = std::mem::transmute(p.codebooks.offset(i as isize));
         setup_free(p, c.codeword_lengths as *mut c_void);
         setup_free(p, c.multiplicands as *mut c_void);
         setup_free(p, c.codewords as *mut c_void);
         setup_free(p, c.sorted_codewords as *mut c_void);
         // c.sorted_values[-1] is the first entry in the array
         setup_free(p, if c.sorted_values.is_null() == false {
                c.sorted_values.offset(-1)
             }else {
                std::ptr::null_mut()
             } as *mut c_void
          );
      }
      
      { let x1 = p.codebooks as *mut c_void; setup_free(p, x1); }
   }
   
   { let x2 = p.floor_config as *mut c_void; setup_free(p, x2); }
   { let x3 = p.residue_config as *mut c_void; setup_free(p, x3); }
   if p.mapping.is_null() == false {
      for i in 0 .. p.mapping_count {
        { let x4 = (*p.mapping.offset(i as isize)).chan as *mut c_void; setup_free(p, x4); }
      }
      { let x5 = p.mapping as *mut c_void; setup_free(p, x5); }
   }
   CHECK!(p);
    let mut i = 0;
    while i < p.channels && i < STB_VORBIS_MAX_CHANNELS {
      { let x6 = p.channel_buffers[i as usize] as *mut c_void; setup_free(p, x6); }
      { let x7 = p.previous_window[i as usize] as *mut c_void; setup_free(p, x7); }
      { let x8 = p.finalY[i as usize] as *mut c_void; setup_free(p, x8); }
      
      i += 1;
   }
   
   for i in 0 .. 2 {
      { let x9 = p.A[i as usize] as *mut c_void; setup_free(p, x9); }
      { let x10 = p.B[i as usize] as *mut c_void; setup_free(p, x10); }
      { let x11 = p.C[i as usize] as *mut c_void; setup_free(p, x11); }
      { let x12 = p.window[i as usize] as *mut c_void; setup_free(p, x12); }
      { let x13 = p.bit_reverse[i as usize] as *mut c_void; setup_free(p, x13); }
   }
   
   if p.close_on_free == true {
       libc::fclose(p.f);
   }
}

unsafe fn compute_samples(mask: i32, output: *mut i16, num_c: i32, data: *mut *mut f32, d_offset: i32, len: i32)
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   
   const BUFFER_SIZE : usize = 32;
   let mut buffer: [f32; BUFFER_SIZE];
//    int i,j,o,
   let mut n = BUFFER_SIZE as i32;
//    check_endianness();

   let mut o : i32 = 0;
   while o < len {
    //   memset(buffer, 0, sizeof(buffer));
      buffer = std::mem::zeroed();
      
      if o + n > len {
          n = len - o;
      }
      for j in 0 .. num_c {
         if (channel_position[num_c as usize][j as usize] as i32 & mask) != 0 {
            for i in 0 .. n {
               buffer[i as usize] += *(*data.offset(j as isize)).offset( (d_offset+o+i) as isize);
            }
         }
      }
      for i in 0 .. n  {
         let mut v : i32 = FAST_SCALED_FLOAT_TO_INT!(buffer[i as usize], 15);
         if (v + 32768) as u32 > 65535{
            v = if v < 0 { -32768 } else { 32767 };
         }
         *output.offset( (o+i as i32) as isize) = v as i16;
      }
       
       o += BUFFER_SIZE as i32;
   }
}

unsafe fn compute_stereo_samples(output: *mut i16, num_c: i32, data: *mut *mut f32, d_offset: i32, len: i32)
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   
   const BUFFER_SIZE : usize = 32;
   let mut buffer: [f32; BUFFER_SIZE];
   
//    int i,j,o,
   let mut n : i32 = BUFFER_SIZE as i32 >> 1;
   // o is the offset in the source data
//    check_endianness();
   let mut o : i32 = 0;
   while o < len {
      // o2 is the offset in the output data
      let o2 : i32 = o << 1;
      buffer = std::mem::zeroed();
      
      if o + n > len {
          n = len - o;
      }
      for j in 0 .. num_c {
         let m : i32 = channel_position[num_c as usize][j as usize] as i32 & (PLAYBACK_LEFT | PLAYBACK_RIGHT);
         if m == (PLAYBACK_LEFT | PLAYBACK_RIGHT) {
            for i in 0 .. n {
               buffer[ (i*2+0) as usize] += *(*data.offset(j as isize)).offset( (d_offset+o+i) as isize);
               buffer[ (i*2+1) as usize] += *(*data.offset(j as isize)).offset( (d_offset+o+i) as isize);
            }
         } else if m == PLAYBACK_LEFT {
            for i in 0 .. n {
               buffer[ (i*2+0) as usize] += *(*data.offset(j as isize)).offset( (d_offset+o+i) as isize);
            }
         } else if m == PLAYBACK_RIGHT {
            for i in 0 .. n  {
               buffer[(i*2+1) as usize] += *(*data.offset(j as isize)).offset( (d_offset+o+i) as isize);
            }
         }
      }
      
      
      for i in 0 .. n << 1 {
         let mut v : i32 = FAST_SCALED_FLOAT_TO_INT!(buffer[i as usize],15);
         if (v + 32768) as u32 > 65535 {
            v = if v < 0 {-32768} else {32767};
         }
         *output.offset((o2+i) as isize) = v as i16;
      }
       
       o += (BUFFER_SIZE >> 1) as i32;
   }

}

// these functions seek in the Vorbis file to (approximately) 'sample_number'.
// after calling seek_frame(), the next call to get_frame_*() will include
// the specified sample. after calling stb_vorbis_seek(), the next call to
// stb_vorbis_get_samples_* will start with the specified sample. If you
// do not need to seek to EXACTLY the target sample when using get_samples_*,
// you can also use seek_frame().
pub unsafe fn stb_vorbis_seek_frame(f: &mut stb_vorbis, sample_number: u32) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   
   let max_frame_samples: u32;

   if IS_PUSH_MODE!(f) { return error(f, STBVorbisError::VORBIS_invalid_api_mixing as i32);}

   // fast page-level search
   if seek_to_sample_coarse(f, sample_number) == 0{
      return 0;
   }

   assert!(f.current_loc_valid != 0);
   assert!(f.current_loc <= sample_number);

   // linear search for the relevant packet
   max_frame_samples = ((f.blocksize_1*3 - f.blocksize_0) >> 2) as u32;
   while f.current_loc < sample_number {
      let mut left_start: i32 = 0; 
      let mut left_end: i32 = 0;
      let mut right_start: i32 = 0;
      let mut right_end: i32 = 0;
      let mut mode: i32 = 0;
      let frame_samples: i32;
      if peek_decode_initial(f, &mut left_start, &mut left_end, &mut right_start, &mut right_end, &mut mode) == 0{
         return error(f, STBVorbisError::VORBIS_seek_failed as i32);
      }
      // calculate the number of samples returned by the next frame
      frame_samples = right_start - left_start;
      if f.current_loc as i32 + frame_samples > sample_number as i32 {
         return 1; // the next frame will contain the sample
      } else if f.current_loc as i32 + frame_samples + max_frame_samples as i32 > sample_number as i32 {
         // there's a chance the frame after this could contain the sample
         vorbis_pump_first_frame(f);
      } else {
         // this frame is too early to be relevant
         f.current_loc += frame_samples as u32;
         f.previous_length = 0;
         maybe_start_packet(f);
         flush_packet(f);
      }
   }
   // the next frame will start with the sample
   assert!(f.current_loc == sample_number);
   return 1;
}

// get the last error detected (clears it, too)
pub fn stb_vorbis_get_error(f: &mut stb_vorbis) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let e = f.error;
   f.error = STBVorbisError::VORBIS__no_error as i32;
   return e;
}

// this function is equivalent to stb_vorbis_seek(f,0)
pub unsafe fn stb_vorbis_seek_start(f: &mut stb_vorbis)
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   if IS_PUSH_MODE!(f) { error(f, STBVorbisError::VORBIS_invalid_api_mixing as i32); return; }
   
   let offset = f.first_audio_page_offset;
   set_file_offset(f, offset);
   f.previous_length = 0;
   f.first_decode = 1; // true
   f.next_seg = -1;
   vorbis_pump_first_frame(f);
}

// these functions return the total length of the vorbis stream
pub unsafe fn stb_vorbis_stream_length_in_seconds(f: &mut stb_vorbis) -> f32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   return stb_vorbis_stream_length_in_samples(f) as f32 / f.sample_rate as f32;
}

// this function returns the offset (in samples) from the beginning of the
// file that will be returned by the next decode, if it is known, or -1
// otherwise. after a flush_pushdata() call, this may take a while before
// it becomes valid again.
// NOT WORKING YET after a seek with PULLDATA API
pub fn stb_vorbis_get_sample_offset(f: &mut stb_vorbis) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   if f.current_loc_valid != 0 {
      return f.current_loc as i32;
   } else{
      return -1;
   }
}

// get general information about the file
pub fn stb_vorbis_get_info(f: &stb_vorbis) -> stb_vorbis_info
{
   stb_vorbis_info {
       channels: f.channels,
       sample_rate: f.sample_rate,
       setup_memory_required: f.setup_memory_required,
       setup_temp_memory_required: f.setup_temp_memory_required,
       temp_memory_required: f.temp_memory_required,
       max_frame_size: f.blocksize_1 >> 1
   }
}

// inform stb_vorbis that your next datablock will not be contiguous with
// previous ones (e.g. you've seeked in the data); future attempts to decode
// frames will cause stb_vorbis to resynchronize (as noted above), and
// once it sees a valid Ogg page (typically 4-8KB, as large as 64KB), it
// will begin decoding the _next_ frame.
//
// if you want to seek using pushdata, you need to seek in your file, then
// call stb_vorbis_flush_pushdata(), then start calling decoding, then once
// decoding is returning you data, call stb_vorbis_get_sample_offset, and
// if you don't like the result, seek your file again and repeat.
pub fn stb_vorbis_flush_pushdata(f: &mut stb_vorbis)
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   f.previous_length = 0;
   f.page_crc_tests  = 0;
   f.discard_samples_deferred = 0;
   f.current_loc_valid = 0; // false
   f.first_decode = 0; // false
   f.samples_output = 0;
   f.channel_buffer_start = 0;
   f.channel_buffer_end = 0;
}

// create an ogg vorbis decoder from an ogg vorbis stream in memory (note
// this must be the entire stream!). on failure, returns NULL and sets *error
pub unsafe fn stb_vorbis_open_memory(data: *const u8, len: i32, error: *mut i32, alloc: *const stb_vorbis_alloc) -> *mut stb_vorbis
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

//    stb_vorbis *f, p;
   if data.is_null() {
     return std::ptr::null_mut();       
   } 
   
   let mut p : stb_vorbis = std::mem::zeroed();
   vorbis_init(&mut p, alloc);
   
   p.stream = data as *mut u8;
   p.stream_end = data.offset(len as isize) as *mut u8;
   p.stream_start = p.stream;
   p.stream_len = len as u32;
   p.push_mode = false;
   
   if start_decoder(&mut p) != 0 {
      let f = vorbis_alloc(&mut p);
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

// decode an entire file and output the data interleaved into a malloc()ed
// buffer stored in *output. The return value is the number of samples
// decoded, or -1 if the file could not be opened or was not an ogg vorbis file.
// When you're done with it, just free() the pointer returned in *output.
pub unsafe fn stb_vorbis_decode_memory(mem: *const u8, len: i32 , channels: *mut i32, sample_rate: *mut i32, output: *mut *mut i16) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

    let mut error : i32 = 0;
   let v : *mut stb_vorbis = stb_vorbis_open_memory(mem, len, &mut error, std::ptr::null_mut());
   if v.is_null() {
       return -1;
   }
   let v : &mut stb_vorbis = std::mem::transmute(v);
   
   let limit : i32 = v.channels as i32 * 4096;
   *channels = v.channels;
   if sample_rate.is_null() == false {
      *sample_rate = v.sample_rate as i32;
   }
   let mut offset = 0;
   let mut data_len = 0;
   let mut total = limit;
   let mut data : *mut i16 = libc::malloc( (total as usize * std::mem::size_of::<i16>()) ) as *mut i16;
   if data.is_null() {
      stb_vorbis_close(v);
      return -2;
   }
   loop {
       let ch = v.channels;
      let n = stb_vorbis_get_frame_short_interleaved(v, ch, data.offset(offset as isize), total-offset);
      
      if n == 0 {break;}
      data_len += n;
      offset += n * v.channels;
      if offset + limit > total {
         total *= 2;
         let data2: *mut i16 = libc::realloc(data as *mut c_void, (total as usize * std::mem::size_of::<i16>()) ) as *mut i16;
         if data2.is_null() {
            libc::free(data as *mut c_void);
            stb_vorbis_close(v);
            return -2;
         }
         data = data2;
      }
   }
   *output = data;
   stb_vorbis_close(v);
   return data_len;
}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. DOES NOT APPLY THE COERCION RULES.
// Returns the number of samples stored per channel; it may be less than requested
// at the end of the file. If there are no more samples in the file, returns 0.
pub unsafe fn stb_vorbis_get_samples_float(f: &mut stb_vorbis, channels: i32 , buffer: *mut *mut f32, num_samples: i32 ) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let mut outputs: *mut *mut f32 = std::mem::zeroed();
   let mut n = 0;
   let mut z = f.channels;
   if z > channels {z = channels;}
   while n < num_samples {
      let mut k = f.channel_buffer_end - f.channel_buffer_start;
      if n + k >= num_samples { k = num_samples - n; }
      if k != 0 {
          let mut i = 0;
          while i < z {
            // memcpy(buffer[i]+n, f.channel_buffers[i]+f.channel_buffer_start, sizeof(float)*k);
            std::ptr::copy_nonoverlapping(
                (*buffer.offset(i as isize)).offset(n as isize),
                f.channel_buffers[i as usize].offset(f.channel_buffer_start as isize),
                std::mem::size_of::<f32>() * k as usize
            );
            i += 1;
          }
          
          while i < channels {
            // memset(buffer[i]+n, 0, sizeof(float) * k);
            std::ptr::write_bytes(
                (*buffer.offset(i as isize)).offset(n as isize),
                0,
                k as usize
            );        
            i += 1;
          }          
      }
      n += k;
      f.channel_buffer_start += k;
      if n == num_samples{
         break;
      }
      if stb_vorbis_get_frame_float(f, std::ptr::null_mut(), &mut outputs) == 0 {
         break;
      }
   }
   return n;
}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. DOES NOT APPLY THE COERCION RULES.
// Returns the number of samples stored per channel; it may be less than requested
// at the end of the file. If there are no more samples in the file, returns 0.
pub unsafe fn stb_vorbis_get_samples_float_interleaved(f: &mut stb_vorbis, channels: i32 , mut buffer: *mut f32, num_floats: i32 ) -> i32 
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let mut outputs: *mut *mut f32 = std::mem::zeroed();
   let len : i32 = num_floats / channels;
   let mut n=0;
   let mut z = f.channels;
   if z > channels {z = channels;}
   while n < len {
    //   int i,j;
      let mut k = f.channel_buffer_end - f.channel_buffer_start;
      if n+k >= len {k = len - n;}
      for j in 0 .. k  {
          let mut i = 0;
          while i < z {
            *buffer = *f.channel_buffers[i as usize].offset( (f.channel_buffer_start+j) as isize);
            buffer = buffer.offset(1);
              i += 1;
          }
          
          while i < channels {
            *buffer = 0.0;
            buffer = buffer.offset(1);
              i += 1;
          }
      }
      n += k;
      f.channel_buffer_start += k;
      if n == len{
         break;
      }
      if stb_vorbis_get_frame_float(f, std::ptr::null_mut(), &mut outputs) == 0{
         break;
      }
   }
   return n;
}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. Applies the coercion rules above
// to produce 'channels' channels. Returns the number of samples stored per channel;
// it may be less than requested at the end of the file. If there are no more
// samples in the file, returns 0.
pub unsafe fn stb_vorbis_get_samples_short(f: &mut stb_vorbis, channels: i32, buffer: *mut *mut i16, len: i32) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let mut outputs: *mut *mut f32 = std::mem::zeroed();
   let mut n = 0;
   let z = f.channels;
   
   // NOTE(bungcip): useless code?
//    if z > channels {z = channels;}

   while n < len {
      let mut k = f.channel_buffer_end - f.channel_buffer_start;
      if n+k >= len {k = len - n;}
      if k != 0 {
         convert_samples_short(channels, buffer, n, f.channels, f.channel_buffers.as_mut_ptr(), f.channel_buffer_start, k);
      }
      n += k;
      f.channel_buffer_start += k;
      if n == len{ break;}
      if stb_vorbis_get_frame_float(f, std::ptr::null_mut(), &mut outputs) == 0 {break;}
   }
   return n;
}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. Applies the coercion rules above
// to produce 'channels' channels. Returns the number of samples stored per channel;
// it may be less than requested at the end of the file. If there are no more
// samples in the file, returns 0.
pub unsafe fn stb_vorbis_get_samples_short_interleaved(f: &mut stb_vorbis, channels: i32, mut buffer: *mut i16, num_shorts: i32 ) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let mut outputs: *mut *mut f32 = std::mem::zeroed();
   let len = num_shorts / channels;
   let mut n = 0;
   let z = f.channels;
   // NOTE(bungcip): useless code?
//    if z > channels {z = channels;}
   while n < len {
      let mut k = f.channel_buffer_end - f.channel_buffer_start;
      if n+k >= len {k = len - n;}
      if k != 0 {
         convert_channels_short_interleaved(channels, buffer, f.channels, f.channel_buffers.as_mut_ptr(), f.channel_buffer_start, k);
      }
      buffer = buffer.offset( (k*channels) as isize);
      n += k;
      f.channel_buffer_start += k;
      if n == len{ break;}
      if stb_vorbis_get_frame_float(f, std::ptr::null_mut(), &mut outputs) == 0 {break;}
   }
   return n;
}

// the same as vorbis_decode_initial, but without advancing
unsafe fn peek_decode_initial(f: &mut vorb, p_left_start: &mut i32, p_left_end: &mut i32, p_right_start: &mut i32, p_right_end: &mut i32, mode: &mut i32) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
//    int bits_read, bytes_read;

   if vorbis_decode_initial(f, p_left_start, p_left_end, p_right_start, p_right_end, mode) == 0 {
      return 0;
   }

   // either 1 or 2 bytes were read, figure out which so we can rewind
   let mut bits_read = 1 + ilog(f.mode_count-1);
   if f.mode_config[*mode as usize].blockflag != 0 {
      bits_read += 2;
   }
   let bytes_read = (bits_read + 7) / 8;

   f.bytes_in_seg += bytes_read as u8;
   f.packet_bytes -= bytes_read;
   skip(f, -bytes_read);
   if f.next_seg == -1{
      f.next_seg = f.segment_count - 1;
   }
   else{
      f.next_seg -= 1;
   }
   f.valid_bits = 0;

   return 1;
}

unsafe fn set_file_offset(f: &mut stb_vorbis, loc: u32) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   if f.push_mode == true {return 0;}
   f.eof = false;
   if USE_MEMORY!(f) {
      if f.stream_start.offset(loc as isize)  >= f.stream_end || f.stream_start.offset(loc as isize) < f.stream_start {
         f.stream = f.stream_end;
         f.eof = true;
         return 0;
      } else {
         f.stream = f.stream_start.offset(loc as isize);
         return 1;
      }
   }
   if loc + f.f_start < loc || loc >= 0x80000000 {
      loc = 0x7fffffff;
      f.eof = true;
   } else {
      loc += f.f_start;
   }
   if libc::fseek(f.f, loc as i32, SEEK_SET) == 0{
      return 1;
   }
   f.eof = true;
   libc::fseek(f.f, f.f_start as i32, SEEK_END);
   return 0;
}

// rarely used function to seek back to the preceeding page while finding the
// start of a packet
unsafe fn go_to_page_before(f: &mut stb_vorbis, limit_offset: u32) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let previous_safe : u32;
   let mut end: u32 = 0;

   // now we want to seek back 64K from the limit
   if limit_offset >= 65536 && limit_offset-65536 >= f.first_audio_page_offset{
      previous_safe = limit_offset - 65536;
   }
   else{
      previous_safe = f.first_audio_page_offset;
   }

   set_file_offset(f, previous_safe);

   while vorbis_find_page(f, &mut end, std::ptr::null_mut()) != 0 {
      if end >= limit_offset && stb_vorbis_get_file_offset(f) < limit_offset{
         return 1;          
      }
      set_file_offset(f, end);
   }

   return 0;
}

unsafe fn vorbis_find_page(f: &mut stb_vorbis, end: *mut u32, last: *mut u32) -> u32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   loop {
      if f.eof == true {return 0;}
      let n : i32 = get8(f) as i32;
      if n == 0x4f { // page header candidate
         let retry_loc = stb_vorbis_get_file_offset(f);
         
         'invalid : loop {
        //  int i;
         // check if we're off the end of a file_section stream
         if retry_loc - 25 > f.stream_len{
            return 0;
         }
         // check the rest of the header
         let mut i = 1;
         while i < 4 {
            if get8(f) != ogg_page_header[i]{
               break;
            }
             i += 1;
         }
         if f.eof == true {return 0;}
         if i == 4 {
            let mut header: [u8; 27] = std::mem::zeroed();
            // uint32 i, crc, goal, len;
            let mut i : usize = 0;
            while i < 4 {
               header[i] = ogg_page_header[i];
                i += 1;
            }
            while i < 27 {
               header[i] = get8(f);
                
                i += 1;
            }
            
            if f.eof == true {return 0;}
            
            if header[4] != 0 {
                // goto invalid;
                break 'invalid;
            }
            let goal : u32 = header[22] as u32 + 
                ((header[23] as u32) << 8) + 
                ((header[24] as u32) << 16) + 
                ((header[25] as u32) << 24);
            
            i = 22;
            while i < 26 {
               header[i] = 0;
                i += 1;
            }
            let mut crc = 0;
            for i in 0usize .. 27 {
               crc = crc32_update(crc, header[i]);
            }
            let mut len = 0;
            for i in 0 .. header[26] {
               let s = get8(f) as i32;
               crc = crc32_update(crc, s as u8);
               len += s;
            }
            if len != 0 && f.eof == true {return 0;}
            for i in 0 .. len {
               crc = crc32_update(crc, get8(f));
            }
            // finished parsing probable page
            if crc == goal as u32 {
               // we could now check that it's either got the last
               // page flag set, OR it's followed by the capture
               // pattern, but I guess TECHNICALLY you could have
               // a file with garbage between each ogg page and recover
               // from it automatically? So even though that paranoia
               // might decrease the chance of an invalid decode by
               // another 2^32, not worth it since it would hose those
               // invalid-but-useful files?
               if end.is_null() == false {
                  *end = stb_vorbis_get_file_offset(f);
               }
               if last.is_null() == false {
                  if (header[5] & 0x04) != 0 {
                     *last = 1;
                  }else{
                     *last = 0;
                  }
               }
               set_file_offset(f, retry_loc-1);
               return 1;
            }
         }
         
         break;
         }// loop 
        // invalid:
         // not a valid page, so rewind and look for next one
         set_file_offset(f, retry_loc);
      }
   }
}

#[inline(always)]
unsafe fn crc32_update(crc: u32, byte: u8) -> u32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   return (crc << 8) ^ crc_table[ (byte as u32 ^ (crc >> 24)) as usize];
}

// given a sufficiently large block of memory, make an array of pointers to subblocks of it
unsafe fn make_block_array(mem: *mut c_void, count: i32, size: usize) -> *mut c_void
{
   let p : *mut *mut c_void  = std::mem::transmute(mem);
   let mut q : *mut i8 = p.offset(count as isize) as *mut i8;
   for i in 0 .. count {
      *p.offset(i as isize) = q as *mut c_void;
      q = q.offset(size as isize);
   }
   return p as *mut c_void;
}

// seeking is implemented with a binary search, which narrows down the range to
// 64K, before using a linear search (because finding the synchronization
// pattern can be expensive, and the chance we'd find the end page again is
// relatively high for small ranges)
//
// two initial interpolation-style probes are used at the start of the search
// to try to bound either side of the binary search sensibly, while still
// working in O(log n) time if they fail.

unsafe fn get_seek_page_info(f: &mut stb_vorbis, z: &mut ProbedPage) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let mut header: [u8; 27] = std::mem::zeroed();
   let mut lacing: [u8; 255] = std::mem::zeroed();
//    int i,len;

   // record where the page starts
   z.page_start = stb_vorbis_get_file_offset(f);

   // parse the header
   getn(f, header.as_mut_ptr(), 27);
   if header[0] != b'O' || header[1] != b'g' || header[2] != b'g' || header[3] != b'S'{
      return 0;
   }
   getn(f, lacing.as_mut_ptr(), header[26] as i32);

   // determine the length of the payload
   let mut len = 0;
   for i in 0 .. header[26] {
      len += lacing[i as usize];
   }

   // this implies where the page ends
   z.page_end = z.page_start + 27 + header[26] as u32 + len as u32;

   // read the last-decoded sample out of the data
   z.last_decoded_sample = header[6] as u32 + 
    ( (header[7] as u32) << 8) + 
    ( (header[8] as u32) << 16) + 
    ( (header[9] as u32) << 24);

   // restore file state to where we were
   set_file_offset(f, z.page_start);
   return 1;
}

// create a vorbis decoder by passing in the initial data block containing
//    the ogg&vorbis headers (you don't need to do parse them, just provide
//    the first N bytes of the file--you're told if it's not enough, see below)
// on success, returns an stb_vorbis *, does not set error, returns the amount of
//    data parsed/consumed on this call in *datablock_memory_consumed_in_bytes;
// on failure, returns NULL on error and sets *error, does not change *datablock_memory_consumed
// if returns NULL and *error is VORBIS_need_more_data, then the input block was
//       incomplete and you need to pass in a larger block from the start of the file
pub unsafe fn stb_vorbis_open_pushdata(
         data: *const u8, data_len: i32, // the memory available for decoding
         data_used: *mut i32,              // only defined if result is not NULL
         error: *mut i32, alloc: *const stb_vorbis_alloc)
         -> *mut stb_vorbis
{

   let mut p : stb_vorbis = std::mem::zeroed();
   vorbis_init(&mut p, alloc);
   p.stream     = data as *mut u8;
   p.stream_end = data.offset(data_len as isize) as *mut u8;
   p.push_mode  = true;
   if start_decoder(&mut p) == 0 {
      if p.eof == true {
         *error = STBVorbisError::VORBIS_need_more_data as i32;
      } else {
         *error = p.error;
      }
      return std::ptr::null_mut();
   }
   let mut f = vorbis_alloc(&mut p);
   if f.is_null() == false {
      *f = p;
      *data_used = ((*f).stream as usize - data as usize) as i32;
      *error = 0;
      return f;
   } else {
      vorbis_deinit(&mut p);
      return std::ptr::null_mut();
   }
}

// decode a frame of audio sample data if possible from the passed-in data block
//
// return value: number of bytes we used from datablock
//
// possible cases:
//     0 bytes used, 0 samples output (need more data)
//     N bytes used, 0 samples output (resynching the stream, keep going)
//     N bytes used, M samples output (one frame of data)
// note that after opening a file, you will ALWAYS get one N-bytes,0-sample
// frame, because Vorbis always "discards" the first frame.
//
// Note that on resynch, stb_vorbis will rarely consume all of the buffer,
// instead only datablock_length_in_bytes-3 or less. This is because it wants
// to avoid missing parts of a page header if they cross a datablock boundary,
// without writing state-machiney code to record a partial detection.
//
// The number of channels returned are stored in *channels (which can be
// NULL--it is always the same as the number of channels reported by
// get_info). *output will contain an array of float* buffers, one per
// channel. In other words, (*output)[0][0] contains the first sample from
// the first channel, and (*output)[1][0] contains the first sample from
// the second channel.

// return value: number of bytes we used
pub unsafe fn stb_vorbis_decode_frame_pushdata(
         f: &mut stb_vorbis,                   // the file we're decoding
         data: *const u8, data_len: i32 , // the memory available for decoding
         channels: *mut i32,                   // place to write number of float * buffers
         output: *mut *mut *mut f32,                 // place to write float ** array of float * buffers
         samples: *mut i32                     // place to write number of output samples
     ) -> i32
{

   if IS_PUSH_MODE!(f) == false{ return error(f, STBVorbisError::VORBIS_invalid_api_mixing as i32) };

   if f.page_crc_tests >= 0 {
      *samples = 0;
      return vorbis_search_for_page_pushdata(f, data as *mut u8, data_len);
   }

   f.stream     = data as *mut u8;
   f.stream_end = data.offset(data_len as isize) as *mut u8;
   f.error      = STBVorbisError::VORBIS__no_error as i32;

   // check that we have the entire packet in memory
   if is_whole_packet_present(f, 0) == 0 { // false
      *samples = 0;
      return 0;
   }

   let mut len : i32 = 0;
   let mut left: i32 = 0;
   let mut right: i32 = 0;
   if vorbis_decode_packet(f, &mut len, &mut left, &mut right) == 0 {
      // save the actual error we encountered
      let error = f.error;
      if error == STBVorbisError::VORBIS_bad_packet_type as i32 {
         // flush and resynch
         f.error = STBVorbisError::VORBIS__no_error as i32;
         while get8_packet(f) != EOP{
            if f.eof == true {break;}
         }
         *samples = 0;
         return (f.stream as usize - data as usize) as i32;
      }
      if error == STBVorbisError::VORBIS_continued_packet_flag_invalid as i32 {
         if f.previous_length == 0 {
            // we may be resynching, in which case it's ok to hit one
            // of these; just discard the packet
            f.error = STBVorbisError::VORBIS__no_error as i32;
            while get8_packet(f) != EOP{
                if f.eof == true {break;}
            }
            *samples = 0;
            return (f.stream as usize - data as usize) as i32;
         }
      }
      // if we get an error while parsing, what to do?
      // well, it DEFINITELY won't work to continue from where we are!
      stb_vorbis_flush_pushdata(f);
      // restore the error that actually made us bail
      f.error = error;
      *samples = 0;
      return 1;
   }

   // success!
   let len = vorbis_finish_frame(f, len, left, right);
   for i in 0 .. f.channels {
      f.outputs[i as usize] = f.channel_buffers[i as usize].offset(left as isize);
   }

   if channels.is_null() == false {
       *channels = f.channels;
   }
   *samples = len;
   *output = f.outputs.as_mut_ptr();
    return (f.stream as usize - data as usize) as i32;
}


unsafe fn is_whole_packet_present(f: &mut stb_vorbis, end_page: i32) -> i32
{
   // make sure that we have the packet available before continuing...
   // this requires a full ogg parse, but we know we can fetch from f->stream

   // instead of coding this out explicitly, we could save the current read state,
   // read the next packet with get8() until end-of-packet, check f->eof, then
   // reset the state? but that would be slower, esp. since we'd have over 256 bytes
   // of state to restore (primarily the page segment table)

   let mut s = f.next_seg;
   let mut first = 1; // true
   let mut p = f.stream;

   if s != -1 { // if we're not starting the packet with a 'continue on next page' flag
      while s < f.segment_count {
         p = p.offset( f.segments[s as usize] as isize);
         if f.segments[s as usize] < 255{               // stop at first short segment
            break;
         }
          
          s += 1;
      }
      
      // either this continues, or it ends it...
      if end_page != 0 {
         if s < f.segment_count-1 {
            return error(f, STBVorbisError::VORBIS_invalid_stream as i32);
         }
      }
      
      if s == f.segment_count {
         s = -1; // set 'crosses page' flag
      }
      if p > f.stream_end {
        return error(f, STBVorbisError::VORBIS_need_more_data as i32);
      }
      first = 0; // false
   }
   
   while s == -1 {
    //   uint8 *q; 
    //   int n;

      // check that we have the page header ready
      if p.offset(26) >= f.stream_end               {return error(f, STBVorbisError::VORBIS_need_more_data as i32);}
      // validate the page
      if libc::memcmp(p as *const c_void, ogg_page_header.as_ptr() as *const c_void, 4) != 0         {return error(f, STBVorbisError::VORBIS_invalid_stream as i32);}
      if *p.offset(4) != 0                             {return error(f, STBVorbisError::VORBIS_invalid_stream as i32);}
      if first != 0 { // the first segment must NOT have 'continued_packet', later ones MUST
         if f.previous_length != 0 {
            if (*p.offset(5) & PAGEFLAG_continued_packet as u8) != 0 { return error(f, STBVorbisError::VORBIS_invalid_stream as i32);}
         }
         // if no previous length, we're resynching, so we can come in on a continued-packet,
         // which we'll just drop
      } else {
         if (*p.offset(5) & PAGEFLAG_continued_packet as u8) == 0 {return error(f, STBVorbisError::VORBIS_invalid_stream as i32);}
      }
      let n = *p.offset(26); // segment counts
      let q = p.offset(27);  // q points to segment table
      p = q.offset(n as isize); // advance past header
      // make sure we've read the segment table
      if p > f.stream_end                     {return error(f, STBVorbisError::VORBIS_need_more_data as i32);}
      for s in 0 .. n {
         p = p.offset( *q.offset(s as isize) as isize);
         if *q.offset(s as isize) < 255{
            break;
         }
      }
      if end_page != 0 {
         if s < (n-1) as i32                            {return error(f, STBVorbisError::VORBIS_invalid_stream as i32);}
      }
      if s == n as i32 {
         s = -1; // set 'crosses page' flag
      }
      if p > f.stream_end                     {return error(f,STBVorbisError::VORBIS_need_more_data as i32);}
      first = 0; // false
   }
   return 1; // true
}

const SAMPLE_unknown : u32 = 0xffffffff;


// these functions return the total length of the vorbis stream
pub unsafe fn stb_vorbis_stream_length_in_samples(f: &mut stb_vorbis) -> u32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
    
//    unsigned int restore_offset, previous_safe;
//    unsigned int end, last_page_loc;

    use STBVorbisError::*;
    
    let restore_offset : u32;
    let previous_safe :u32;
    let mut end: u32 = 0;
    let mut last_page_loc :u32;

   if IS_PUSH_MODE!(f) { return error(f, VORBIS_invalid_api_mixing as i32) as u32; }
   if f.total_samples == 0 {
      let mut last : u32 = 0;
      let mut lo : u32;
      let hi : u32;
      let mut header: [i8; 6] = std::mem::zeroed();
    //   uint32 lo,hi;
    //   char header[6];
    
      'done: loop {
      // first, store the current decode position so we can restore it
      restore_offset = stb_vorbis_get_file_offset(f);

      // now we want to seek back 64K from the end (the last page must
      // be at most a little less than 64K, but let's allow a little slop)
      if f.stream_len >= 65536 && f.stream_len-65536 >= f.first_audio_page_offset {
         previous_safe = f.stream_len - 65536;
      } else {
         previous_safe = f.first_audio_page_offset;
      }

      set_file_offset(f, previous_safe);
      // previous_safe is now our candidate 'earliest known place that seeking
      // to will lead to the final page'

      if vorbis_find_page(f, &mut end, &mut last) == 0 {
         // if we can't find a page, we're hosed!
         f.error = VORBIS_cant_find_last_page as i32;
         f.total_samples = 0xffffffff;
        //  goto done;
        break 'done;
      }

      // check if there are more pages
      last_page_loc = stb_vorbis_get_file_offset(f);

      // stop when the last_page flag is set, not when we reach eof;
      // this allows us to stop short of a 'file_section' end without
      // explicitly checking the length of the section
      while last == 0 {
         set_file_offset(f, end);
         if vorbis_find_page(f, &mut end, &mut last) == 0 {
            // the last page we found didn't have the 'last page' flag
            // set. whoops!
            break;
         }
         // NOTE(bungcip): not used?
        //  previous_safe = last_page_loc+1;

         last_page_loc = stb_vorbis_get_file_offset(f);
      }

      set_file_offset(f, last_page_loc);

      // parse the header
      getn(f, header.as_mut_ptr() as *mut u8, 6);
      // extract the absolute granule position
      lo = get32(f);
      hi = get32(f);
      if lo == 0xffffffff && hi == 0xffffffff {
         f.error = VORBIS_cant_find_last_page as i32;
         f.total_samples = SAMPLE_unknown;
        //  goto done;
        break 'done;
      }
      if hi != 0{
         lo = 0xfffffffe; // saturate
      }
      f.total_samples = lo;

      f.p_last.page_start = last_page_loc;
      f.p_last.page_end   = end;
      f.p_last.last_decoded_sample = lo;

      break 'done;
     }

    //  done:
      set_file_offset(f, restore_offset);
   }
   return if f.total_samples == SAMPLE_unknown {0} else {f.total_samples};
}

// implements the search logic for finding a page and starting decoding. if
// the function succeeds, current_loc_valid will be true and current_loc will
// be less than or equal to the provided sample number (the closer the
// better).
unsafe fn seek_to_sample_coarse(f: &mut stb_vorbis, sample_number: u32) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   
   let mut left : ProbedPage;
   let mut right: ProbedPage;
   let mut mid: ProbedPage = std::mem::zeroed();
   let mut start_seg_with_known_loc : i32;
   let mut end_pos : i32;
   let mut page_start : i32;
   let mut delta: u32;
   let stream_length : u32;
   let padding : u32;
   let mut offset: f64 = 0.0;
   let mut bytes_per_sample : f64 = 0.0;
   let mut probe = 0; 
   
   use STBVorbisError::*;

   // find the last page and validate the target sample
   stream_length = stb_vorbis_stream_length_in_samples(f);
   if stream_length == 0            {return error(f, VORBIS_seek_without_length as i32);}
   if sample_number > stream_length { return error(f, VORBIS_seek_invalid as i32);}

   'error: loop {
   // this is the maximum difference between the window-center (which is the
   // actual granule position value), and the right-start (which the spec
   // indicates should be the granule position (give or take one)).
   padding = ((f.blocksize_1 - f.blocksize_0) >> 2) as u32;
   if sample_number < padding{
      sample_number = 0;
   }else{
      sample_number -= padding;
   }
   
   left = f.p_first;
   while left.last_decoded_sample == !0 {
      // (untested) the first page does not have a 'last_decoded_sample'
      set_file_offset(f, left.page_end);
      if get_seek_page_info(f, &mut left) == 0 {
        //   goto error;
        break 'error;
        }
   }

   right = f.p_last;
   assert!(right.last_decoded_sample != !0 );

   // starting from the start is handled differently
   if sample_number <= left.last_decoded_sample {
      stb_vorbis_seek_start(f);
      return 1;
   }

   while left.page_end != right.page_start {
      assert!(left.page_end < right.page_start);
      // search range in bytes
      delta = right.page_start - left.page_end;
      if delta <= 65536 {
         // there's only 64K left to search - handle it linearly
         set_file_offset(f, left.page_end);
      } else {
         if probe < 2 {
            if probe == 0 {
               // first probe (interpolate)
               let data_bytes : f64 = (right.page_end - left.page_start) as f64;
               bytes_per_sample = data_bytes / right.last_decoded_sample as f64;
               offset = left.page_start as f64 + bytes_per_sample * (sample_number - left.last_decoded_sample) as f64;
            } else {
               // second probe (try to bound the other side)
               let mut error: f64 = (sample_number as f64 - mid.last_decoded_sample as f64) * bytes_per_sample;
               if error >= 0.0 && error <  8000.0 {error =  8000.0;}
               if error <  0.0 && error > -8000.0 {error = -8000.0;}
               offset += error * 2.0;
            }

            // ensure the offset is valid
            if offset < left.page_end as f64{
               offset = left.page_end as f64;
            }
            if offset > (right.page_start - 65536) as f64{
               offset = (right.page_start - 65536) as f64;
            }
            
            set_file_offset(f, offset as u32);
         } else {
            // binary search for large ranges (offset by 32K to ensure
            // we don't hit the right page)
            set_file_offset(f, left.page_end + (delta / 2) - 32768);
         }

         if vorbis_find_page(f, std::ptr::null_mut(), std::ptr::null_mut()) == 0 {
            //  goto error;
            break 'error;
        }
      }

      loop {
         if get_seek_page_info(f, &mut mid) == 0 {
            //  goto error;
            break 'error;
         }
         if mid.last_decoded_sample != !0 {break;}
         // (untested) no frames end on this page
         set_file_offset(f, mid.page_end);
         assert!(mid.page_start < right.page_start);
      }

      // if we've just found the last page again then we're in a tricky file,
      // and we're close enough.
      if mid.page_start == right.page_start{
         break;
      }
      
      if sample_number < mid.last_decoded_sample{
         right = mid;
      }else{
         left = mid;
      }
      
      probe += 1;
   }

   // seek back to start of the last packet
   page_start = left.page_start as i32;
   set_file_offset(f, page_start as u32);
   if start_page(f) == 0 { return error(f, VORBIS_seek_failed as i32);}
   end_pos = f.end_seg_with_known_loc;
   assert!(end_pos >= 0);

   loop {
       let mut i = end_pos;
       while i > 0 {
            if f.segments[ (i-1) as usize] != 255{
                break;
            }
           i -= 1;
       }

      start_seg_with_known_loc = i;

      if start_seg_with_known_loc > 0 || (f.page_flag & PAGEFLAG_continued_packet as u8) == 0{
         break;
      }

      // (untested) the final packet begins on an earlier page
      if go_to_page_before(f, page_start as u32) == 0{
        //  goto error;
        break 'error;
      }

      page_start = stb_vorbis_get_file_offset(f) as i32;
      if start_page(f) == 0 {
        //   goto error;
        break 'error;
        }
        
      end_pos = f.segment_count - 1;
   }

   // prepare to start decoding
   f.current_loc_valid = 0; // false
   f.last_seg = 0; // false
   f.valid_bits = 0;
   f.packet_bytes = 0;
   f.bytes_in_seg = 0;
   f.previous_length = 0;
   f.next_seg = start_seg_with_known_loc;

   for i in 0 .. start_seg_with_known_loc {
       let seg = f.segments[i as usize] as i32;
      skip(f, seg);
   }

   // start decoding (optimizable - this frame is generally discarded)
   vorbis_pump_first_frame(f);
   return 1;
   } // loop -- 'error
// error:
   // try to restore the file to a valid state
   stb_vorbis_seek_start(f);
   return error(f, VORBIS_seek_failed as i32);
}

unsafe fn vorbis_search_for_page_pushdata(f: &mut vorb, data: *mut u8, mut data_len: i32) -> i32
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let mut n;
    for i in 0 .. f.page_crc_tests {
      f.scan[i as usize].bytes_done = 0;
    } 

   // if we have room for more scans, search for them first, because
   // they may cause us to stop early if their header is incomplete
   if f.page_crc_tests < STB_VORBIS_PUSHDATA_CRC_COUNT {
      if data_len < 4 {return 0;}
      data_len -= 3; // need to look for 4-byte sequence, so don't miss
                     // one that straddles a boundary
      for i in 0 .. data_len {
         if *data.offset(i as isize) == 0x4f {
            if 0 == libc::memcmp(data.offset(i as isize) as *mut c_void, ogg_page_header.as_ptr() as *const c_void, 4) {
            //    int j,len;
               let mut crc : u32;
               // make sure we have the whole page header
               if i+26 >= data_len || i + 27 + *data.offset(i as isize+26) as i32 >= data_len {
                  // only read up to this page start, so hopefully we'll
                  // have the whole page header start next time
                  data_len = i;
                  break;
               }
               // ok, we have it all; compute the length of the page
               let mut len : i32 = 27 + *data.offset(i as isize+26) as i32;
               for j in 0 .. *data.offset(i as isize + 26) {
                  len += *data.offset(i as isize+27 + j as isize) as i32;
               }
               // scan everything up to the embedded crc (which we must 0)
               crc = 0;
               for j in 0 .. 22 {
                  crc = crc32_update(crc, *data.offset(i as isize + j as isize));
               }
               // now process 4 0-bytes
               for j in 0 .. 4 {
                  crc = crc32_update(crc, 0);
               }
               let j = 26;
               // len is the total number of bytes we need to scan
               n = f.page_crc_tests;
               f.page_crc_tests += 1;
               f.scan[n as usize].bytes_left = len-j;
               f.scan[n as usize].crc_so_far = crc;
               f.scan[n as usize].goal_crc = *data.offset(i as isize + 22) as u32
                    + ((*data.offset(i as isize + 23) as u32) << 8)
                    + ((*data.offset(i as isize + 24) as u32) <<16)
                    + ((*data.offset(i as isize + 25) as u32) <<24);
               // if the last frame on a page is continued to the next, then
               // we can't recover the sample_loc immediately
               if *data.offset((i+ 27 + *data.offset(i as isize+26) as i32 - 1) as isize) == 255 {
                  f.scan[n as usize].sample_loc = !0;
               }else{
                  f.scan[n as usize].sample_loc = *data.offset(i as isize + 6) as u32
                    + ((*data.offset(i as isize + 7) as u32) <<  8)
                    + ((*data.offset(i as isize + 8) as u32) << 16)
                    + ((*data.offset(i as isize + 9) as u32) << 24);
               }
               f.scan[n as usize].bytes_done = i+j;
               if f.page_crc_tests == STB_VORBIS_PUSHDATA_CRC_COUNT{
                  break;
               }
               // keep going if we still have room for more
            }
         }
      }
   }

   let mut i = 0;
   while i < f.page_crc_tests {
      let mut crc : u32;
      let j : i32;
      let n = f.scan[i as usize].bytes_done;
      let mut m = f.scan[i as usize].bytes_left;
      if m > data_len - n {m = data_len - n;}
      // m is the bytes to scan in the current chunk
      crc = f.scan[i as usize].crc_so_far;
      for j in 0 .. m {
         crc = crc32_update(crc, *data.offset( (n+j) as isize));
      }
      f.scan[i as usize].bytes_left -= m;
      f.scan[i as usize].crc_so_far = crc;
      if f.scan[i as usize].bytes_left == 0 {
         // does it match?
         if f.scan[i as usize].crc_so_far == f.scan[i as usize].goal_crc {
            // Houston, we have page
            data_len = n+m; // consumption amount is wherever that scan ended
            f.page_crc_tests = -1; // drop out of page scan mode
            f.previous_length = 0; // decode-but-don't-output one frame
            f.next_seg = -1;       // start a new page
            f.current_loc = f.scan[i as usize].sample_loc; // set the current sample location
                                    // to the amount we'd have decoded had we decoded this page
            f.current_loc_valid =!0; 
            f.current_loc != !0;
            return data_len;
         }
         // delete entry
         f.page_crc_tests -= 1;
         f.scan[i as usize] = f.scan[f.page_crc_tests as usize];
      } else {
         i += 1;
      }
   }

   return data_len;
}

// if the fast table above doesn't work, we want to binary
// search them... need to reverse the bits

unsafe fn compute_sorted_huffman(c: &mut Codebook, lengths: *mut u8, values: *mut u32)
{
   // build a list of all the entries
   // OPTIMIZATION: don't include the short ones, since they'll be caught by FAST_HUFFMAN.
   // this is kind of a frivolous optimization--I don't see any performance improvement,
   // but it's like 4 extra lines of code, so.
   if c.sparse == false {
      let mut k = 0;
      for i in 0 .. c.entries {
         if include_in_sort(c, *lengths.offset(i as isize)) != 0 {
            *c.sorted_codewords.offset(k as isize) = bit_reverse(
                *c.codewords.offset(i as isize));
            k += 1;
         }
      }
      assert!(k == c.sorted_entries);
   } else {
      for i in 0 .. c.sorted_entries {
         *c.sorted_codewords.offset(i as isize) = bit_reverse(
                *c.codewords.offset(i as isize));
      }
   }

   qsort(
       c.sorted_codewords as *mut c_void, 
       c.sorted_entries as usize, 
       std::mem::size_of::<i32>(), 
       uint32_compare as *const c_void);
       
   *c.sorted_codewords.offset(c.sorted_entries as isize) = 0xffffffff;

   let len = if c.sparse == true  { c.sorted_entries } else { c.entries };
   // now we need to indicate how they correspond; we could either
   //   #1: sort a different data structure that says who they correspond to
   //   #2: for each sorted entry, search the original list to find who corresponds
   //   #3: for each original entry, find the sorted entry
   // #1 requires extra storage, #2 is slow, #3 can use binary search!
   for i in 0 .. len {
      let huff_len = if c.sparse == true {
          *lengths.offset(*values.offset(i as isize) as isize)
      } else {
          *lengths.offset(i as isize)
      };

      if include_in_sort(c,huff_len) != 0 {
         let code: u32 = bit_reverse(*c.codewords.offset(i as isize));
         let mut x : i32 = 0;
         let mut n : i32 = c.sorted_entries;
         while n > 1 {
            // invariant: sc[x] <= code < sc[x+n]
            let m : i32 = x + (n >> 1);
            if *c.sorted_codewords.offset(m as isize) <= code {
               x = m;
               n -= n >> 1;
            } else {
               n >>= 1;
            }
         }
         assert!(*c.sorted_codewords.offset(x as isize) == code);
         if c.sparse == true {
            *c.sorted_values.offset(x as isize) = *values.offset(i as isize) as i32;
            *c.codeword_lengths.offset(x as isize) = huff_len;
         } else {
            *c.sorted_values.offset(x as isize) = i;
         }
      }

   }
}

unsafe fn vorbis_decode_packet_rest(f: &mut vorb, len: &mut i32, m: &Mode, mut left_start: i32, left_end: i32, right_start: i32, right_end: i32, p_left: *mut i32) -> i32
{
//    Mapping *map;
//    int i,j,k,n,n2;
    let mut zero_channel: [bool; 256] = std::mem::zeroed();
    let mut really_zero_channel : [bool; 256] = std::mem::zeroed();

// WINDOWING

    let n : i32 = f.blocksize[m.blockflag as usize];
    let map: &Mapping = std::mem::transmute(f.mapping.offset(m.mapping as isize));

// FLOORS
   let n2 : i32 = n >> 1;

   CHECK!(f);

   use STBVorbisError::*;

   for i in 0 .. f.channels {
      let s: i32 = (*map.chan.offset(i as isize)).mux as i32;
      zero_channel[ i as usize ] = false; // false
      let floor : i32 = map.submap_floor[s as usize] as i32;
      if f.floor_types[floor as usize] == 0 {
         return error(f, VORBIS_invalid_stream as i32);
      } else {
          let g : &Floor1 = &(*f.floor_config.offset(floor as isize)).floor1;
         if get_bits(f, 1) != 0 {
//             short *finalY;
//             uint8 step2_flag[256];
            static range_list: [i32; 4] = [ 256, 128, 86, 64 ];
            let range = range_list[ (g.floor1_multiplier-1) as usize];
            let mut offset = 2;
            let finalY : *mut i16 = f.finalY[i as usize];
            *finalY.offset(0) = get_bits(f, ilog(range)-1) as i16;
            *finalY.offset(1) = get_bits(f, ilog(range)-1) as i16;
            for j in 0 .. g.partitions {
               let pclass = g.partition_class_list[j as usize] as usize;
               let cdim = g.class_dimensions[pclass];
               let cbits = g.class_subclasses[pclass];
               let csub = (1 << cbits)-1;
               let mut cval = 0;
               if cbits != 0 {
                  let c: &mut Codebook = std::mem::transmute(f.codebooks.offset( g.class_masterbooks[pclass] as isize));
                  DECODE!(cval,f,c);
               }
               for k in 0 .. cdim {
                  let book = g.subclass_books[pclass][ (cval & csub) as usize];
                  cval = cval >> cbits;
                  if book >= 0 {
                     let mut temp : i32;
                     let c: &mut Codebook = std::mem::transmute(f.codebooks.offset(book as isize));
                     DECODE!(temp,f,c);
                     *finalY.offset(offset) = temp as i16;
                  } else {
                     *finalY.offset(offset) = 0;
                  }
                    offset += 1;
               }
            }

            if f.valid_bits == INVALID_BITS {
                // goto error;
                zero_channel[i as usize] = true;
                continue;
            } // behavior according to spec
            
            let mut step2_flag: [u8; 256] = mem::zeroed();
            step2_flag[0] = 1; 
            step2_flag[1] = 1;
            for j in 2 .. g.values {
//                int low, high, pred, highroom, lowroom, room, val;
               let j = j as usize;
               let low = g.neighbors[j][0];
               let high = g.neighbors[j][1];
               //neighbors(g.Xlist, j, &low, &high);
               let pred = predict_point(
                   g.Xlist[j] as i32, 
                   g.Xlist[low as usize] as i32,
                   g.Xlist[high as usize] as i32, 
                   *finalY.offset(low as isize) as i32, 
                   *finalY.offset(high as isize) as i32
               );
               let val = *finalY.offset(j as isize);
               let highroom = range - pred;
               let lowroom = pred;
               let room;
               if highroom < lowroom {
                  room = highroom * 2;
               }else{
                  room = lowroom * 2;
               }
               if val != 0 {
                  step2_flag[low as usize] = 1;
                  step2_flag[high as usize] = 1;
                  step2_flag[j] = 1;
                  
                  if val >= room as i16 {
                     if highroom > lowroom {
                        *finalY.offset(j as isize) = (val - lowroom as i16 + pred as i16) as i16;
                     } else {
                        *finalY.offset(j as isize) = (pred as i16 - val + highroom as i16 - 1) as i16;
                     }
                  } else {
                     if (val & 1) != 0 {
                        *finalY.offset(j as isize) = pred as i16 - ((val+1)>>1);
                     } else {
                        *finalY.offset(j as isize) = pred as i16+ (val>>1);
                     }
                  }
               } else {
                  step2_flag[j] = 0;
                  *finalY.offset(j as isize) = pred as i16;
               }
            }

            // defer final floor computation until _after_ residue
            for j in 0 .. g.values {
               if step2_flag[j as usize] == 0 {
                  *finalY.offset(j as isize) = -1;
               }
            }
         } else {
//            error:
            zero_channel[i as usize] = true;
         }
         // So we just defer everything else to later

         // at this point we've decoded the floor into buffer
      }
   }
   // at this point we've decoded all floors

   if f.alloc.alloc_buffer.is_null() == false{
      assert!(f.alloc.alloc_buffer_length_in_bytes == f.temp_offset);
   }

   // re-enable coupled channels if necessary
   CHECK!(f);
   std::ptr::copy_nonoverlapping(zero_channel.as_ptr(), really_zero_channel.as_mut_ptr(), f.channels as usize);

   for i in 0 .. map.coupling_steps {
      let magnitude = (*map.chan.offset(i as isize)).magnitude as usize;
      let angle = (*map.chan.offset(i as isize)).angle as usize;
      
      if zero_channel[magnitude] == false || zero_channel[angle] == false {
         zero_channel[magnitude] = false;
         zero_channel[angle] = false;
      }
   }

   CHECK!(f);
// RESIDUE DECODE
   for i in 0 .. map.submaps {
      let mut residue_buffers: [*mut f32; STB_VORBIS_MAX_CHANNELS as usize] = mem::zeroed();
    //   int r;
      let mut do_not_decode: [u8; 256] = mem::zeroed();
      let mut ch : usize = 0;
      for j in 0 .. f.channels {
         if (*map.chan.offset(j as isize)).mux == i {
            if zero_channel[j as usize] {
               do_not_decode[ch] = 1; // true
               residue_buffers[ch] = std::ptr::null_mut();
            } else {
               do_not_decode[ch] = 0; // false
               residue_buffers[ch] = f.channel_buffers[j as usize];
            }
            ch += 1;
         }
      }
      let r = map.submap_residue[i as usize];
      // FIXME(bungcip): change do_not_decode to bool
      decode_residue(f, mem::transmute(&mut residue_buffers), ch as i32, n2, r as i32, do_not_decode.as_mut_ptr());
   }

   if f.alloc.alloc_buffer.is_null() == false {
      assert!(f.alloc.alloc_buffer_length_in_bytes == f.temp_offset);
   }
   CHECK!(f);

// INVERSE COUPLING
   let mut i : i32 = map.coupling_steps as i32 - 1; 
   while i >= 0 {
      let n2 = n >> 1;
      let ref c = *map.chan.offset(i as isize);
      let m : *mut f32 = f.channel_buffers[c.magnitude as usize];
      let a : *mut f32 = f.channel_buffers[c.angle  as usize];
      for j in 0 .. n2 {
         let a2 : f32;
         let m2 : f32;
         
         let j = j as isize;
         if *m.offset(j) > 0.0 {
            if *a.offset(j) > 0.0 {
               m2 = *m.offset(j);
               a2 = *m.offset(j) - *a.offset(j);
            } else {
               a2 = *m.offset(j);
               m2 = *m.offset(j) + *a.offset(j);
            }
         } else {
            if *a.offset(j) > 0.0 {
               m2 = *m.offset(j);
               a2 = *m.offset(j) + *a.offset(j);
            } else {
               a2 = *m.offset(j);
               m2 = *m.offset(j) - *a.offset(j);
            }
         }
         *m.offset(j) = m2;
         *a.offset(j) = a2;
      }
    i -= 1;
   }
   CHECK!(f);

   // finish decoding the floors
   for i in 0 .. f.channels {
      if really_zero_channel[i as usize] {
        //  memset(f.channel_buffers[i], 0, sizeof(*f.channel_buffers[i]) * n2);
        std::ptr::write_bytes(f.channel_buffers[i as usize], b'0', n2 as usize);
      } else {
          let cb = f.channel_buffers[i as usize];
          let fy = f.finalY[i as usize]; 
         do_floor(f, map, i, n, cb,
            fy, std::ptr::null_mut());
      }
   }

// INVERSE MDCT
   CHECK!(f);
   for i in 0 .. f.channels{
      inverse_mdct(f.channel_buffers[i as usize], n, f, m.blockflag as i32);
   }
   CHECK!(f);

   // this shouldn't be necessary, unless we exited on an error
   // and want to flush to get to the next packet
   flush_packet(f);

   if f.first_decode != 0 {
      // assume we start so first non-discarded sample is sample 0
      // this isn't to spec, but spec would require us to read ahead
      // and decode the size of all current frames--could be done,
      // but presumably it's not a commonly used feature
      // NOTE(bungcip): maybe this is bug?
      f.current_loc = -n2 as u32; // start of first frame is positioned for discard
      // we might have to discard samples "from" the next frame too,
      // if we're lapping a large block then a small at the start?
      f.discard_samples_deferred = n - right_end;
      f.current_loc_valid = 1; // true
      f.first_decode = 0; // false
   } else if f.discard_samples_deferred != 0 {
      if f.discard_samples_deferred >= right_start - left_start {
         f.discard_samples_deferred -= right_start - left_start;
         left_start = right_start;
         *p_left = left_start;
      } else {
         left_start += f.discard_samples_deferred;
         *p_left = left_start;
         f.discard_samples_deferred = 0;
      }
   } else if f.previous_length == 0 && f.current_loc_valid != 0 {
      // we're recovering from a seek... that means we're going to discard
      // the samples from this packet even though we know our position from
      // the last page header, so we need to update the position based on
      // the discarded samples here
      // but wait, the code below is going to add this in itself even
      // on a discard, so we don't need to do it here...
   }
   
   // check if we have ogg information about the sample # for this packet
   if f.last_seg_which == f.end_seg_with_known_loc {
      // if we have a valid current loc, and this is final:
      if f.current_loc_valid != 0 && (f.page_flag & PAGEFLAG_last_page as u8) != 0 {
         let current_end : u32 = f.known_loc_for_packet - (n-right_end) as u32;
         // then let's infer the size of the (probably) short final frame
         if current_end < f.current_loc + (right_end-left_start) as u32 {
            if current_end < f.current_loc {
               // negative truncation, that's impossible!
               *len = 0;
            } else {
               *len = (current_end - f.current_loc) as i32;
            }
            *len += left_start;
            if *len > right_end {
                *len = right_end; // this should never happen
            }
            f.current_loc += *len as u32;
            return 1; // true
         }
      }
      // otherwise, just set our sample loc
      // guess that the ogg granule pos refers to the _middle_ of the
      // last frame?
      // set f.current_loc to the position of left_start
      f.current_loc = f.known_loc_for_packet - (n2-left_start) as u32;
      f.current_loc_valid = 1; // true
   }

   if f.current_loc_valid != 0 {
       let temp_1 = (right_start - left_start) as u32;
      // NOTE(bungcip): maybe this is bug?
      f.current_loc = f.current_loc.wrapping_add(temp_1);
   }

   if f.alloc.alloc_buffer.is_null() == false {
      assert!(f.alloc.alloc_buffer_length_in_bytes == f.temp_offset);
   }
   *len = right_end;  // ignore samples after the window goes to 0
   CHECK!(f);

   return 1; // true
}

macro_rules! array_size_required {
    ($count: expr, $size: expr) => {
        ($count as usize * (::std::mem::size_of::<c_void>() + ($size as usize)))
    }
}

macro_rules! temp_alloc {
    ($f: expr, $size: expr) => {
        if $f.alloc.alloc_buffer.is_null() == false {
            setup_temp_malloc($f, $size as i32)
        }else{
            // NOTE(bungcip): for now just allocate using libc malloc & leak....
            //                rust don't have alloca() 
            // alloca(size)
            libc::malloc($size)
        }
    }
}

macro_rules! temp_alloc_save {
    ($f: expr) => {
        $f.temp_offset
    }
}

macro_rules! temp_alloc_restore {
    ($f: expr, $p: expr) => {
        $f.temp_offset = $p        
    }
}

macro_rules! temp_free {
    ($f: expr, $p: expr) => {
        0
    }
}

macro_rules! temp_alloc_save {
    ($f: expr) => ($f.temp_offset)
}


macro_rules! temp_block_array {
    ($f: expr, $count: expr, $size: expr) => {
        make_block_array(
            temp_alloc!(
                $f,
                array_size_required!($count,$size)
            ) as *mut c_void, 
            $count, $size)        
    }
}



unsafe fn decode_residue(f: &mut vorb, residue_buffers: *mut *mut f32, ch: i32, n: i32, rn: i32, do_not_decode: *mut u8)
{
//    int i,j,pass;
   let r: &Residue = mem::transmute(f.residue_config.offset(rn as isize));
   let rtype : i32 = f.residue_types[rn as usize] as i32;
   let c : i32 = r.classbook as i32;
   let classwords : i32 = (*f.codebooks.offset(c as isize)).dimensions;
   let n_read : i32 = (r.end - r.begin) as i32;
   let part_read : i32 = n_read / r.part_size as i32;
   let temp_alloc_point : i32 = temp_alloc_save!(f);
   let part_classdata: *mut *mut *mut u8 = {
       let temp_1 = f.channels;
       let zz = temp_block_array!(f,
            temp_1, part_read as usize * mem::size_of::<*mut *mut u8>() 
        );
      zz 
   } as *mut *mut *mut u8;

   CHECK!(f);

   for i in 0 .. ch {
      if *do_not_decode.offset(i as isize) == 0 {
          std::ptr::write_bytes(*residue_buffers.offset(i as isize), 0, n as usize);
      }
   }
   
   // note(bungcip): simulate goto
   'done: loop {

   if rtype == 2 && ch != 1 {
       let mut j = 0;
       while j < ch {
         if *do_not_decode.offset(j as isize) == 0 {
            break;
         }
         j += 1;
       }
       
      if j == ch {
        //  goto done;
        break 'done;
      }

      for pass in 0 .. 8 {
         let mut pcount : i32 = 0;
         let mut class_set: i32 = 0;
         if ch == 2 {
            while pcount < part_read {
               let z : i32 = r.begin as i32 + (pcount*r.part_size as i32);
               let mut i32er : i32 = z & 1;
               let mut p_inter : i32 = z>>1;
               if pass == 0 {
                  let c: &Codebook = mem::transmute(f.codebooks.offset(r.classbook as isize));
                  let mut q : i32;
                  DECODE!(q,f,c);
                  if q == EOP {
                    // goto done;
                    break 'done;  
                  } 
                  *(*part_classdata.offset(0)).offset(class_set as isize) = *r.classdata.offset(q as isize);
               }
               
               let mut i = 0;
               while i < classwords && pcount < part_read {
                  let mut z : i32 = r.begin as i32 + (pcount*r.part_size as i32);
                  let c : i32 = *(*(*part_classdata.offset(0)).offset(class_set as isize)).offset(i as isize) as i32;
                  let b : i32 = (*r.residue_books.offset(c as isize))[pass as usize] as i32;
                  if b >= 0 {
                      let book : &Codebook = mem::transmute(f.codebooks.offset(b as isize));
//                      // saves 1%
                     if codebook_decode_deinterleave_repeat(f, book, residue_buffers, ch, &mut i32er, &mut p_inter, n, r.part_size as i32) == 0{
                        // goto done;
                        break 'done;
                     }
                  } else {
                     z += r.part_size as i32;
                     i32er = z & 1;
                     p_inter = z >> 1;
                  }
                    i += 1; pcount += 1;
               }
               class_set += 1;
            }
         } else if ch == 1 {
            while pcount < part_read {
               let z : i32 = r.begin as i32 + pcount*r.part_size as i32;
               let mut i32er : i32 = 0;
               let mut p_inter = z;
               if pass == 0 {
                  let c : &Codebook = mem::transmute(f.codebooks.offset(r.classbook as isize));
                  let mut q;
                  DECODE!(q,f,c);
                  if q == EOP{
                    // goto done;
                    break 'done; 
                  } 
                  *(*part_classdata.offset(0)).offset(class_set as isize) = *r.classdata.offset(q as isize);
               }
               let mut i = 0;
               while i < classwords && pcount < part_read {
                  let mut z : i32 = r.begin as i32 + pcount*r.part_size as i32;
                  let c : i32 = *(*(*part_classdata.offset(0)).offset(class_set as isize)).offset(i as isize) as i32;
                  let b : i32 = (*r.residue_books.offset(c as isize))[pass as usize] as i32;
                  if b >= 0 {
                      let book : &Codebook = mem::transmute(f.codebooks.offset(b as isize));
                     if codebook_decode_deinterleave_repeat(f, book, residue_buffers, ch, &mut i32er, &mut p_inter, n, r.part_size as i32) == 0{
                        // goto done;
                        break 'done;
                     }
                  } else {
                     z += r.part_size as i32;
                     i32er = 0;
                     p_inter = z;
                  }
                    i += 1; pcount += 1;
               }
               class_set += 1;
            }
         } else {
            while pcount < part_read {
                let z : i32 = r.begin as i32 + pcount*r.part_size as i32;
               let mut i32er : i32 = z % ch;
               let mut p_inter = z/ch;
               if pass == 0 {
                  let c : &Codebook = mem::transmute(f.codebooks.offset(r.classbook as isize));
                  let mut q;
                  DECODE!(q,f,c);
                  if q == EOP{
                    // goto done;
                    break 'done;  
                  } 
                  *(*part_classdata.offset(0)).offset(class_set as isize) = *r.classdata.offset(q as isize);
               }
               let mut i = 0;
               while i < classwords && pcount < part_read {
                  let mut z : i32 = r.begin as i32 + pcount*r.part_size as i32;
                  let c : i32 = *(*(*part_classdata.offset(0)).offset(class_set as isize)).offset(i as isize) as i32;
                  let b : i32 = (*r.residue_books.offset(c as isize))[pass as usize] as i32;
                  if b >= 0 {
                      let book : &Codebook = mem::transmute(f.codebooks.offset(b as isize));
                     if codebook_decode_deinterleave_repeat(f, book, residue_buffers, ch, &mut i32er, &mut p_inter, n, r.part_size as i32) == 0{
                        // goto done;
                        break 'done;
                     }
                  } else {
                     z += r.part_size as i32;
                     i32er = z % ch;
                     p_inter = z / ch;
                  }
                    i += 1; pcount += 1;
               }
               class_set += 1;
            }
         }
      }
    //   goto done;
    break 'done;
   }
   CHECK!(f);

   for pass in 0 .. 8 {
      let mut pcount : i32 = 0;
      let mut class_set : i32 = 0;
      while pcount < part_read {
         if pass == 0 {
            for j in 0 .. ch {
               if *do_not_decode.offset(j as isize) == 0 {
                  let c : &Codebook = mem::transmute(f.codebooks.offset(r.classbook as isize));
                  let mut temp;
                  DECODE!(temp,f,c);
                  if temp == EOP {
                    //   goto done;
                    break 'done;
                  }
                  *(*part_classdata.offset(j as isize)).offset(class_set as isize) = *r.classdata.offset(temp as isize);
               }
            }
         }
            let mut i = 0;
            while i < classwords && pcount < part_read {
            for j in 0 .. ch {
               if *do_not_decode.offset(j as isize) == 0 {
                  let c : i32 = *(*(*part_classdata.offset(j as isize)).offset(class_set as isize)).offset(i as isize) as i32;
                  let b : i32 = (*r.residue_books.offset(c as isize))[pass as usize] as i32;
                  if b >= 0 {
                      let target = *residue_buffers.offset(j as isize);
                      let offset : i32 =  r.begin as i32 + pcount*r.part_size as i32;
                      let n : i32 = r.part_size as i32;
                      let book : &Codebook = mem::transmute(f.codebooks.offset(b as isize));
                     if residue_decode(f, book, target, offset, n, rtype) == 0{
                        // goto done;
                        break 'done;
                     }
                  }

                }
            }
            i += 1; pcount += 1;
         }
         class_set += 1;
      }
   }

    break;
    } // loop done
//   done:
   CHECK!(f);
   temp_free!(f,part_classdata);
   temp_alloc_restore!(f,temp_alloc_point);
}

// the following were split out into separate functions while optimizing;
// they could be pushed back up but eh. __forceinline showed no change;
// they're probably already being inlined.

unsafe fn imdct_step3_iter0_loop(n: i32, e: *mut f32, i_off: i32, k_off: i32 , mut A: *mut f32)
{
   let mut ee0 = e.offset(i_off as isize);
   let mut ee2 = ee0.offset(k_off as isize);
   let mut i : i32;

   assert!((n & 3) == 0);
   i = n>>2;
   while i > 0 {
      let mut k00_20: f32;
      let mut k01_21: f32;
      k00_20  = *ee0.offset(0) - *ee2.offset(0);
      k01_21  = *ee0.offset(-1) - *ee2.offset(-1);
      *ee0.offset(-0) += *ee2.offset(0);
      *ee0.offset(-1) += *ee2.offset(-1);
      *ee2.offset(-0) = k00_20 * *A.offset(0) - k01_21 * *A.offset(1);
      *ee2.offset(-1) = k01_21 * *A.offset(0) + k00_20 * *A.offset(1);
      A = A.offset(8);

      k00_20  = *ee0.offset(-2) - *ee2.offset(-2);
      k01_21  = *ee0.offset(-3) - *ee2.offset(-3);
      *ee0.offset(-2) += *ee2.offset(-2);
      *ee0.offset(-3) += *ee2.offset(-3);
      *ee2.offset(-2) = k00_20 * *A.offset(0) - k01_21 * *A.offset(1);
      *ee2.offset(-3) = k01_21 * *A.offset(0) + k00_20 * *A.offset(1);
      A = A.offset(8);

      k00_20  = *ee0.offset(-4) - *ee2.offset(-4);
      k01_21  = *ee0.offset(-5) - *ee2.offset(-5);
      *ee0.offset(-4) += *ee2.offset(-4);
      *ee0.offset(-5) += *ee2.offset(-5);
      *ee2.offset(-4) = k00_20 * *A.offset(0) - k01_21 * *A.offset(1);
      *ee2.offset(-5) = k01_21 * *A.offset(0) + k00_20 * *A.offset(1);
      A = A.offset(8);

      k00_20  = *ee0.offset(-6) - *ee2.offset(-6);
      k01_21  = *ee0.offset(-7) - *ee2.offset(-7);
      *ee0.offset(-6) += *ee2.offset(-6);
      *ee0.offset(-7) += *ee2.offset(-7);
      *ee2.offset(-6) = k00_20 * *A.offset(0) - k01_21 * *A.offset(1);
      *ee2.offset(-7) = k01_21 * *A.offset(0) + k00_20 * *A.offset(1);
      A = A.offset(8);
      ee0 = ee0.offset(-8);
      ee2 = ee2.offset(-8);

        i -= 1;
   }
}


unsafe fn imdct_step3_inner_r_loop(lim: i32, e: *mut f32, d0: i32 , k_off: i32 , mut A: *mut f32, k1: i32)
{
   let mut i : i32;
   let mut k00_20 : f32; 
   let mut k01_21 : f32;

   let mut e0 = e.offset(d0 as isize);
   let mut e2 = e0.offset(k_off as isize);

   i = lim >> 2;
   while i > 0 {
      k00_20 = *e0.offset(-0) - *e2.offset(-0);
      k01_21 = *e0.offset(-1) - *e2.offset(-1);
      *e0.offset(-0) += *e2.offset(-0);
      *e0.offset(-1) += *e2.offset(-1);
      *e2.offset(-0) = (k00_20)**A.offset(0) - (k01_21) * *A.offset(1);
      *e2.offset(-1) = (k01_21)**A.offset(0) + (k00_20) * *A.offset(1);

      A = A.offset(k1 as isize);

      k00_20 = *e0.offset(-2) - *e2.offset(-2);
      k01_21 = *e0.offset(-3) - *e2.offset(-3);
      *e0.offset(-2) += *e2.offset(-2);
      *e0.offset(-3) += *e2.offset(-3);
      *e2.offset(-2) = (k00_20)**A.offset(0) - (k01_21) * *A.offset(1);
      *e2.offset(-3) = (k01_21)**A.offset(0) + (k00_20) * *A.offset(1);

      A = A.offset(k1 as isize);

      k00_20 = *e0.offset(-4) - *e2.offset(-4);
      k01_21 = *e0.offset(-5) - *e2.offset(-5);
      *e0.offset(-4) += *e2.offset(-4);
      *e0.offset(-5) += *e2.offset(-5);
      *e2.offset(-4) = (k00_20)**A.offset(0) - (k01_21) * *A.offset(1);
      *e2.offset(-5) = (k01_21)**A.offset(0) + (k00_20) * *A.offset(1);

      A = A.offset(k1 as isize);

      k00_20 = *e0.offset(-6) - *e2.offset(-6);
      k01_21 = *e0.offset(-7) - *e2.offset(-7);
      *e0.offset(-6) += *e2.offset(-6);
      *e0.offset(-7) += *e2.offset(-7);
      *e2.offset(-6) = (k00_20)**A.offset(0) - (k01_21) * *A.offset(1);
      *e2.offset(-7) = (k01_21)**A.offset(0) + (k00_20) * *A.offset(1);

      e0 = e0.offset(-8);
      e2 = e2.offset(-8);

      A = A.offset(k1 as isize);
    
        i -= 1;
   }
}


unsafe fn imdct_step3_inner_s_loop(n: i32, e: *mut f32, i_off: i32, k_off: i32, A: *mut f32, a_off: i32 , k0: i32)
{
   let mut i : i32;
   let a_off = a_off as isize;
   
   let A0 = *A.offset(0);
   let A1 = *A.offset(0+1);
   let A2 = *A.offset(0+a_off);
   let A3 = *A.offset(0+a_off+1);
   let A4 = *A.offset(0+a_off*2+0);
   let A5 = *A.offset(0+a_off*2+1);
   let A6 = *A.offset(0+a_off*3+0);
   let A7 = *A.offset(0+a_off*3+1);

    let mut k00: f32;
    let mut k11: f32;

   let mut ee0 = e.offset(i_off as isize);
   let mut ee2 = ee0.offset(k_off as isize);

   i = n;
   while i > 0 {
      k00     = *ee0.offset(0) - *ee2.offset(0);
      k11     = *ee0.offset(-1) - *ee2.offset(-1);
      *ee0.offset(0) =  *ee0.offset(0) + *ee2.offset(0);
      *ee0.offset(-1) =  *ee0.offset(-1) + *ee2.offset(-1);
      *ee2.offset(0) = (k00) * A0 - (k11) * A1;
      *ee2.offset(-1) = (k11) * A0 + (k00) * A1;

      k00     = *ee0.offset(-2) - *ee2.offset(-2);
      k11     = *ee0.offset(-3) - *ee2.offset(-3);
      *ee0.offset(-2) =  *ee0.offset(-2) + *ee2.offset(-2);
      *ee0.offset(-3) =  *ee0.offset(-3) + *ee2.offset(-3);
      *ee2.offset(-2) = (k00) * A2 - (k11) * A3;
      *ee2.offset(-3) = (k11) * A2 + (k00) * A3;

      k00     = *ee0.offset(-4) - *ee2.offset(-4);
      k11     = *ee0.offset(-5) - *ee2.offset(-5);
      *ee0.offset(-4) =  *ee0.offset(-4) + *ee2.offset(-4);
      *ee0.offset(-5) =  *ee0.offset(-5) + *ee2.offset(-5);
      *ee2.offset(-4) = (k00) * A4 - (k11) * A5;
      *ee2.offset(-5) = (k11) * A4 + (k00) * A5;

      k00     = *ee0.offset(-6) - *ee2.offset(-6);
      k11     = *ee0.offset(-7) - *ee2.offset(-7);
      *ee0.offset(-6) =  *ee0.offset(-6) + *ee2.offset(-6);
      *ee0.offset(-7) =  *ee0.offset(-7) + *ee2.offset(-7);
      *ee2.offset(-6) = (k00) * A6 - (k11) * A7;
      *ee2.offset(-7) = (k11) * A6 + (k00) * A7;

      ee0 = ee0.offset(-k0 as isize);
      ee2 = ee2.offset(-k0 as isize);

        i -= 1;
   }
}


unsafe fn imdct_step3_inner_s_loop_ld654(n: i32, e: *mut f32, i_off: i32, A: *mut f32, base_n: i32)
{
   let a_off = base_n >> 3;
   let A2 = *A.offset( 0 + a_off as isize);
   let mut z = e.offset(i_off as isize);
   let base = z.offset(- (16 * n) as isize);

   while z > base {
      let mut k00 : f32;
      let mut k11 : f32;

      k00   = *z.offset(-0) - *z.offset(-8);
      k11   = *z.offset(-1) - *z.offset(-9);
      *z.offset(-0) = *z.offset(-0) + *z.offset(-8);
      *z.offset(-1) = *z.offset(-1) + *z.offset(-9);
      *z.offset(-8) =  k00;
      *z.offset(-9) =  k11 ;

      k00    = *z.offset(-2) - *z.offset(-10);
      k11    = *z.offset(-3) - *z.offset(-11);
      *z.offset(-2) = *z.offset(-2) + *z.offset(-10);
      *z.offset(-3) = *z.offset(-3) + *z.offset(-11);
      *z.offset(-10) = (k00+k11) * A2;
      *z.offset(-11) = (k11-k00) * A2;

      k00    = *z.offset(-12) - *z.offset(-4);  // reverse to avoid a unary negation
      k11    = *z.offset(-5) - *z.offset(-13);
      *z.offset(-4) = *z.offset(-4) + *z.offset(-12);
      *z.offset(-5) = *z.offset(-5) + *z.offset(-13);
      *z.offset(-12) = k11;
      *z.offset(-13) = k00;

      k00    = *z.offset(-14) - *z.offset(-6);  // reverse to avoid a unary negation
      k11    = *z.offset(-7) - *z.offset(-15);
      *z.offset(-6) = *z.offset(-6) + *z.offset(-14);
      *z.offset(-7) = *z.offset(-7) + *z.offset(-15);
      *z.offset(-14) = (k00+k11) * A2;
      *z.offset(-15) = (k00-k11) * A2;

      iter_54(z);
      iter_54(z.offset(-8));
      z = z.offset(-16);
   }
}

#[inline(always)]
unsafe fn iter_54(z: *mut f32)
{
//    float k00,k11,k22,k33;
//    float y0,y1,y2,y3;

   let k00  = *z.offset(0) - *z.offset(-4);
   let y0   = *z.offset(0) + *z.offset(-4);
   let y2   = *z.offset(-2) + *z.offset(-6);
   let k22  = *z.offset(-2) - *z.offset(-6);

   *z.offset(-0) = y0 + y2;      // z0 + z4 + z2 + z6
   *z.offset(-2) = y0 - y2;      // z0 + z4 - z2 - z6

   // done with y0,y2

   let k33  = *z.offset(-3) - *z.offset(-7);

   *z.offset(-4) = k00 + k33;    // z0 - z4 + z3 - z7
   *z.offset(-6) = k00 - k33;    // z0 - z4 - z3 + z7

   // done with k33

   let k11  = *z.offset(-1) - *z.offset(-5);
   let y1   = *z.offset(-1) + *z.offset(-5);
   let y3   = *z.offset(-3) + *z.offset(-7);

   *z.offset(-1) = y1 + y3;      // z1 + z5 + z3 + z7
   *z.offset(-3) = y1 - y3;      // z1 + z5 - z3 - z7
   *z.offset(-5) = k11 - k22;    // z1 - z5 + z2 - z6
   *z.offset(-7) = k11 + k22;    // z1 - z5 - z2 + z6
}



unsafe fn inverse_mdct(buffer: *mut f32, n: i32, f: &mut vorb, blocktype: i32)
{
   let n2 : i32 = n >> 1;
   let n4 : i32 = n >> 2; 
   let n8 : i32 = n >> 3;
   let mut l : i32;
   let ld: i32;
   // @OPTIMIZE: reduce register pressure by using fewer variables?
   let save_point : i32 = temp_alloc_save!(f);
   let buf2 : *mut f32 = temp_alloc!(f, n2 as usize * mem::size_of::<f32>() ) as *mut f32;
   let u: *mut f32;
   let v: *mut f32;
//    twiddle factors
   let A: *mut f32 = f.A[blocktype as usize];

   // IMDCT algorithm from "The use of multirate filter banks for coding of high quality digital audio"
   // See notes about bugs in that paper in less-optimal implementation 'inverse_mdct_old' after this function.

   // kernel from paper


   // merged:
   //   copy and reflect spectral data
   //   step 0

   // note that it turns out that the items added together during
   // this step are, in fact, being added to themselves (as reflected
   // by step 0). inexplicable inefficiency! this became obvious
   // once I combined the passes.

   // so there's a missing 'times 2' here (for adding X to itself).
   // this propogates through linearly to the end, where the numbers
   // are 1/2 too small, and need to be compensated for.

   {
       let mut d: *mut f32; let mut e: *mut f32; let mut AA: *mut f32; let e_stop: *mut f32;
      d = buf2.offset( (n2-2) as isize);
      AA = A;
      e = buffer.offset(0);
      e_stop = buffer.offset(n2 as isize);
      while e != e_stop {
         *d.offset(1) = *e.offset(0) * *AA.offset(0) - *e.offset(2) * *AA.offset(1);
         *d.offset(0) = *e.offset(0) * *AA.offset(1) + *e.offset(2) * *AA.offset(0);
         d = d.offset(-2);
         AA = AA.offset(2);
         e = e.offset(4);
      }

      e = buffer.offset( (n2-3) as isize);
      while d >= buf2 {
         *d.offset(1) = -*e.offset(2) * *AA.offset(0) - -*e.offset(0) * *AA.offset(1);
         *d.offset(0) = -*e.offset(2) * *AA.offset(1) + -*e.offset(0) * *AA.offset(0);
         d = d.offset(-2);
         AA = AA.offset(2);
         e = e.offset(-4);
      }
   }

   // now we use symbolic names for these, so that we can
   // possibly swap their meaning as we change which operations
   // are in place

   u = buffer;
   v = buf2;

   // step 2    (paper output is w, now u)
   // this could be in place, but the data ends up in the wrong
   // place... _somebody_'s got to swap it, so this is nominated
   {
      let mut AA : *mut f32 = A.offset( (n2-8) as isize);
      let mut d0 : *mut f32; let mut d1: *mut f32; let mut e0: *mut f32; let mut e1: *mut f32;

      e0 = v.offset(n4 as isize);
      e1 = v.offset(0);

      d0 = u.offset(n4 as isize);
      d1 = u.offset(0);

      while AA >= A {
         let mut v40_20 : f32; let mut v41_21: f32;

         v41_21 = *e0.offset(1) - *e1.offset(1);
         v40_20 = *e0.offset(0) - *e1.offset(0);
         *d0.offset(1)  = *e0.offset(1) + *e1.offset(1);
         *d0.offset(0)  = *e0.offset(0) + *e1.offset(0);
         *d1.offset(1)  = v41_21 * *AA.offset(4) - v40_20 * *AA.offset(5);
         *d1.offset(0)  = v40_20 * *AA.offset(4) + v41_21 * *AA.offset(5);

         v41_21 = *e0.offset(3) - *e1.offset(3);
         v40_20 = *e0.offset(2) - *e1.offset(2);
         *d0.offset(3)  = *e0.offset(3) + *e1.offset(3);
         *d0.offset(2)  = *e0.offset(2) + *e1.offset(2);
         *d1.offset(3)  = v41_21 * *AA.offset(0) - v40_20 * *AA.offset(1);
         *d1.offset(2)  = v40_20 * *AA.offset(0) + v41_21 * *AA.offset(1);

         AA = AA.offset(-8);

         d0 = d0.offset(4);
         d1 = d1.offset(4);
         e0 = e0.offset(4);
         e1 = e1.offset(4);
      }
   }

   // step 3
   ld = ilog(n) - 1; // ilog is off-by-one from normal definitions

   // optimized step 3:

   // the original step3 loop can be nested r inside s or s inside r;
   // it's written originally as s inside r, but this is dumb when r
   // iterates many times, and s few. So I have two copies of it and
   // switch between them halfway.

   // this is iteration 0 of step 3
   imdct_step3_iter0_loop(n >> 4, u, n2-1-n4*0, -(n >> 3), A);
   imdct_step3_iter0_loop(n >> 4, u, n2-1-n4*1, -(n >> 3), A);

   // this is iteration 1 of step 3
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*0, -(n >> 4), A, 16);
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*1, -(n >> 4), A, 16);
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*2, -(n >> 4), A, 16);
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*3, -(n >> 4), A, 16);

   l=2;
   while l < (ld-3)>>1 {
      let k0 : i32 = n >> (l+2);
      let k0_2 : i32 = k0>>1;
      let lim : i32 = 1 << (l+1);
      for i in 0 .. lim {
         imdct_step3_inner_r_loop(n >> (l+4), u, n2-1 - k0*i, -k0_2, A, 1 << (l+3));
      }
    l += 1;
   }

   while l < ld-6 {
      let k0 : i32 = n >> (l+2);
      let k1 = 1 << (l+3);
      let k0_2 = k0>>1;
      let rlim : i32 = n >> (l+6);
      let mut r : i32;
      let lim : i32 = 1 << (l+1);
      let mut i_off : i32;
      let mut A0 : *mut f32 = A;
      i_off = n2-1;
      r = rlim;
      while r > 0 {
         imdct_step3_inner_s_loop(lim, u, i_off, -k0_2, A0, k1, k0);
         A0 = A0.offset( (k1*4) as isize);
         i_off -= 8;
         
        r -= 1;
      }
    l += 1;
   }

   // iterations with count:
   //   ld-6,-5,-4 all interleaved together
   //       the big win comes from getting rid of needless flops
   //         due to the constants on pass 5 & 4 being all 1 and 0;
   //       combining them to be simultaneous to improve cache made little difference
   imdct_step3_inner_s_loop_ld654(n >> 5, u, n2-1, A, n);

   // output is u

   // step 4, 5, and 6
   // cannot be in-place because of step 5
   {
      let mut bitrev : *mut u16 = f.bit_reverse[blocktype as usize];
      // weirdly, I'd have thought reading sequentially and writing
      // erratically would have been better than vice-versa, but in
      // fact that's not what my testing showed. (That is, with
      // j = bitreverse(i), do you read i and write j, or read j and write i.)

      let mut d0 : *mut f32 = v.offset( (n4-4) as isize);
      let mut d1 : *mut f32 = v.offset( (n2-4) as isize);
      while d0 >= v {
         let mut k4;

         k4 = *bitrev.offset(0);
         *d1.offset(3) = *u.offset((k4+0) as isize);
         *d1.offset(2) = *u.offset((k4+1) as isize);
         *d0.offset(3) = *u.offset((k4+2) as isize);
         *d0.offset(2) = *u.offset((k4+3) as isize);

         k4 = *bitrev.offset(1);
         *d1.offset(1) = *u.offset((k4+0) as isize);
         *d1.offset(0) = *u.offset((k4+1) as isize);
         *d0.offset(1) = *u.offset((k4+2) as isize);
         *d0.offset(0) = *u.offset((k4+3) as isize);
         
         d0 = d0.offset(-4);
         d1 = d1.offset(-4);
         bitrev = bitrev.offset(2);
      }
   }
   // (paper output is u, now v)


   // data must be in buf2
   assert!(v == buf2);

   // step 7   (paper output is v, now v)
   // this is now in place
   {
      let mut C = f.C[blocktype as usize];
      let mut d : *mut f32; let mut e : *mut f32;

      d = v;
      e = v.offset( (n2 - 4) as isize );

      while d < e {
         let mut a02: f32; let mut a11: f32;let mut b0: f32;let mut b1: f32;let mut b2: f32;let mut b3: f32;

         a02 = *d.offset(0) - *e.offset(2);
         a11 = *d.offset(1) + *e.offset(3);

         b0 = *C.offset(1) * a02 + *C.offset(0)*a11;
         b1 = *C.offset(1) * a11 - *C.offset(0)*a02;

         b2 = *d.offset(0) + *e.offset( 2);
         b3 = *d.offset(1) - *e.offset( 3);

         *d.offset(0) = b2 + b0;
         *d.offset(1) = b3 + b1;
         *e.offset(2) = b2 - b0;
         *e.offset(3) = b1 - b3;

         a02 = *d.offset(2) - *e.offset(0);
         a11 = *d.offset(3) + *e.offset(1);

         b0 = *C.offset(3)*a02 + *C.offset(2)*a11;
         b1 = *C.offset(3)*a11 - *C.offset(2)*a02;

         b2 = *d.offset(2) + *e.offset( 0);
         b3 = *d.offset(3) - *e.offset( 1);

         *d.offset(2) = b2 + b0;
         *d.offset(3) = b3 + b1;
         *e.offset(0) = b2 - b0;
         *e.offset(1) = b1 - b3;

         C = C.offset(4);
         d = d.offset(4);
         e = e.offset(-4);
      }
   }

   // data must be in buf2


   // step 8+decode   (paper output is X, now buffer)
   // this generates pairs of data a la 8 and pushes them directly through
   // the decode kernel (pushing rather than pulling) to avoid having
   // to make another pass later

   // this cannot POSSIBLY be in place, so we refer to the buffers directly

   {
//       float *d0,*d1,*d2,*d3;

      let mut B = f.B[blocktype as usize].offset( (n2 - 8) as isize);
      let mut e = buf2.offset( (n2 - 8) as isize );
      let mut d0 = buffer.offset(0);
      let mut d1 = buffer.offset( (n2-4) as isize);
      let mut d2 = buffer.offset( n2 as isize);
      let mut d3 = buffer.offset( (n-4) as isize);
      while e >= v {
//          float p0,p1,p2,p3;

         let mut p3 =  *e.offset(6)* *B.offset(7) - *e.offset(7) * *B.offset(6);
         let mut p2 = -*e.offset(6)* *B.offset(6) - *e.offset(7) * *B.offset(7); 

         *d0.offset(0) =   p3;
         *d1.offset(3) = - p3;
         *d2.offset(0) =   p2;
         *d3.offset(3) =   p2;

         let mut p1 =  *e.offset(4)**B.offset(5) - *e.offset(5)**B.offset(4);
         let mut p0 = -*e.offset(4)**B.offset(4) - *e.offset(5)**B.offset(5); 

         *d0.offset(1) =   p1;
         *d1.offset(2) = - p1;
         *d2.offset(1) =   p0;
         *d3.offset(2) =   p0;

         p3 =  *e.offset(2)**B.offset(3) - *e.offset(3)**B.offset(2);
         p2 = -*e.offset(2)**B.offset(2) - *e.offset(3)**B.offset(3); 

         *d0.offset(2) =   p3;
         *d1.offset(1) = - p3;
         *d2.offset(2) =   p2;
         *d3.offset(1) =   p2;

         p1 =  *e.offset(0)**B.offset(1) - *e.offset(1)**B.offset(0);
         p0 = -*e.offset(0)**B.offset(0) - *e.offset(1)**B.offset(1); 

         *d0.offset(3) =   p1;
         *d1.offset(0) = - p1;
         *d2.offset(3) =   p0;
         *d3.offset(0) =   p0;

         B = B.offset(-8);
         e = e.offset(-8);
         d0 = d0.offset(4);
         d2 = d2.offset(4);
         d1 = d1.offset(-4);
         d3 = d3.offset(-4);
      }
   }

   temp_free!(f,buf2);
   temp_alloc_restore!(f,save_point);
}

const VORBIS_packet_id : u8 = 1;
const VORBIS_packet_comment : u8 = 3;
const VORBIS_packet_setup : u8 = 5;


pub unsafe fn start_decoder(f: &mut vorb) -> i32
{
    let mut header : [u8; 6] = mem::zeroed();
    let mut x : u8;
    let mut y : u8;
//    int len,i,j,k, max_submaps = 0;
   let mut max_submaps = 0;
   let mut longest_floorlist = 0;
    use STBVorbisError::*;

   // first page, first packet

   if start_page(f) == 0                              {return 0;} // false
   // validate page flag
   if (f.page_flag & PAGEFLAG_first_page as u8) == 0       {return error(f, VORBIS_invalid_first_page as i32)}
   if (f.page_flag & PAGEFLAG_last_page as u8) != 0           {return error(f, VORBIS_invalid_first_page as i32);}
   if (f.page_flag & PAGEFLAG_continued_packet as u8) != 0   {return error(f, VORBIS_invalid_first_page as i32);}
   // check for expected packet length
   if f.segment_count != 1                       {return error(f, VORBIS_invalid_first_page as i32);}
   if f.segments[0] != 30                        {return error(f, VORBIS_invalid_first_page as i32);}
   // read packet
   // check packet header
   if get8(f) != VORBIS_packet_id                 {return error(f, VORBIS_invalid_first_page as i32);}
   if getn(f, header.as_mut_ptr(), 6) == 0                         {return error(f, VORBIS_unexpected_eof as i32);}
   if vorbis_validate(header.as_ptr()) == 0                    {return error(f, VORBIS_invalid_first_page as i32);}
   // vorbis_version
   if get32(f) != 0                               {return error(f, VORBIS_invalid_first_page as i32);}
   f.channels = get8(f) as i32; if f.channels == 0        { return error(f, VORBIS_invalid_first_page as i32);}
   if f.channels > STB_VORBIS_MAX_CHANNELS       {return error(f, VORBIS_too_many_channels as i32);}
   f.sample_rate = get32(f); if f.sample_rate == 0  {return error(f, VORBIS_invalid_first_page as i32);}
   get32(f); // bitrate_maximum
   get32(f); // bitrate_nominal
   get32(f); // bitrate_minimum
   x  = get8(f);
   {
      let log0 : i32 = (x & 15) as i32;
      let log1 : i32 = (x >> 4) as i32;
      f.blocksize_0 = 1 << log0;
      f.blocksize_1 = 1 << log1;
      if log0 < 6 || log0 > 13                       {return error(f, VORBIS_invalid_setup as i32);}
      if log1 < 6 || log1 > 13                       {return error(f, VORBIS_invalid_setup as i32);}
      if log0 > log1                                 {return error(f, VORBIS_invalid_setup as i32);}
   }
   // framing_flag
   x = get8(f);
   if (x & 1) == 0                                    {return error(f, VORBIS_invalid_first_page as i32);}

   // second packet!
   if start_page(f) == 0                              {return 0;} // false

   if start_packet(f) == 0                            {return 0;} // false
   
   let mut len;
   while {
      len = next_segment(f);
      skip(f, len);
      f.bytes_in_seg = 0;
      len != 0
   } {/* do nothing */}

   // third packet!
   if start_packet(f) == 0                            {return 0;} // false

   if IS_PUSH_MODE!(f) {
      if is_whole_packet_present(f, 1) == 0 {
         // convert error in ogg header to write type
         if f.error == VORBIS_invalid_stream as i32 {
            f.error = VORBIS_invalid_setup as i32;
         }
         return 0; // false
      }
   }

   crc32_init(); // always init it, to avoid multithread race conditions

   if get8_packet(f) != VORBIS_packet_setup as i32       {return error(f, VORBIS_invalid_setup as i32);}
   for i in 0usize .. 6 {header[i] = get8_packet(f) as u8;}
   if vorbis_validate(header.as_ptr()) == 0                    {return error(f, VORBIS_invalid_setup as i32);}

   // codebooks

   f.codebook_count = (get_bits(f,8) + 1) as i32;
   f.codebooks = {
       let codebook_count = f.codebook_count as usize;
       setup_malloc(f, (mem::size_of::<Codebook>() * codebook_count) as i32)
   } as *mut Codebook;
    if f.codebooks.is_null()                        {return error(f, VORBIS_outofmem as i32);}
    std::ptr::write_bytes(f.codebooks, 0, f.codebook_count as usize);
    
   for i in 0 .. f.codebook_count {
      let mut values: *mut u32;
      let ordered: i32;
      let mut sorted_count: i32;
      let mut total : i32 = 0;
      let mut lengths: *mut u8;
      let mut c : &mut Codebook= mem::transmute(f.codebooks.offset(i as isize));
      CHECK!(f);
      x = get_bits(f, 8) as u8; if x != 0x42            {return error(f, VORBIS_invalid_setup as i32);}
      x = get_bits(f, 8) as u8; if x != 0x43            {return error(f, VORBIS_invalid_setup as i32);}
      x = get_bits(f, 8) as u8; if x != 0x56            {return error(f, VORBIS_invalid_setup as i32);}
      x = get_bits(f, 8) as u8;
      c.dimensions = ((get_bits(f, 8) << 8) as i32 + x as i32) as i32;
      x = get_bits(f, 8) as u8;
      y = get_bits(f, 8) as u8;
      c.entries = ((get_bits(f, 8)<<16) + ( (y as u32) <<8) + x as u32) as i32;
      ordered = get_bits(f,1) as i32;
      c.sparse = if ordered != 0 { false } else { get_bits(f,1) != 0 };

      if c.dimensions == 0 && c.entries != 0    {return error(f, VORBIS_invalid_setup as i32);}

      if c.sparse == true {
         lengths = setup_temp_malloc(f, c.entries) as *mut u8;
      }else{
         c.codeword_lengths = setup_malloc(f, c.entries) as *mut u8 ;
        lengths = c.codeword_lengths;
      }

      if lengths.is_null() {return error(f, VORBIS_outofmem as i32);}

      if ordered != 0 {
         let mut current_entry : i32 = 0;
         let mut current_length : i32 = (get_bits(f,5) + 1) as i32;
         while current_entry < c.entries {
            let limit : i32 = c.entries - current_entry;
            let n : i32 = get_bits(f, ilog(limit)) as i32;
            if current_entry + n > c.entries as i32 { return error(f, VORBIS_invalid_setup as i32); }
            std::ptr::write_bytes(lengths.offset(current_entry as isize), current_length as u8, n as usize);
//             memset(lengths + current_entry, current_length, n);
            current_entry += n;
            current_length += 1;
         }
      } else {
         for j in 0 .. c.entries {
            let present : i32 = if c.sparse == true { get_bits(f,1) } else { 1 } as i32;
            if present != 0 {
               *lengths.offset(j as isize) = ( get_bits(f, 5) + 1) as u8;
               total += 1;
               if *lengths.offset(j as isize) == 32 {
                  return error(f, VORBIS_invalid_setup as i32);
               }
            } else {
               *lengths.offset(j as isize) = NO_CODE as u8;
            }
         }
      }

      if c.sparse == true && total >= c.entries >> 2 {
         // convert sparse items to non-sparse!
         if c.entries > f.setup_temp_memory_required as i32 {
            f.setup_temp_memory_required = c.entries as u32;
         }

         c.codeword_lengths = setup_malloc(f, c.entries) as *mut u8;
         if c.codeword_lengths.is_null() {return error(f, VORBIS_outofmem as i32);}
         std::ptr::copy_nonoverlapping(lengths, c.codeword_lengths, c.entries as usize);
//          memcpy(c.codeword_lengths, lengths, c.entries);
         setup_temp_free(f, lengths as *mut c_void, c.entries); // note this is only safe if there have been no intervening temp mallocs!
         lengths = c.codeword_lengths;
         c.sparse = false;
      }

      // compute the size of the sorted tables
      if c.sparse == true {
         sorted_count = total;
      } else {
         sorted_count = 0;
         for j in 0 .. c.entries {
            if *lengths.offset(j as isize) > STB_VORBIS_FAST_HUFFMAN_LENGTH as u8 && 
                *lengths.offset(j as isize) != NO_CODE as u8 {
               sorted_count += 1;
            }
         }
      }

      c.sorted_entries = sorted_count;
      values = std::ptr::null_mut();

      CHECK!(f);
      if c.sparse == false {
         c.codewords = setup_malloc(f, mem::size_of_val(&*c.codewords.offset(0)) as i32 * c.entries) as *mut u32;
         if c.codewords.is_null()                  {return error(f, VORBIS_outofmem as i32);}
      } else {
//          unsigned int size;
         if c.sorted_entries != 0 {
            c.codeword_lengths = setup_malloc(f, c.sorted_entries) as *mut u8;
            if c.codeword_lengths.is_null()           {return error(f, VORBIS_outofmem as i32);}
            c.codewords = setup_temp_malloc(f, mem::size_of::<u32>() as i32 * c.sorted_entries) as *mut u32;
            if c.codewords.is_null()                  {return error(f, VORBIS_outofmem as i32);}
            values = setup_temp_malloc(f, mem::size_of::<u32>()  as i32* c.sorted_entries) as *mut u32 ;
            if values.is_null()                        {return error(f, VORBIS_outofmem as i32);}
         }
         let size: u32 = c.entries as u32 + (mem::size_of::<u32>() + mem::size_of::<u32>()) as u32 * c.sorted_entries as u32;
         if size > f.setup_temp_memory_required {
            f.setup_temp_memory_required = size as u32;
         }
      }

      {
          let temp_entries = c.entries; // note(bungcip): just to satisfy borrow checker
      if compute_codewords(c, lengths, temp_entries, values) == 0 {
         if c.sparse == true {setup_temp_free(f, values as *mut c_void, 0);}
         return error(f, VORBIS_invalid_setup as i32);
      }
      }

      if c.sorted_entries != 0 {
         // allocate an extra slot for sentinels
         c.sorted_codewords = setup_malloc(f, mem::size_of::<u32>() as i32 * (c.sorted_entries+1)) as *mut u32;
         if c.sorted_codewords.is_null() {return error(f, VORBIS_outofmem as i32);}
         // allocate an extra slot at the front so that c.sorted_values[-1] is defined
         // so that we can catch that case without an extra if
         c.sorted_values    = setup_malloc(f, mem::size_of::<i32>() as i32 * (c.sorted_entries+1)) as *mut i32;
         if c.sorted_values.is_null() { return error(f, VORBIS_outofmem as i32); }
         c.sorted_values = c.sorted_values.offset(1);
         *c.sorted_values.offset(-1) = -1;
         compute_sorted_huffman(c, lengths, values);
      }

      if c.sparse == true {
         setup_temp_free(f, values as *mut c_void, mem::size_of::<u32>() as i32 * c.sorted_entries);
         setup_temp_free(f, c.codewords as *mut c_void, mem::size_of::<u32>() as i32 *c.sorted_entries);
         setup_temp_free(f, lengths as *mut c_void, c.entries);
         c.codewords = std::ptr::null_mut();
      }

      compute_accelerated_huffman(c);

      CHECK!(f);
      c.lookup_type = get_bits(f, 4) as u8;
      if c.lookup_type > 2 {return error(f, VORBIS_invalid_setup as i32);}
      if c.lookup_type > 0 {
//          uint16 *mults;
         c.minimum_value = float32_unpack(get_bits(f, 32));
         c.delta_value = float32_unpack(get_bits(f, 32));
         c.value_bits = ( get_bits(f, 4)+1 ) as u8;
         c.sequence_p = ( get_bits(f,1) ) as u8;
         if c.lookup_type == 1 {
            c.lookup_values = lookup1_values(c.entries, c.dimensions) as u32;
         } else {
            c.lookup_values = c.entries as u32 * c.dimensions as u32;
         }
         if c.lookup_values == 0 {return error(f, VORBIS_invalid_setup as i32);}
         let mults : *mut u16 = setup_temp_malloc(f, mem::size_of::<u16>() as i32 * c.lookup_values as i32) as *mut u16;
         if mults.is_null() {return error(f, VORBIS_outofmem as i32);}
         for j in 0 .. c.lookup_values {
            let q : i32 = get_bits(f, c.value_bits as i32) as i32;
            if q as u32 == EOP as u32 { 
                setup_temp_free(f,mults as *mut c_void,mem::size_of::<u16>() as i32 * c.lookup_values as i32); 
                return error(f, VORBIS_invalid_setup as i32); 
            }
            *mults.offset(j as isize) = q as u16;
         }
         
         'skip: loop {
         if c.lookup_type == 1 {
//             int len, sparse = c.sparse;
            let sparse : i32 = c.sparse as i32;
            let mut last : f32 = 0.0;
            // pre-expand the lookup1-style multiplicands, to avoid a divide in the inner loop
            if sparse != 0 {
               if c.sorted_entries == 0 { 
                //    goto skip; FIXME: buat loop
                break 'skip;
                }
               c.multiplicands = setup_malloc(f, mem::size_of::<codetype>() as i32 * c.sorted_entries * c.dimensions) as *mut codetype;
            } else{
               c.multiplicands = setup_malloc(f, mem::size_of::<codetype>() as i32 * c.entries        * c.dimensions) as *mut codetype;
            }
            if c.multiplicands.is_null() { 
                setup_temp_free(f, mults as *mut c_void, mem::size_of::<u16>() as i32 * c.lookup_values as i32); 
                return error(f, VORBIS_outofmem as i32);
            }
            len = if sparse != 0 { c.sorted_entries } else {c.entries};
            for j in 0 .. len {
               let z : u32 = if sparse != 0 { *c.sorted_values.offset(j as isize) } else {j} as u32;
               let mut div: u32 = 1;
               for k in 0 .. c.dimensions {
                  let off: i32 = (z / div) as i32 % c.lookup_values as i32;
                //   let mut val: f32 = *mults.offset(off as isize) as f32; // NOTE(bungcip) : maybe bugs?
                  let val = *mults.offset(off as isize) as f32 * c.delta_value + c.minimum_value + last;
                  *c.multiplicands.offset( (j*c.dimensions + k) as isize) = val;
                  if c.sequence_p !=0 {
                     last = val;
                  }
                  if k+1 < c.dimensions {
                      use std::u32;
                     if div > u32::MAX / c.lookup_values as u32 {
                        setup_temp_free(f, mults as *mut c_void, mem::size_of::<u16>() as i32 * c.lookup_values as i32);
                        return error(f, VORBIS_invalid_setup as i32);
                     }
                     div *= c.lookup_values;
                  }
               }
            }
            c.lookup_type = 2;
         }
         else
         {
            let mut last : f32 = 0.0;
            CHECK!(f);
            c.multiplicands = setup_malloc(f, mem::size_of::<codetype>() as i32 * c.lookup_values as i32) as *mut codetype;
            if c.multiplicands.is_null() {
                 setup_temp_free(f, mults as *mut c_void, mem::size_of::<codetype>() as i32 * c.lookup_values as i32); 
                 return error(f, VORBIS_outofmem as i32);
            }
            for j in 0 .. c.lookup_values {
               let val : f32 = *mults.offset(j as isize) as f32 * c.delta_value + c.minimum_value + last;
               *c.multiplicands.offset(j as isize) = val;
               if c.sequence_p != 0{
                  last = val;
               }
            }
         }
         
         break;
         } // loop 'skip
//         skip:;
         setup_temp_free(f, mults as *mut c_void, mem::size_of::<codetype>() as i32 * c.lookup_values as i32);

         CHECK!(f);
      }
      CHECK!(f);
   }

   // time domain transfers (notused)

   x = ( get_bits(f, 6) + 1) as u8;
   for i in 0 .. x {
      let z : u32 = get_bits(f, 16);
      if z != 0 { return error(f, VORBIS_invalid_setup as i32); }
   }

   // Floors
   f.floor_count = (get_bits(f, 6)+1) as i32;
   {
       // safity borrow checker
       let fc = f.floor_count * mem::size_of::<Floor>() as i32;
       f.floor_config = setup_malloc(f, fc) as *mut Floor;
   }
   if f.floor_config.is_null() { return error(f, VORBIS_outofmem as i32);}
   for i in 0 .. f.floor_count {
      f.floor_types[i as usize] = get_bits(f, 16) as u16;
      if f.floor_types[i as usize] > 1 {return error(f, VORBIS_invalid_setup as i32);}
      if f.floor_types[i as usize] == 0 {
      // NOTE(bungcip): using transmute because rust don't have support for union yet.. 
      //                transmute floor0 to floor1
          let g: &mut Floor0 = mem::transmute(&mut (*f.floor_config.offset(i as isize)).floor1);
         g.order = get_bits(f,8) as u8;
         g.rate = get_bits(f,16) as u16;
         g.bark_map_size = get_bits(f,16) as u16;
         g.amplitude_bits = get_bits(f,6) as u8;
         g.amplitude_offset = get_bits(f,8) as u8;
         g.number_of_books = (get_bits(f,4) + 1) as u8;
         for j in 0 .. g.number_of_books{
            g.book_list[j as usize] = get_bits(f,8) as u8;
         }
         return error(f, VORBIS_feature_not_supported as i32);
      } else {
         let mut p : [Point; 31*8+2] = mem::zeroed();
         let mut g : &mut Floor1 = &mut (*f.floor_config.offset(i as isize)).floor1;
         let mut max_class : i32 = -1; 
         g.partitions = get_bits(f, 5) as u8;
         for j in 0 .. g.partitions {
            g.partition_class_list[j as usize] = get_bits(f, 4) as u8;
            if g.partition_class_list[j as usize] as i32 > max_class {
               max_class = g.partition_class_list[j as usize] as i32;
            }
         }
         for j in 0 .. max_class + 1 {
            g.class_dimensions[j as usize] = get_bits(f, 3) as u8 + 1;
            g.class_subclasses[j as usize] = get_bits(f, 2) as u8;
            if g.class_subclasses[j as usize] != 0 {
               g.class_masterbooks[j as usize] = get_bits(f, 8) as u8;
               if g.class_masterbooks[j as usize] >= f.codebook_count as u8 {
                   return error(f, VORBIS_invalid_setup as i32);
               }
            }
            for k in 0 ..  (1 << g.class_subclasses[j as usize]) {
               g.subclass_books[j as usize][k as usize] = get_bits(f,8) as i16 -1;
               if g.subclass_books[j as usize][k as usize] >= f.codebook_count as i16 {
                   return error(f, VORBIS_invalid_setup as i32);
               }
            }
         }
         g.floor1_multiplier = (get_bits(f,2) +1) as u8;
         g.rangebits = get_bits(f,4) as u8;
         g.Xlist[0] = 0;
         g.Xlist[1] = 1 << g.rangebits;
         g.values = 2;
         for j in 0 .. g.partitions {
            let c : i32 = g.partition_class_list[j as usize] as i32;
            for k in 0 .. g.class_dimensions[c as usize] {
               g.Xlist[g.values as usize] = get_bits(f, g.rangebits as i32) as u16;
               g.values += 1;
            }
         }
         // precompute the sorting
         for j in 0 .. g.values {
            p[j as usize].x = g.Xlist[j as usize];
            p[j as usize].y = j as u16;
         }
         qsort(p.as_mut_ptr() as *mut c_void, g.values as usize, mem::size_of::<Point>(), point_compare as *const c_void);
         for j in 0 .. g.values {
            g.sorted_order[j as usize] = p[j as usize].y as u8;
         }
         // precompute the neighbors
         for j in 2 .. g.values {
            let mut low : i32 = 0;
            let mut hi: i32 = 0;
            neighbors(g.Xlist.as_mut_ptr(), j, &mut low, &mut hi);
            g.neighbors[j as usize][0] = low as u8;
            g.neighbors[j as usize][1] = hi as u8;
         }

         if g.values > longest_floorlist{
            longest_floorlist = g.values;
         }
      }
   }

   // Residue
   f.residue_count = get_bits(f, 6) as i32 + 1;
   {
       // to satifying borrow checker
        let residue_size = f.residue_count * mem::size_of::<Residue>() as i32;
        f.residue_config = setup_malloc(f, residue_size) as *mut Residue;
        if f.residue_config.is_null() {return error(f, VORBIS_outofmem as i32);}
        std::ptr::write_bytes(f.residue_config, 0, f.residue_count as usize);
   }
   for i in 0 .. f.residue_count {
       let mut residue_cascade: [u8; 64] = mem::zeroed();
      let mut r : &mut Residue = mem::transmute(f.residue_config.offset(i as isize));
      f.residue_types[i as usize] = get_bits(f, 16) as u16;
      if f.residue_types[i as usize] > 2 {return error(f, VORBIS_invalid_setup as i32);}
      r.begin = get_bits(f, 24);
      r.end = get_bits(f, 24);
      if r.end < r.begin {return error(f, VORBIS_invalid_setup as i32);}
      r.part_size = get_bits(f,24)+1;
      r.classifications = get_bits(f,6) as u8 + 1;
      r.classbook = get_bits(f,8) as u8;
      if r.classbook as i32 >= f.codebook_count {return error(f, VORBIS_invalid_setup as i32);}
      for j in 0 .. r.classifications {
         let mut high_bits: u8 = 0;
         let low_bits: u8 = get_bits(f,3) as u8;
         if get_bits(f,1) != 0 {
            high_bits = get_bits(f,5) as u8;
         }
         residue_cascade[j as usize] = high_bits*8 + low_bits;
      }
      r.residue_books = setup_malloc(f, mem::size_of::<[i16; 8]>() as i32 * r.classifications as i32) as *mut [i16; 8];
      if r.residue_books.is_null() {return error(f, VORBIS_outofmem as i32);}
      for j in 0 .. r.classifications {
         for k in 0 .. 8 {
            if (residue_cascade[j as usize] & (1 << k)) != 0 {
               (*r.residue_books.offset(j as isize))[k as usize] = get_bits(f, 8) as i16;
               if (*r.residue_books.offset(j as isize))[k as usize] as i32 >= f.codebook_count {
                   return error(f, VORBIS_invalid_setup as i32);
                }
            } else {
               (*r.residue_books.offset(j as isize))[k as usize] = -1;
            }
         }
      }
      // precompute the classifications[] array to avoid inner-loop mod/divide
      // call it 'classdata' since we already have r.classifications
      {
          // satify borrow checker
          let classdata_size =  mem::size_of::<*mut u8>() as i32 * 
            (*f.codebooks.offset(r.classbook as isize) ).entries;
          r.classdata = setup_malloc(f, classdata_size) as *mut *mut u8;
          if r.classdata.is_null() {return error(f, VORBIS_outofmem as i32);}
          std::ptr::write_bytes(r.classdata, 0, (*f.codebooks.offset(r.classbook as isize) ).entries as usize);
      }
      for j in 0 .. (*f.codebooks.offset(r.classbook as isize) ).entries {
         let classwords = (*f.codebooks.offset(r.classbook as isize) ).dimensions;
         let mut temp = j;
         
         *r.classdata.offset(j as isize) = setup_malloc(f, mem::size_of::<u8>() as i32 * classwords) as *mut u8;
         if (*r.classdata.offset(j as isize)).is_null() {return error(f, VORBIS_outofmem as i32);}
         
         let mut k = classwords-1;
         while k >= 0 {
            *(*r.classdata.offset(j as isize)).offset(k as isize) = (temp % r.classifications as i32) as u8;
            temp /= r.classifications as i32;
            k -= 1;
         }
      }
   }

   f.mapping_count = get_bits(f,6) as i32 +1;
   {
       // satify borrow checker
       let mapping_size = f.mapping_count * mem::size_of::<Mapping>() as i32;
       f.mapping = setup_malloc(f, mapping_size) as *mut Mapping;
       if f.mapping.is_null() {return error(f, VORBIS_outofmem as i32);}
       std::ptr::write_bytes(f.mapping, 0, f.mapping_count as usize);
   }
   for i in 0 .. f.mapping_count {
      let m : &mut Mapping = mem::transmute(f.mapping.offset(i as isize));      
      let mapping_type : i32 = get_bits(f,16) as i32;
      if mapping_type != 0 {return error(f, VORBIS_invalid_setup as i32);}
      {
        // satify borrow checker   
        let mapping_channel_size = f.channels * mem::size_of::<MappingChannel>() as i32;   
          m.chan = setup_malloc(f, mapping_channel_size) as *mut MappingChannel;
          if m.chan.is_null() {return error(f, VORBIS_outofmem as i32);}
      }
      if get_bits(f,1) != 0 {
         m.submaps = get_bits(f,4) as u8 + 1;
      }
      else{
         m.submaps = 1;
      }
      if m.submaps > max_submaps{
         max_submaps = m.submaps;
      }
      if get_bits(f,1) != 0 {
         m.coupling_steps = get_bits(f,8) as u16 + 1;
         for k in 0 .. m.coupling_steps {
             // satify borrow checker
             let ilog_result = ilog(f.channels-1);
            (*m.chan.offset(k as isize)).magnitude = get_bits(f, ilog_result) as u8;
             let ilog_result = ilog(f.channels-1);
            (*m.chan.offset(k as isize)).angle = get_bits(f, ilog_result) as u8;
            if (*m.chan.offset(k as isize)).magnitude as i32 >= f.channels        {return error(f, VORBIS_invalid_setup as i32);}
            if (*m.chan.offset(k as isize)).angle     as i32 >= f.channels        {return error(f, VORBIS_invalid_setup as i32);}
            if (*m.chan.offset(k as isize)).magnitude == (*m.chan.offset(k as isize)).angle   {return error(f, VORBIS_invalid_setup as i32);}
         }
      } else{
         m.coupling_steps = 0;
      }

      // reserved field
      if get_bits(f,2) != 0 {return error(f, VORBIS_invalid_setup as i32);}
      if m.submaps > 1 {
         for j in 0 .. f.channels {
            (*m.chan.offset(j as isize)).mux = get_bits(f, 4) as u8;
            if (*m.chan.offset(j as isize)).mux >= m.submaps                {return error(f, VORBIS_invalid_setup as i32);}
         }
      } else{
         // @SPECIFICATION: this case is missing from the spec
         for j in 0 .. f.channels {
            (*m.chan.offset(j as isize)).mux = 0;
         }
      }

      for j in 0 .. m.submaps {
         get_bits(f,8); // discard
         m.submap_floor[j as usize] = get_bits(f,8) as u8;
         m.submap_residue[j as usize] = get_bits(f,8) as u8;
         if m.submap_floor[j as usize] as i32 >= f.floor_count      {return error(f, VORBIS_invalid_setup as i32);}
         if m.submap_residue[j as usize] as i32 >= f.residue_count  {return error(f, VORBIS_invalid_setup as i32);}
      }
   }

   // Modes
   f.mode_count = get_bits(f, 6) as i32 + 1;
   for i in 0 .. f.mode_count {
      let m: &mut Mode = {
          // satify borrow checker
          let p : &mut Mode = &mut f.mode_config[i as usize];
          let p : *mut Mode = p as *mut _;
          let p : &mut Mode = mem::transmute(p);
          p
      };
      m.blockflag = get_bits(f,1) as u8;
      m.windowtype = get_bits(f,16) as u16;
      m.transformtype = get_bits(f,16) as u16;
      m.mapping = get_bits(f,8) as u8;
      if m.windowtype != 0                 {return error(f, VORBIS_invalid_setup as i32);}
      if m.transformtype != 0              {return error(f, VORBIS_invalid_setup as i32);}
      if m.mapping as i32 >= f.mapping_count     {return error(f, VORBIS_invalid_setup as i32);}
   }

   flush_packet(f);

   f.previous_length = 0;

   for i in 0 .. f.channels {
       let block_size_1 = f.blocksize_1;
      f.channel_buffers[i as usize] = setup_malloc(f, mem::size_of::<f32>() as i32 * block_size_1) as *mut f32;
      f.previous_window[i as usize] = setup_malloc(f, mem::size_of::<f32>() as i32 * block_size_1/2) as *mut f32;
      f.finalY[i as usize]          = setup_malloc(f, mem::size_of::<i16>() as i32 * longest_floorlist) as *mut i16;
      if f.channel_buffers[i as usize].is_null() || f.previous_window[i as usize].is_null() || f.finalY[i as usize].is_null() {
          return error(f, VORBIS_outofmem as i32);
      }
   }

   {  
       let blocksize_0 = f.blocksize_0;
       let blocksize_1 = f.blocksize_1;
        if init_blocksize(f, 0, blocksize_0) == 0 {return 0;} // false
        if init_blocksize(f, 1, blocksize_1) == 0 {return 0;} // false
        f.blocksize[0] = blocksize_0;
        f.blocksize[1] = blocksize_1;
   }

   // compute how much temporary memory is needed

   // 1.
   {
      let imdct_mem : u32 = f.blocksize_1 as u32 * mem::size_of::<f32>() as u32 >> 1;
      let classify_mem : u32;
      let mut max_part_read = 0;
      for i in 0 .. f.residue_count {
         let r : &Residue = mem::transmute(f.residue_config.offset(i as isize));
         let n_read = r.end - r.begin;
         let part_read = n_read / r.part_size;
         if part_read > max_part_read{
            max_part_read = part_read;
         }
      }
      classify_mem = f.channels as u32 * (mem::size_of::<*mut c_void>() as u32 + max_part_read * mem::size_of::<*mut u8>() as u32);

      f.temp_memory_required = classify_mem;
      if imdct_mem > f.temp_memory_required{
         f.temp_memory_required = imdct_mem;
      }
   }

   f.first_decode = 1; // true

   if f.alloc.alloc_buffer.is_null() == false {
      assert!(f.temp_offset == f.alloc.alloc_buffer_length_in_bytes);
      // check if there's enough temp memory so we don't error later
      if f.setup_offset as u32 + mem::size_of::<stb_vorbis>() as u32 + f.temp_memory_required as u32 > f.temp_offset as u32{
         return error(f, VORBIS_outofmem as i32);
      }
   }

   f.first_audio_page_offset = stb_vorbis_get_file_offset(f);
   return 1; // true
}



// Below is function that still live in C code
extern {
    fn qsort(base: *mut c_void, nmemb: size_t, size: size_t, compar: *const c_void);
}