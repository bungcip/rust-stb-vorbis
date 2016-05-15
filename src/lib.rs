// temporary disable lint for now...
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
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

impl Clone for Codebook {
    fn clone(&self) -> Self {
        *self
    }
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
//    floor0: Floor0,
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
   scan: [CRCscan; STB_VORBIS_PUSHDATA_CRC_COUNT as usize],

  // sample-access
   channel_buffer_start: c_int,
   channel_buffer_end: c_int,
}

pub type vorb = stb_vorbis;

pub struct stb_vorbis_info
{
   sample_rate: c_uint,
   channels: c_int,

   setup_memory_required: c_uint,
   setup_temp_memory_required: c_uint,
   temp_memory_required: c_uint,

   max_frame_size: c_int,
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
unsafe fn add_entry(c: &Codebook, huff_code: u32, symbol: c_int, count: c_int, len: c_int, values: *mut u32)
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


#[no_mangle]
pub unsafe extern fn compute_codewords(c: &mut Codebook, len: *mut u8, n: c_int, values: *mut u32) -> c_int
{
   let mut m=0;
   let mut available: [u32; 32] = std::mem::zeroed();

//    memset(available, 0, sizeof(available));
   // find the first entry
   let mut k = 0;
   while k < n {
       if (*len.offset(k as isize) as c_int) < NO_CODE {
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
      if z as c_int == NO_CODE {
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

fn get_window(f: &vorb, len: c_int) -> *mut f32
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

// from CRC
const M_PI : f32 = 3.14159265358979323846264;

// called twice per file
fn compute_twiddle_factors(n: c_int, A: *mut f32, B: *mut f32, C: *mut f32)
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

// NOTE(bungcip): p must be zeroed before using it
#[no_mangle]
pub unsafe extern fn vorbis_init(p: &mut stb_vorbis, z: *const stb_vorbis_alloc)
{
   
   if z.is_null() == false {
      p.alloc = *z;
      p.alloc.alloc_buffer_length_in_bytes = (p.alloc.alloc_buffer_length_in_bytes+3) & !3;
      p.temp_offset = p.alloc.alloc_buffer_length_in_bytes;
   }
   p.eof = 0;
   p.error = STBVorbisError::VORBIS__no_error as c_int;
   p.stream = std::ptr::null_mut();
   p.codebooks = std::ptr::null_mut();
   p.page_crc_tests = -1;

   p.close_on_free = 0;
   p.f = std::ptr::null_mut();
}

#[no_mangle]
pub unsafe extern fn stb_vorbis_close(p: *mut stb_vorbis)
{
   if p.is_null(){
       return;
   }
   
   vorbis_deinit(std::mem::transmute(p));
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

#[no_mangle]
pub unsafe extern fn vorbis_finish_frame(f: &mut stb_vorbis, len: c_int, left: c_int, right: c_int) -> c_int
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

#[no_mangle]
pub unsafe extern fn stb_vorbis_get_frame_short_interleaved(f: &mut stb_vorbis, num_c: c_int, buffer: *mut i16, num_shorts: i32) -> c_int
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


#[no_mangle]
pub unsafe extern fn stb_vorbis_decode_filename(filename: *const i8, channels: *mut c_int, sample_rate: *mut c_int, output: *mut *mut i16) -> c_int
{
//    int data_len, offset, total, limit, error;
//    short *data;
   let mut error : c_int = 0;
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


#[no_mangle]
pub unsafe extern fn stb_vorbis_get_frame_float(f: &mut stb_vorbis, channels: *mut c_int, output: *mut *mut *mut f32) -> c_int
{
//    int len, right,left,i;
   if IS_PUSH_MODE!(f){
       return error(f, STBVorbisError::VORBIS_invalid_api_mixing as c_int);
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

#[no_mangle]
pub unsafe extern fn stb_vorbis_get_frame_short(f: &mut vorb, num_c: c_int, buffer: *mut *mut i16, num_samples: c_int) -> c_int
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


const PLAYBACK_MONO  : c_int =   1;
const PLAYBACK_LEFT  : c_int =   2;
const PLAYBACK_RIGHT : c_int =   4;


#[no_mangle]
pub unsafe extern fn convert_samples_short(buf_c: c_int, buffer: *mut *mut i16, b_offset: c_int, data_c: c_int, data: *mut *mut f32, d_offset: c_int, samples: c_int)
{
   if buf_c != data_c && buf_c <= 2 && data_c <= 6 {
    //   static int channel_selector[3][2] = { {0}, {PLAYBACK_MONO}, {PLAYBACK_LEFT, PLAYBACK_RIGHT} };
      
      static channel_selector: [[c_int; 2]; 3] = [
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



#[no_mangle]
pub unsafe extern fn convert_channels_short_interleaved(buf_c: c_int, buffer: *mut i16, data_c: c_int, data: *mut *mut f32, d_offset: c_int, len: c_int)
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
               if ( (v + 32768) as c_uint) > 65535 {
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

unsafe fn copy_samples(dest: *mut i16, src: *mut f32, len: c_int)
{
   for i in 0 .. len  {
      let mut v : i32 = FAST_SCALED_FLOAT_TO_INT!(*src.offset(i as isize), 15);
      if ((v + 32768) as c_uint) > 65535 {
         v = if v < 0 { -32768 } else { 32767 };
      }
      *dest.offset(i as isize) = v as i16;
   }
}

#[no_mangle]
pub unsafe extern fn stb_vorbis_seek(f: &mut stb_vorbis, sample_number: c_uint) -> c_int
{
   if stb_vorbis_seek_frame(f, sample_number) == 0 {
      return 0;
   }

   if sample_number != f.current_loc {
      let mut n = 0;
      let frame_start = f.current_loc;
      stb_vorbis_get_frame_float(f, &mut n, std::ptr::null_mut());
      assert!(sample_number > frame_start);
      assert!(f.channel_buffer_start + (sample_number-frame_start) as c_int <= f.channel_buffer_end);
      f.channel_buffer_start += (sample_number - frame_start) as i32;
   }

   return 1;
}


#[no_mangle]
pub unsafe extern fn init_blocksize(f: &mut vorb, b: c_int, n: c_int) -> c_int
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
     return error(f, VORBIS_outofmem as c_int);  
   } 
   
   compute_twiddle_factors(n, f.A[b], f.B[b], f.C[b]);
   f.window[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n2) as *mut f32;
   
   if f.window[b].is_null() {return error(f, VORBIS_outofmem as c_int);}
   compute_window(n, f.window[b]);

   f.bit_reverse[b] = setup_malloc(f, std::mem::size_of::<f32>() as i32 * n8) as *mut u16;
   if f.bit_reverse[b].is_null() { return error(f, VORBIS_outofmem as c_int); }
   
   compute_bitreverse(n, f.bit_reverse[b]);
   return 1; // true
}

// accelerated huffman table allows fast O(1) match of all symbols
// of length <= STB_VORBIS_FAST_HUFFMAN_LENGTH
#[no_mangle]
pub unsafe extern fn compute_accelerated_huffman(c: &mut Codebook)
{
//    int i, len;
   for i in 0 .. FAST_HUFFMAN_TABLE_SIZE {
       c.fast_huffman[i as usize] = -1;
   }

   let mut len = if c.sparse != 0 { c.sorted_entries } else  {c.entries};
   
   if len > 32767 {len = 32767;} // largest possible value we can encode!
   
   for i in 0 .. len {
      if *c.codeword_lengths.offset(i as isize) <= STB_VORBIS_FAST_HUFFMAN_LENGTH as u8 {
         let mut z : u32 = if c.sparse != 0 { 
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

#[no_mangle]
pub unsafe extern fn stb_vorbis_get_file_offset(f: &stb_vorbis) -> c_uint
{
   if f.push_mode != 0 {return 0;}
   if USE_MEMORY!(f) {return (f.stream as usize - f.stream_start as usize) as c_uint;}
   return (libc::ftell(f.f) - f.f_start as i32) as c_uint;
}

#[no_mangle]
pub unsafe extern fn start_page_no_capturepattern(f: &mut vorb) -> c_int
{
    use STBVorbisError::*;
    
   // stream structure version
   if 0 != get8(f) {return error(f, VORBIS_invalid_stream_structure_version as c_int);}
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
      return error(f, VORBIS_unexpected_eof as c_int);
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

#[no_mangle]
pub unsafe extern fn predict_point(x: c_int, x0: c_int , x1: c_int , y0: c_int , y1: c_int ) -> c_int
{
   let dy = y1 - y0;
   let adx = x1 - x0;
   // @OPTIMIZE: force int division to round in the right direction... is this necessary on x86?
   let err = libc::abs(dy) * (x - x0);
   let off = err / adx;
   return if dy < 0  {y0 - off} else {y0 + off};
}

pub type YTYPE = i16;

#[no_mangle]
pub unsafe extern fn do_floor(f: &mut vorb, map: &mut Mapping, i: c_int, n: c_int , target: *mut f32, finalY: *mut YTYPE, _: *mut u8) -> c_int
{
   let n2 = n >> 1;

   let s : &MappingChannel = std::mem::transmute(map.chan.offset(i as isize));
   let s : i32 = s.mux as i32;
   let floor = map.submap_floor[s as usize];
   
   if f.floor_types[floor as usize] == 0 {
      return error(f, STBVorbisError::VORBIS_invalid_stream as c_int);
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
unsafe fn draw_line(output: *mut f32, x0: c_int, y0: c_int, mut x1: c_int, y1: c_int, n: c_int)
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


#[no_mangle]
pub unsafe extern fn residue_decode(f: &mut vorb, book: &Codebook, target: *mut f32, mut offset: c_int, n: c_int, rtype: c_int) -> c_int
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


unsafe fn codebook_decode(f: &mut vorb, c: &Codebook, output: *mut f32, mut len: c_int ) -> c_int
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


unsafe fn codebook_decode_start(f: &mut vorb, c: &Codebook) -> c_int
{
   let mut z = -1;

   // type 0 is only legal in a scalar context
   if c.lookup_type == 0 {
      error(f, STBVorbisError::VORBIS_invalid_stream as c_int);
   } else {
      DECODE_VQ!(z,f,c);
      if c.sparse != 0 {assert!(z < c.sorted_entries);}
      if z < 0 {  // check for EOP
         if f.bytes_in_seg == 0 {
            if f.last_seg != 0 {
               return z;
            }
         }
         error(f,  STBVorbisError::VORBIS_invalid_stream as c_int);
      }
   }
   return z;
}

#[inline(always)]
unsafe fn codebook_decode_scalar(f: &mut vorb, c: &Codebook) -> c_int
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

#[no_mangle]
pub unsafe extern fn codebook_decode_scalar_raw(f: &mut vorb, c: &Codebook) -> c_int
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
      if c.sparse == 0 {
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
   assert!(c.sparse == 0);
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

   error(f, STBVorbisError::VORBIS_invalid_stream as c_int);
   f.valid_bits = 0;
   return -1;
}

unsafe fn codebook_decode_step(f: &mut vorb, c: &Codebook, output: *mut f32, mut len: c_int , step: c_int ) -> c_int
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

#[no_mangle]
pub unsafe extern fn codebook_decode_deinterleave_repeat(f: &mut vorb, c: &Codebook, outputs: *mut *mut f32, ch: c_int, c_inter_p: *mut c_int, p_inter_p: *mut c_int, len: c_int, mut total_decode: c_int) -> c_int
{
   let mut c_inter = *c_inter_p;
   let mut p_inter = *p_inter_p;
   let mut effective = c.dimensions;
   let mut z : i32;

   // type 0 is only legal in a scalar context
   if c.lookup_type == 0 {
     return error(f, STBVorbisError::VORBIS_invalid_stream as c_int);
   } 
   while total_decode > 0 {
      let mut last : f32 = CODEBOOK_ELEMENT_BASE!(c);
      DECODE_VQ!(z,f,c);
      assert!(c.sparse == 0 || z < c.sorted_entries);
      if z < 0 {
         if f.bytes_in_seg == 0{
            if f.last_seg != 0{
              return 0; // false
            } 
         }
         return error(f, STBVorbisError::VORBIS_invalid_stream as c_int);
      }

      // if this will take us off the end of the buffers, stop short!
      // we check by computing the length of the virtual interleaved
      // buffer (len*ch), our current offset within it (p_inter*ch)+(c_inter),
      // and the length we'll be using (effective)
      if c_inter + p_inter*ch + effective > len * ch {
         effective = len*ch - (p_inter*ch - c_inter);
      }

      {
         z *= c.dimensions;
         if c.sequence_p != 0 {
            for i in 0 .. effective {
               let val : f32 = CODEBOOK_ELEMENT_FAST!(c,z+i) + last;
               if (*outputs.offset(c_inter as isize)).is_null() == false {
                  *(*outputs.offset(c_inter as isize)).offset(p_inter as isize) += val;
               }
               c_inter += 1;
               if c_inter == ch { c_inter = 0; p_inter += 1; }
               last = val;
            }
         } else {
            for i in 0 .. effective {
               let val : f32 = CODEBOOK_ELEMENT_FAST!(c,z+i) + last;
               if (*outputs.offset(c_inter as isize)).is_null() == false {
                  *(*outputs.offset(c_inter as isize)).offset(p_inter as isize) += val;
               }
               c_inter += 1;
               if c_inter == ch { c_inter = 0; p_inter += 1; }
            }
         }
      }

      total_decode -= effective;
   }
   *c_inter_p = c_inter;
   *p_inter_p = p_inter;

   return 1; // true
}

unsafe fn compute_window(n: c_int, window: *mut f32)
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

#[no_mangle]
pub unsafe extern fn vorbis_alloc(f: &mut stb_vorbis) -> *mut stb_vorbis
{
   let p : *mut stb_vorbis = setup_malloc(f, std::mem::size_of::<stb_vorbis>() as i32)  as *mut stb_vorbis;
   return p;
}

#[no_mangle]
pub unsafe fn vorbis_deinit(p: &mut stb_vorbis)
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
   
   if p.close_on_free != 0 {
       libc::fclose(p.f);
   }
}

unsafe fn compute_samples(mask: c_int, output: *mut i16, num_c: c_int, data: *mut *mut f32, d_offset: c_int, len: c_int)
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
         if (v + 32768) as c_uint > 65535{
            v = if v < 0 { -32768 } else { 32767 };
         }
         *output.offset( (o+i as i32) as isize) = v as i16;
      }
       
       o += BUFFER_SIZE as i32;
   }
}

unsafe fn compute_stereo_samples(output: *mut i16, num_c: c_int, data: *mut *mut f32, d_offset: c_int, len: c_int)
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
         if (v + 32768) as c_uint > 65535 {
            v = if v < 0 {-32768} else {32767};
         }
         *output.offset((o2+i) as isize) = v as i16;
      }
       
       o += (BUFFER_SIZE >> 1) as i32;
   }

}

pub unsafe fn stb_vorbis_seek_frame(f: &mut stb_vorbis, sample_number: c_uint) -> c_int
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   
   let max_frame_samples: u32;

   if IS_PUSH_MODE!(f) { return error(f, STBVorbisError::VORBIS_invalid_api_mixing as c_int);}

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
         return error(f, STBVorbisError::VORBIS_seek_failed as c_int);
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
pub fn stb_vorbis_get_error(f: &mut stb_vorbis) -> c_int
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   let e = f.error;
   f.error = STBVorbisError::VORBIS__no_error as c_int;
   return e;
}

// this function is equivalent to stb_vorbis_seek(f,0)
#[no_mangle]
pub unsafe extern fn stb_vorbis_seek_start(f: &mut stb_vorbis)
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   if IS_PUSH_MODE!(f) { error(f, STBVorbisError::VORBIS_invalid_api_mixing as c_int); return; }
   
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
pub fn stb_vorbis_get_sample_offset(f: &mut stb_vorbis) -> c_int
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   if f.current_loc_valid != 0 {
      return f.current_loc as c_int;
   } else{
      return -1;
   }
}

// get general information about the file
pub fn stb_vorbis_get_info(f: &mut stb_vorbis) -> stb_vorbis_info
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
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
#[no_mangle]
pub extern fn stb_vorbis_flush_pushdata(f: &mut stb_vorbis)
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

// Below is function that still live in C code
extern {
    static mut crc_table: [u32; 256];
 
    pub fn vorbis_decode_packet_rest(f: *mut vorb, len: *mut c_int, m: *mut Mode, left_start: c_int, left_end: c_int, right_start: c_int, right_end: c_int, p_left: *mut c_int) -> c_int;

    pub fn start_decoder(f: *mut vorb) -> c_int;
    pub fn seek_to_sample_coarse(f: &mut stb_vorbis, sample_number: u32) -> c_int;
    pub fn peek_decode_initial(f: &mut vorb, p_left_start: &mut c_int, p_left_end: &mut c_int, p_right_start: &mut c_int, p_right_end: &mut c_int, mode: &mut c_int) -> c_int;
    pub fn set_file_offset(f: &mut stb_vorbis, loc: c_uint) -> c_int;
    
    // Real API
    pub fn stb_vorbis_stream_length_in_samples(f: &mut stb_vorbis) -> c_uint;

}