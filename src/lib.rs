#![feature(float_extras)]

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

use std::mem;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::fs::File;
use std::path::Path;


mod helper;
pub use helper::*;

// STB_VORBIS_MAX_CHANNELS [number]
//     globally define this to the maximum number of channels you need.
//     The spec does not put a restriction on channels except that
//     the count is stored in a byte, so 255 is the hard limit.
//     Reducing this saves about 16 bytes per value, so using 16 saves
//     (255-16)*16 or around 4KB. Plus anything other memory usage
//     I forgot to account for. Can probably go as low as 8 (7.1 audio),
//     6 (5.1 audio), or 2 (stereo only).
const STB_VORBIS_MAX_CHANNELS : i32 = 16;  // enough for anyone?

// STB_PUSHDATA_CRC_COUNT [number]
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
const STB_PUSHDATA_CRC_COUNT : i32 = 4;

// STB_FAST_HUFFMAN_LENGTH [number]
//     sets the log size of the huffman-acceleration table.  Maximum
//     supported value is 24. with larger numbers, more decodings are O(1),
//     but the table size is larger so worse cache missing, so you'll have
//     to probe (and try multiple ogg vorbis files) to find the sweet spot.
const STB_FAST_HUFFMAN_LENGTH : i32 = 10;

const PACKET_ID : u8 = 1;
// const packet_comment : u8 = 3;
const PACKET_SETUP : u8 = 5;

const PAGEFLAG_CONTINUED_PACKET : u8 =   1;
const PAGEFLAG_FIRST_PAGE       : u8 =   2;
const PAGEFLAG_LAST_PAGE        : u8 =   4;




static mut CRC_TABLE: [u32; 256] = [0; 256];


// the following table is block-copied from the specification
static INVERSE_DB_TABLE: [f32; 256] =
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

const PLAYBACK_MONO  : i8 =   1;
const PLAYBACK_LEFT  : i8=   2;
const PLAYBACK_RIGHT : i8 =   4;

const CP_L : i8 = (PLAYBACK_LEFT  | PLAYBACK_MONO);
const CP_C : i8 = (PLAYBACK_LEFT  | PLAYBACK_RIGHT | PLAYBACK_MONO);
const CP_R : i8 = (PLAYBACK_RIGHT | PLAYBACK_MONO);

static CHANNEL_POSITION: [[i8; 6]; 7] = [
   [ 0, 0, 0, 0, 0, 0 ],
   [ CP_C, CP_C, CP_C, CP_C, CP_C, CP_C ],
   [ CP_L, CP_R, CP_R, CP_R, CP_R, CP_R ],
   [ CP_L, CP_C, CP_R, CP_R, CP_R, CP_R ],
   [ CP_L, CP_R, CP_L, CP_R, CP_R, CP_R ],
   [ CP_L, CP_C, CP_R, CP_L, CP_R, CP_R ],
   [ CP_L, CP_C, CP_R, CP_L, CP_R, CP_C ],
];

static OGG_PAGE_HEADER: [u8; 4] = [ 0x4f, 0x67, 0x67, 0x53 ];


/// macro to force rust to borrow 
macro_rules! FORCE_BORROW_MUT {
    ($e: expr) => {{
        // satify borrow checker
        let p = $e;
        let p = p as *mut _;
        mem::transmute(p)
    }}
}

macro_rules! FORCE_BORROW {
    ($e: expr) => {{
        // satify borrow checker
        let p = $e;
        let p = p as *const _;
        mem::transmute(p)
    }}
}

macro_rules! FAST_SCALED_FLOAT_TO_INT {
    ($x: expr, $s: expr) => {{
        let temp = $x + (1.5f32 * (1 << (23-$s)) as f32 + 0.5f32/(1 << $s) as f32);
        let temp : i32 = $crate::std::mem::transmute(temp);
        temp - (((150-$s) << 23) + (1 << 22) )        
    }}
}

fn convert_to_i16(value: f32) -> i16 {
    let mut v : i32 = unsafe { FAST_SCALED_FLOAT_TO_INT!(value, 15) };
    if (v + 32768) as u32 > 65535{
        v = if v < 0 { -32768 } else { 32767 };
    }

    v as i16
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


pub type CodeType = f32;
pub type YTYPE = i16;


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

const FAST_HUFFMAN_TABLE_SIZE : i32 =   (1 << STB_FAST_HUFFMAN_LENGTH);
const FAST_HUFFMAN_TABLE_MASK : i32 =   (FAST_HUFFMAN_TABLE_SIZE - 1);

// code length assigned to a value with no huffman encoding
const NO_CODE : u8 =   255;

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
   multiplicands: Vec<CodeType>,
   codewords: Vec<u32>,
   fast_huffman: [i16; FAST_HUFFMAN_TABLE_SIZE as usize],
   sorted_codewords: Vec<u32>,
   sorted_values: Vec<i32>,
   sorted_entries: i32,
} 

// NOTE(bungcip): maybe can be deleted?
impl Clone for Codebook {
    fn clone(&self) -> Self {
        Codebook {
            dimensions: self.dimensions, entries: self.entries,
            codeword_lengths: self.codeword_lengths.clone(),
            minimum_value: self.minimum_value,
            delta_value: self.delta_value,
            value_bits: self.value_bits,
            lookup_type: self.lookup_type,
            sequence_p: self.sequence_p,
            sparse: self.sparse,
            lookup_values: self.lookup_values,
            multiplicands: self.multiplicands.clone(),
            codewords: self.codewords.clone(),
            fast_huffman: self.fast_huffman,
            sorted_codewords: self.sorted_codewords.clone(),
            sorted_values: self.sorted_values.clone(),
            sorted_entries: self.sorted_entries
        }
    }
}

impl Default for Codebook {
    fn default() -> Self {
        let mut instance : Codebook = unsafe { mem::zeroed() };
        instance.multiplicands = Vec::new();
        instance.codewords = Vec::new();
        instance.codeword_lengths = Vec::new();
        instance.sorted_codewords = Vec::new();
        instance.sorted_values = Vec::new();
        instance
    } 
}

#[repr(C)]
#[derive(Copy, Clone)]
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
   xlist: [u16; 31*8+2], // varies
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

// union Floor, still need repr(C) because we transmute it to either Floor0/Floor1
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Floor
{
//    floor0: Floor0,
   floor1: Floor1,
}

impl Default for Floor {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[derive(Clone)]
pub struct Residue
{
   begin: u32, end: u32,
   part_size: u32,
   classifications: u8,
   classbook: u8,
   classdata: Vec<Vec<u8>>,
   residue_books: Vec<[i16; 8]>,
} 

impl Default for Residue {
    fn default() -> Self {
        let mut instance : Residue = unsafe { mem::zeroed() };
        instance.residue_books = Vec::new();
        instance
    }
}


#[derive(Copy, Clone, Default)]
pub struct MappingChannel
{
   magnitude: u8,
   angle: u8,
   mux: u8,
}


#[derive(Clone)]
pub struct Mapping
{
   coupling_steps: u16,
   chan: Vec<MappingChannel>,
   submaps: u8,
   submap_floor: [u8; 15], // varies
   submap_residue: [u8; 15], // varies
}

impl Default for Mapping {
    fn default() -> Self {
        Mapping { 
            coupling_steps: 0, 
            submaps: 0, submap_floor: [0; 15], submap_residue: [0; 15],
            chan: Vec::new()
        }
    }
}


#[derive(Copy, Clone, Default)]
pub struct Mode
{
   blockflag: u8,
   mapping: u8,
   windowtype: u16,
   transformtype: u16,
}

#[derive(Copy, Clone, Default)]
pub struct CRCscan
{
   goal_crc: u32,    // expected crc if match
   bytes_left: i32,  // bytes left in packet
   crc_so_far: u32,  // running crc
   bytes_done: i32,  // bytes processed in _current_ chunk
   sample_loc: u32,  // granule pos encoded in page
} 

#[derive(Copy, Clone, Default)]
pub struct ProbedPage
{
   page_start: u32, page_end: u32,
   last_decoded_sample: u32
}
 

pub struct Vorbis
{
  // user-accessible info
   pub sample_rate: u32,
   pub channels: i32,

  // input config
   f: Option<BufReader<File>>,
   f_start: u32,

   stream: *const u8,
   stream_start: *const u8,
   stream_end: *const u8,
   stream_len: u32,

   push_mode: bool,

   first_audio_page_offset: u32,

   p_first: ProbedPage, p_last: ProbedPage,


  // run-time results
   pub eof: bool,
   pub error: VorbisError,

  // user-useful data

  // header info
   blocksize: [usize; 2],
   blocksize_0: usize, blocksize_1: usize,
   codebook_count: i32,
   codebooks: Vec<Codebook>,
   floor_count: i32,
   floor_types: [u16; 64], // varies
   floor_config: Vec<Floor>,
   residue_count: i32,
   residue_types: [u16; 64], // varies
   residue_config: Vec<Residue>,
   mapping_count: i32,
   mapping: Vec<Mapping>,
   mode_count: i32,
   mode_config: [Mode; 64],  // varies

   total_samples: u32,

  // decode buffer
   channel_buffers: Vec<Vec<f32>>,
   outputs        : AudioBufferSlice<f32>,

   previous_window: Vec<Vec<f32>>,
   previous_length: i32,

   final_y: Vec<Vec<i16>>,

   current_loc: u32, // sample location of next frame to decode
   current_loc_valid: bool,

  // per-blocksize precomputed data
   
   // twiddle factors
   a: [Vec<f32>; 2], b: [Vec<f32>; 2], c: [Vec<f32>; 2],
   window: [Vec<f32>; 2],
   bit_reverse: [Vec<u16>; 2],

  // current page/packet/segment streaming info
//    serial: u32, // stream serial number for verification. NOTE(bungcip): not used?
   last_page: i32,
   segment_count: i32,
   segments: [u8; 255],
   page_flag: u8,
   bytes_in_seg: u8,
   first_decode: bool,
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
   scan: [CRCscan; STB_PUSHDATA_CRC_COUNT as usize],

  // sample-access
   channel_buffer_start: i32,
   channel_buffer_end: i32,
}

impl Vorbis {
    pub fn new() -> Self {
        Vorbis {
            eof: false,
            error: VorbisError::NoError,
            stream: std::ptr::null_mut(),
            codebooks: Vec::new(),
            page_crc_tests: -1,
            f: None,
            
            // zero
            sample_rate: 0,
            channels: 0,
            f_start: 0,
            stream_start: std::ptr::null_mut(),
            stream_end: std::ptr::null_mut(),
            stream_len: 0,
            push_mode: false,
            first_audio_page_offset: 0,
            p_first: ProbedPage::default(), p_last: ProbedPage::default(),
            blocksize: [0; 2],
            blocksize_0: 0, blocksize_1: 0,
            codebook_count: 0,
            floor_count: 0,
            floor_types: [0; 64], // varies
            floor_config: Vec::new(),
            residue_count: 0,
            residue_types: [0; 64], // varies
            residue_config: Vec::new(),
            mapping_count: 0,
            mapping: Vec::new(),
            mode_count: 0,
            mode_config: [Mode::default(); 64],  // varies
            total_samples: 0,
            channel_buffers: Vec::with_capacity(STB_VORBIS_MAX_CHANNELS as usize),
            outputs        : AudioBufferSlice::new(STB_VORBIS_MAX_CHANNELS as usize),
            previous_window: Vec::with_capacity(STB_VORBIS_MAX_CHANNELS as usize),
            previous_length: 0,
            final_y: Vec::with_capacity(STB_VORBIS_MAX_CHANNELS as usize),
            current_loc: 0, // sample location of next frame to decode
            current_loc_valid: false,
            a: [Vec::new(), Vec::new()], b: [Vec::new(), Vec::new()], c: [Vec::new(), Vec::new()],
            window: [Vec::new(), Vec::new()],
            bit_reverse:  [Vec::new(), Vec::new()],
            last_page: 0,
            segment_count: 0,
            segments: [0; 255],
            page_flag: 0,
            bytes_in_seg: 0,
            first_decode: false,
            next_seg: 0,
            last_seg: false,  // flag that we're on the last segment
            last_seg_which: 0, // what was the segment number of the last seg?
            acc: 0,
            valid_bits: 0,
            packet_bytes: 0,
            end_seg_with_known_loc: 0,
            known_loc_for_packet: 0,
            discard_samples_deferred: 0,
            samples_output: 0,
            scan: [CRCscan::default(); STB_PUSHDATA_CRC_COUNT as usize],
            channel_buffer_start: 0,
            channel_buffer_end: 0,
        }
    }
}


#[derive(Copy, Clone)]
pub struct VorbisInfo
{
   pub sample_rate: u32,
   pub channels: i32,

   pub max_frame_size: usize,
}

////////   ERROR CODES

#[repr(i32)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum VorbisError
{
   NoError,

   NeedMoreData=1,             // not a real error

   InvalidApiMixing,           // can't mix API modes
   OutOfMem,                     // not enough memory
   FeatureNotSupported,        // uses floor 0
   TooManyChannels,            // STB_VORBIS_MAX_CHANNELS is too small
   FileOpenFailure,            // fopen() failed
   SeekWithoutLength,          // can't seek in unknown-length file

   UnexpectedEof=10,            // file is truncated?
   SeekInvalid,                 // seek past EOF

   // decoding errors (corrupt/invalid stream) -- you probably
   // don't care about the exact details of these

   // vorbis errors:
   InvalidSetup=20,
   InvalidStream,

   // ogg errors:
   MissingCapturePattern=30,
   InvalidStreamStructureVersion,
   ContinuedPacketFlagInvalid,
   IncorrectStreamSerialNumber,
   InvalidFirstPage,
   BadPacketType,
   CantFindLastPage,
   SeekFailed
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

fn error(f: &mut Vorbis, e: VorbisError) -> bool
{
    // NOTE: e is VorbisError
    f.error = e;
    if f.eof == false && e != VorbisError::NeedMoreData {
        f.error = e; // breakpoint for debugging
    }
    
    return false;
}

fn include_in_sort(c: &Codebook, len: u8) -> bool
{
   if c.sparse == true { 
       assert!(len != NO_CODE); 
       return true;
    }
   if len == NO_CODE {
       return false;
   }
   if len > STB_FAST_HUFFMAN_LENGTH as u8 {
       return true;
   }
   return false;
}

const  CRC32_POLY  : u32 =  0x04c11db7;   // from spec

unsafe fn crc32_init()
{
   for i in 0 .. 256 {
       let mut s : u32 = i << 24;
       for _ in 0 .. 8 {
           s = (s << 1) ^ (if s >= (1u32<<31) {CRC32_POLY} else {0});
       }
       CRC_TABLE[i as usize] = s;
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
fn add_entry(c: &mut Codebook, huff_code: u32, symbol: usize, count: i32, len: u8, values: &mut [u32])
{
   if c.sparse == false {
      c.codewords[symbol] = huff_code;
   } else {
      let count = count as usize;
      c.codewords[count] = huff_code;
      c.codeword_lengths[count] = len;
      values[count] = symbol as u32;
   }
}



fn compute_codewords(c: &mut Codebook, len: &mut [u8], values: &mut [u32]) -> bool
{
   let n = len.len();
    
   // find the first entry
   let mut k = 0;
   while k < n {
       if len[k] < NO_CODE {
           break;
       }
       k += 1;
   }
   
   if k == n { 
       assert!(c.sorted_entries == 0); 
       return true;
   }
   
   // add to the list
   let mut m = 0;
   add_entry(c, 0, k, m, len[k], values);
   m += 1;
   
   // add all available leaves
   let mut available: [u32; 32] = [0; 32];
   let mut i = 1;
   while i <= len[k] {
      available[i as usize] = 1u32 << (32-i);
      i += 1;
   }
   
   // note that the above code treats the first case specially,
   // but it's really the same as the following code, so they
   // could probably be combined (except the initial code is 0,
   // and I use 0 in available[] to mean 'empty')
   for i in k + 1 .. n {
      let res : u32;
      let mut z = len[i as usize];
      if z == NO_CODE {
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
      if z == 0 { 
          return false;
      }
      res = available[z as usize];
      assert!(z < 32); // NOTE(z is u8 so negative is impossible)
      available[z as usize] = 0;
      add_entry(c, bit_reverse(res), i, m, len[i as usize], values);
      m += 1;
      
      // propogate availability up the tree
      if z != len[i as usize] {
        //  assert!(len[i as usize] >= 0 && len[i as usize] < 32);
         assert!(len[i as usize] < 32); // NOTE (len[x] is already unsigned)
         
         let mut y = len[i as usize];
         while y > z {
            assert!(available[y as usize] == 0);
            available[y as usize] = res + (1 << (32-y));
             
             y -= 1;
         }         
      }
   }
   
   return true;
}


// this is a weird definition of log2() for which log2(1) = 1, log2(2) = 2, log2(4) = 3
// as required by the specification. fast(?) implementation from stb.h
// @OPTIMIZE: called multiple times per-packet with "constants"; move to setup

fn ilog(n: i32) -> i32
{
    static LOG2_4: [i8; 16] = [0,1,2,2,3,3,3,3,4,4,4,4,4,4,4,4];

    let n = n as usize;

    // 2 compares if n < 16, 3 compares otherwise (4 if signed or n > 1<<29)
    let result = if n < (1 << 14) {
        if n < (1 << 4) {
            LOG2_4[n]
        } else if n < (1 << 9) {
            5 + LOG2_4[n >> 5]
        } else {
            10 + LOG2_4[n >> 10]
        }
    }
    else if n < (1 << 24) {
        if n < (1 << 19) {
            15 + LOG2_4[n >> 15]
        }
        else {
            20 + LOG2_4[n >> 20]
        }
    }
    else if n < (1 << 29) {
        25 + LOG2_4[n >> 25]
    }
    else if n < (1 << 31) {
        30 + LOG2_4[n >> 30]
    }
    else {
        0 // signed n returns 0
    };
    
    result as i32
       
}

fn get_window(f: &Vorbis, len: usize) -> &[f32]
{
   let len = len << 1;
   if len == f.blocksize_0 as usize { return &f.window[0]; }
   if len == f.blocksize_1 as usize { return &f.window[1]; }

   unreachable!();
}

fn compute_bitreverse(n: i32, rev: &mut [u16])
{
   let ld = ilog(n) - 1; // ilog is off-by-one from normal definitions
   let n8 = n >> 3;
   
   for i in 0 .. n8 {
       rev[i as usize] = ((bit_reverse(i as u32) >> (32-ld+3)) << 2) as u16;
   }
}


// only run while parsing the header (3 times)
fn vorbis_validate(data: &[u8]) -> bool
{
    static VORBIS_HEADER: &'static [u8; 6] = b"vorbis";
    return data == VORBIS_HEADER;
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
fn compute_twiddle_factors(n: i32, a: &mut [f32], b: &mut [f32], c: &mut [f32])
{
    use std::f32;
    
    let n4 = n >> 2;
    let n8 = n >> 3;

    let mut k = 0;
    let mut k2 = 0;
    
    while k < n4 {
        let x1 = (4*k) as f32 * M_PI / n as f32;
        a[k2 as usize]     = f32::cos(x1) as f32;
        a[(k2+1) as usize] =  -f32::sin(x1) as f32;
        
        let x2 = (k2+1) as f32 * M_PI / n as f32 / 2.0;
        b[k2 as usize]    = (f32::cos(x2) * 0.5) as f32;
        b[(k2+1) as usize] = (f32::sin(x2) * 0.5) as f32;
        
        k += 1; k2 += 2;
    }

    let mut k = 0;
    let mut k2 = 0;
    
    while k < n8 {
        let x1 = (2*(k2+1)) as f32 * M_PI / n as f32;
        c[k2] = f32::cos(x1) as f32;
        
        let x2 = (2*(k2+1)) as f32 * M_PI / n as f32;
        c[k2+1] = -f32::sin(x2) as f32;
        
        k += 1; k2 += 2;
    }

}


fn neighbors(x: &[u16], n: usize, plow: &mut i32, phigh: &mut i32)
{
    let mut low : i32 = -1;
    let mut high : i32 = 65536;
    
    for i in 0 .. n {
        if (x[i] as i32) > low && (x[i] as i32) < (x[n] as i32) { 
            *plow = i as i32;
            low = x[i] as i32; 
        }
        if (x[i] as i32) < high && (x[i] as i32) > (x[n] as i32) { 
            *phigh = i as i32; 
            high = x[i] as i32;
        }
    }
}


fn get8(z: &mut Vorbis) -> u8
{
   if !z.stream.is_null() {
      if z.stream >= z.stream_end { 
          z.eof = true;
          return 0;
      }
      
      unsafe {
        let c = *z.stream;
        z.stream = z.stream.offset(1);
        return c;
      }
   }

   let mut buf = [0; 1];
   let mut f = z.f.as_mut().unwrap();
   match f.read(&mut buf){
       Ok(n) if n == 1 => return buf[0],
       _ => {
           z.eof = true;
           return 0;
       }
   }
}


fn get32(f: &mut Vorbis) -> u32
{
   let mut x : u32 = get8(f) as u32;
   x += (get8(f) as u32) << 8;
   x += (get8(f) as u32) << 16;
   x += (get8(f) as u32) << 24;
   return x;
}

/// get from stream/file and copy to data
fn getn(z: &mut Vorbis, data: &mut [u8]) -> bool
{
   if !z.stream.is_null() {
      unsafe {
        let n = data.len();
        if z.stream.offset(n as isize) > z.stream_end { 
            z.eof = true; 
            return false;
        }
        std::ptr::copy_nonoverlapping(z.stream, data.as_mut_ptr(), n);
        z.stream = z.stream.offset(n as isize);
        return true;
      }
   }

   let mut f = z.f.as_mut().unwrap();
   match f.read_exact(data) {
       Ok(_) => return true,
       Err(_) => {
           z.eof = true;
           return false;
       }
   }
}


fn skip(z: &mut Vorbis, n: i32)
{
   if !z.stream.is_null() {
      unsafe {
        z.stream = z.stream.offset(n as isize);
        if z.stream >= z.stream_end {z.eof = true;}
        return;
      }
   }

   // file must not None
   let mut f = z.f.as_mut().unwrap();
   f.seek(SeekFrom::Current(n as i64)).unwrap();
}

fn capture_pattern(f: &mut Vorbis) -> bool
{
   if 0x4f != get8(f) {return false;}
   if 0x67 != get8(f) {return false;}
   if 0x67 != get8(f) {return false;}
   if 0x53 != get8(f) {return false;}
   return true;
}


const EOP : i32 = -1;
const INVALID_BITS : i32 = -1;

fn get8_packet_raw(f: &mut Vorbis) -> i32
{
    if f.bytes_in_seg == 0 {
        if f.last_seg == true {
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


fn get8_packet(f: &mut Vorbis) -> i32
{
    let x = get8_packet_raw(f);
    f.valid_bits = 0;
    return x;
}


fn flush_packet(f: &mut Vorbis)
{
    while get8_packet_raw(f) != EOP {}
}


// @OPTIMIZE: this is the secondary bit decoder, so it's probably not as important
// as the huffman decoder?

fn get_bits(f: &mut Vorbis, n: i32) -> u32
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


fn start_page(f: &mut Vorbis) -> bool
{
   if capture_pattern(f) == false {
       return error(f, VorbisError::MissingCapturePattern);
   } 
   return start_page_no_capturepattern(f);
}


fn start_packet(f: &mut Vorbis) -> bool
{
   while f.next_seg == -1 {
      if start_page(f) == false { return false; }
      if (f.page_flag & PAGEFLAG_CONTINUED_PACKET) != 0 {
         return error(f, VorbisError::ContinuedPacketFlagInvalid);
      }
   }
   f.last_seg = false;
   f.valid_bits = 0;
   f.packet_bytes = 0;
   f.bytes_in_seg = 0;
   // f.next_seg is now valid
   return true;
}

fn maybe_start_packet(f: &mut Vorbis) -> bool
{
    use VorbisError::{MissingCapturePattern, ContinuedPacketFlagInvalid};
    
   if f.next_seg == -1 {
      let x = get8(f) as i32;
      if f.eof == true { return false; } // EOF at page boundary is not an error!
      if 0x4f != x       { return error(f, MissingCapturePattern); }
      if 0x67 != get8(f) { return error(f, MissingCapturePattern); }
      if 0x67 != get8(f) { return error(f, MissingCapturePattern); }
      if 0x53 != get8(f) { return error(f, MissingCapturePattern); }
      if start_page_no_capturepattern(f) == false { return false; }
      if (f.page_flag & PAGEFLAG_CONTINUED_PACKET) != 0 {
         // set up enough state that we can read this packet if we want,
         // e.g. during recovery
         f.last_seg = false;
         f.bytes_in_seg = 0;
         return error(f, ContinuedPacketFlagInvalid);
      }
   }
   return start_packet(f);
}


fn next_segment(f: &mut Vorbis) -> i32
{
   if f.last_seg == true {return 0;}
   if f.next_seg == -1 {
      f.last_seg_which = f.segment_count-1; // in case start_page fails
      if start_page(f) == false { f.last_seg = true; return 0; }
      if (f.page_flag & PAGEFLAG_CONTINUED_PACKET) == 0 {
          error(f, VorbisError::ContinuedPacketFlagInvalid); 
          return 0;
      }
   }
   
   let len = f.segments[f.next_seg as usize];
   f.next_seg += 1;
   
   if len < 255 {
      f.last_seg = true;
      f.last_seg_which = f.next_seg-1;
   }
   if f.next_seg >= f.segment_count{
      f.next_seg = -1;
   }
   assert!(f.bytes_in_seg == 0);
   f.bytes_in_seg = len;
   return len as i32;
}

fn vorbis_decode_packet(f: &mut Vorbis, len: &mut i32, p_left: &mut i32, p_right: &mut i32) -> bool
{
    let mut mode_index = 0;
    let mut left_end = 0;
    let mut right_end = 0;
    
    if vorbis_decode_initial(f, p_left, &mut left_end, p_right, &mut right_end, &mut mode_index) ==  false {
        return false;
    }
    
    unsafe {
        let mode : &Mode = FORCE_BORROW!( &f.mode_config[mode_index as usize] );
        return vorbis_decode_packet_rest(
            f, len, mode, 
            *p_left, left_end, *p_right, right_end, p_left
        );
    }
}


fn vorbis_pump_first_frame(f: &mut Vorbis)
{
    let mut len = 0;
    let mut right = 0;
    let mut left = 0;
    
    if vorbis_decode_packet(f, &mut len, &mut left, &mut right) == true {
        vorbis_finish_frame(f, len, left, right);
    }
}



// create an ogg vorbis decoder from an open FILE *, looking for a stream at
// the _current_ seek point (ftell); the stream will be of length 'len' bytes.
// on failure, returns NULL and sets *error. note that stb_vorbis must "own"
// this stream; if you seek it in between calls to stb_vorbis, it will become
// confused.
pub fn stb_vorbis_open_file_section(mut file: File, length: u64) -> Result<Vorbis, VorbisError>
{
   let mut p = Vorbis::new();
   p.f_start = file.seek(SeekFrom::Current(0)).unwrap() as u32; // NOTE(bungcip): change it to i64/u64?
   p.f = Some(BufReader::new(file));
   p.stream_len   = length as u32;
    
   unsafe {
    if start_decoder(&mut p) == true {
        vorbis_pump_first_frame(&mut p);
        return Ok(p);
    }
   }

   return Err(p.error);
}

// create an ogg vorbis decoder from an open file handle, looking for a stream at
// the _current_ seek point. on failure, returns NULL and sets *error.
// note that stb_vorbis must "own" this stream; if you seek it in between
// calls to stb_vorbis, it will become confused. Morever, if you attempt to
// perform stb_vorbis_seek_*() operations on this file, it will assume it
// owns the _entire_ rest of the file after the start point. Use the next
// function, stb_vorbis_open_file_section(), to limit it.
pub fn stb_vorbis_open_file(mut file: File) -> Result<Vorbis, VorbisError>
{
    let start = file.seek(SeekFrom::Current(0)).unwrap();
    let end = file.seek(SeekFrom::End(0)).unwrap();
    let len = end - start;
    
    // seek to start position
    file.seek(SeekFrom::Start(start)).unwrap();
    
    return stb_vorbis_open_file_section(file, len);
}


// create an ogg vorbis decoder from a filename. on failure,
// returns Result
pub fn stb_vorbis_open_filename(filename: &Path)-> Result<Vorbis, VorbisError>
{    
    let file = match File::open(filename){
        Err(_)   => return Err(VorbisError::FileOpenFailure),
        Ok(file) => file
    };
    
    return stb_vorbis_open_file(file);
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

fn vorbis_decode_initial(f: &mut Vorbis, 
    p_left_start: &mut i32, p_left_end: &mut i32, 
    p_right_start: &mut i32, p_right_end: &mut i32, 
    mode: &mut i32) -> bool
{
   f.channel_buffer_start = 0;
   f.channel_buffer_end = 0;

   loop {
        if f.eof == true {return false;} 
        if maybe_start_packet(f) == false {
            return false; 
        }
        // check packet type
        if get_bits(f,1) != 0 {
            if f.push_mode {
                return error(f, VorbisError::BadPacketType);
            }
            while EOP != get8_packet(f){}
            continue;
        }
        
       break;
   }

   let x = ilog(f.mode_count-1);
   let i : i32 = get_bits(f, x) as i32;
   if i == EOP {return false;}
   if i >= f.mode_count {return false;}
   
   *mode = i;

   // NOTE: hack to forget borrow
   let &mut m : &mut Mode = unsafe { FORCE_BORROW_MUT!(&mut f.mode_config[i as usize])};
   
   let n;
   let prev;
   let next;
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
      *p_left_start = (n - f.blocksize_0) as i32 >> 2;
      *p_left_end   = (n + f.blocksize_0) as i32 >> 2;
   } else {
      *p_left_start = 0;
      *p_left_end   = window_center as i32;
   }
   if m.blockflag != 0 && next == 0 {
      *p_right_start = (n*3 - f.blocksize_0) as i32 >> 2;
      *p_right_end   = (n*3 + f.blocksize_0) as i32 >> 2;
   } else {
      *p_right_start = window_center as i32;
      *p_right_end   = n as i32;
   }

   return true;
}

fn vorbis_finish_frame(f: &mut Vorbis, len: i32, left: i32, right: i32) -> i32
{
   // we use right&left (the start of the right- and left-window sin()-regions)
   // to determine how much to return, rather than inferring from the rules
   // (same result, clearer code); 'left' indicates where our sin() window
   // starts, therefore where the previous window's right edge starts, and
   // therefore where to start mixing from the previous buffer. 'right'
   // indicates where our sin() ending-window starts, therefore that's where
   // we start saving, and where our returned-data ends.

   // mixin from previous window
   if f.previous_length != 0 {
      let n = f.previous_length as usize;
      // NOTE(bungcip): need to force borrow because mut f is borrowed....
      let w : &[f32] = unsafe { FORCE_BORROW!( get_window(f, n)) };
      let left = left as usize;
      for i in 0 .. f.channels as usize {
         for j in 0 .. n {
            f.channel_buffers[i][left + j] =
               f.channel_buffers[i][left + j] * w[j] +
               f.previous_window[i][       j] * w[n - 1 - j];
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
   for i in 0 .. f.channels as usize {
      let mut j = 0;
      while right + j < len {
         f.previous_window[i][j as usize] = f.channel_buffers[i][ (right+j) as usize];
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
pub fn stb_vorbis_get_frame_short_interleaved(f: &mut Vorbis, 
    channel_count: u32, buffer: &mut [i16]) -> i32
{
   if channel_count == 1 {
       let mut buffer = unsafe {
           AudioBufferSlice::from_single_channel(buffer)
       };
       return stb_vorbis_get_frame_short(f, channel_count as i32, &mut buffer);
   }
   
   let mut output: AudioBufferSlice<f32> = AudioBufferSlice::new(f.channels as usize);
   let num_shorts = buffer.len();
   let mut len = stb_vorbis_get_frame_float(f, None, Some(&mut output)) as usize;
   
   if len != 0 {
      if len * channel_count as usize > num_shorts {
        len = num_shorts / channel_count as usize;  
      } 
      convert_channels_short_interleaved(channel_count, buffer, &output, len);
   }
   return len as i32;
}

// decode an entire file and output the data interleaved into 
// buffer. The return value is the number of samples
// decoded, or -1 if the file could not be opened or was not an ogg vorbis file.
// When you're done with it, just free() the pointer returned in *output.
pub fn stb_vorbis_decode_filename(filename: &Path, 
    channels: &mut i32, sample_rate: &mut u32, output: &mut Vec<i16>) -> i32
{
   let mut v = match stb_vorbis_open_filename(filename){
        Err(_) => return -1,
        Ok(v)  => v
   };
   
   *channels = v.channels;
   *sample_rate = v.sample_rate;
   
   let mut offset = 0;
   let mut data_len = 0;
   let limit = v.channels as usize * 4096;
   let mut total = limit;
   
   output.resize(total as usize, 0);
   
   loop {
       let ch = v.channels as u32;
       let n = {
           let mut output_slice = output.as_mut_slice();
           stb_vorbis_get_frame_short_interleaved(
               &mut v, ch, 
               &mut output_slice[ offset .. total ]
            )
       };

      if n == 0{
        break;  
      }
         
      data_len += n;
      offset += n as usize * v.channels as usize;
      
      if offset + limit > total {
         total *= 2;
         output.resize(total as usize, 0);
      }
   }

   // resize to fit data_len
   output.resize( (data_len * v.channels) as usize , 0);
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
pub fn stb_vorbis_get_frame_float(f: &mut Vorbis, 
    channel_count: Option<&mut i32>, output: Option<&mut AudioBufferSlice<f32>>) -> i32
{
   if f.push_mode{
       error(f, VorbisError::InvalidApiMixing);
       return 0;
   } 

    let mut len = 0;
    let mut left = 0;
    let mut right = 0;
    
   if vorbis_decode_packet(f, &mut len, &mut left, &mut right) == false {
      f.channel_buffer_start = 0;
      f.channel_buffer_end = 0;
      return 0;
   }

   let len = vorbis_finish_frame(f, len, left, right);
   for i in 0 .. f.channels as usize {
    unsafe {
        f.outputs.set(i, &mut f.channel_buffers[i][left as usize ..]);
    }
   }

   f.channel_buffer_start = left;
   f.channel_buffer_end   = left+len;

   if let Some(channel_count) = channel_count {
       *channel_count = f.channels;
   }

   if let Some(output) = output {
       *output = f.outputs;
   }

   return len;
}

pub fn stb_vorbis_get_frame_short(f: &mut Vorbis, num_c: i32, mut sample_buffer: &mut AudioBufferSlice<i16>) -> i32
{
  // NOTE(bungcip): change return value to u32 or usize?

   let mut output: AudioBufferSlice<f32> = AudioBufferSlice::new(f.channels as usize);
   let len = stb_vorbis_get_frame_float(f, None, Some(&mut output)) as usize;
   let len = std::cmp::min(len, sample_buffer.len());
   
   if len != 0 {
        convert_samples_short(num_c, &mut sample_buffer, &output, len);
   }
   return len as i32;
}


fn convert_samples_short(buf_c: i32, buffer: &mut AudioBufferSlice<i16>, data: &AudioBufferSlice<f32>, samples: usize)
{
   let buf_c = buf_c as usize;
   if buf_c != data.channel_count() && buf_c <= 2 && data.channel_count() <= 6 {
      static CHANNEL_SELECTOR : [[i8;2]; 3] = [
          [0, 0],
          [PLAYBACK_MONO, PLAYBACK_MONO],
          [PLAYBACK_LEFT, PLAYBACK_RIGHT]
      ];
      
      for i in 0 .. buf_c {
         compute_samples(CHANNEL_SELECTOR[buf_c][i] as i32, &mut buffer[i], 
            data, samples);
      }
   } else {
      let limit = std::cmp::min(buf_c, data.channel_count());
      
      let mut i = 0;
      while i < limit {
         let mut buffer_slice = &mut buffer[i]; 
         let data_slice = &data[i];
         copy_samples(&mut buffer_slice, data_slice, samples);
         i += 1;
      }
      
      while i < buf_c {
          unsafe {
            std::ptr::write_bytes(
                (&mut buffer[i]).as_mut_ptr(),
                0, samples);
          }
          i += 1;
      }
   }
}


fn convert_channels_short_interleaved(buf_c: u32, buffer: &mut [i16], data: &AudioBufferSlice<f32>, len: usize)
{
   if buf_c != data.channel_count() as u32 && buf_c <= 2 && data.channel_count() <= 6 {
       assert!(buf_c == 2);
       for _ in 0 .. buf_c {
         compute_stereo_samples(buffer, data, len);
       }
   } else {
       let limit = std::cmp::min(buf_c as usize, data.channel_count()) as usize;
       let mut buffer_index = 0;
       for j in 0 .. len as usize {
           let mut i = 0;
           while i < limit {
               buffer[buffer_index] = convert_to_i16(data[i][ j as usize ]);
               buffer_index += 1;
               
               i += 1;
           }
           
           while i < buf_c as usize {
               buffer[buffer_index] = 0;
               buffer_index += 1;
               i += 1;
           }
       }
       
   }
}

fn copy_samples(dest: &mut [i16], src: &[f32], len: usize)
{
   for i in 0 .. len  {
      dest[i] = convert_to_i16(src[i]);
   }
}

// these functions seek in the Vorbis file to (approximately) 'sample_number'.
// after calling seek_frame(), the next call to get_frame_*() will include
// the specified sample. after calling stb_vorbis_seek(), the next call to
// stb_vorbis_get_samples_* will start with the specified sample. If you
// do not need to seek to EXACTLY the target sample when using get_samples_*,
// you can also use seek_frame().
pub fn stb_vorbis_seek(f: &mut Vorbis, sample_number: u32) -> bool
{
   if stb_vorbis_seek_frame(f, sample_number) == false {
      return false;
   }

   if sample_number != f.current_loc {
      let mut n = 0;
      let frame_start = f.current_loc;
      stb_vorbis_get_frame_float(f, Some(&mut n), None);
      assert!(sample_number > frame_start);
      assert!(f.channel_buffer_start + (sample_number-frame_start) as i32 <= f.channel_buffer_end);
      f.channel_buffer_start += (sample_number - frame_start) as i32;
   }

   return true;
}


fn init_blocksize(f: &mut Vorbis, b: usize, n: usize)
{
   let n2 = n >> 1;
   let n4 = n >> 2;
   let n8 = n >> 3;
   
   f.a[b].resize(n2, 0.0);
   f.b[b].resize(n2, 0.0);
   f.c[b].resize(n4, 0.0);
   
   compute_twiddle_factors(n as i32, &mut f.a[b], &mut f.b[b], &mut f.c[b]);
   
   f.window[b].resize(n2, 0.0);
   compute_window(n as i32, &mut f.window[b]);

   f.bit_reverse[b].resize(n8, 0);
   compute_bitreverse(n as i32, &mut f.bit_reverse[b]);
}

// accelerated huffman table allows fast O(1) match of all symbols
// of length <= STB_FAST_HUFFMAN_LENGTH

fn compute_accelerated_huffman(c: &mut Codebook)
{
   for i in 0 .. FAST_HUFFMAN_TABLE_SIZE as usize {
       c.fast_huffman[i] = -1;
   }


   let len = if c.sparse == true { c.sorted_entries } else  {c.entries};
   let len = std::cmp::min(len, 32767);// largest possible value we can encode!
   
   for i in 0 .. len as usize {
      if c.codeword_lengths[i] <= STB_FAST_HUFFMAN_LENGTH as u8 {
         let mut z : u32 = if c.sparse == true { 
             bit_reverse(c.sorted_codewords[i]) 
         } else { 
             c.codewords[i] 
        };
         // set table entries for all bit combinations in the higher bits
         while z < FAST_HUFFMAN_TABLE_SIZE as u32 {
             c.fast_huffman[z as usize] = i as i16;
             z += 1 << c.codeword_lengths[i as usize];
         }
      }
   }
}

// returns the current seek point within the file, or offset from the beginning
// of the memory buffer. In pushdata mode it returns 0.
pub fn stb_vorbis_get_file_offset(f: &mut Vorbis) -> u32
{
   if f.push_mode == true {return 0;}
   if !f.stream.is_null() {return (f.stream as usize - f.stream_start as usize) as u32;}

   let mut file = f.f.as_mut().unwrap();
   let current = file.seek(SeekFrom::Current(0)).unwrap();
   
   // NOTE(bungcip): change to u64/i64?
   return (current as u32 - f.f_start) as u32;
}

fn start_page_no_capturepattern(f: &mut Vorbis) -> bool
{
    use VorbisError::*;
    
   // stream structure version
   if 0 != get8(f) {return error(f, InvalidStreamStructureVersion);}
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
   unsafe {
        let segments_slice = {
            let sc = f.segment_count as usize;
            FORCE_BORROW_MUT!(&mut f.segments[0 .. sc])
        };
        if getn(f, segments_slice) == false {
            return error(f, UnexpectedEof);
        }
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

   if f.first_decode == true {
      let mut len : i32 = 0;
      for i in 0 .. f.segment_count as usize {
         len += f.segments[i] as i32;
      }
      len += 27 + f.segment_count as i32;
      
      let p = ProbedPage {
          page_start: f.first_audio_page_offset,
          page_end: f.first_audio_page_offset + len as u32,
          last_decoded_sample: loc0
      };
      
      f.p_first = p;
   }
   f.next_seg = 0;
   return true;
}

fn predict_point(x: i32, x0: i32 , x1: i32 , y0: i32 , y1: i32 ) -> i32
{
   let dy = y1 - y0;
   let adx = x1 - x0;
   // @OPTIMIZE: force int division to round in the right direction... is this necessary on x86?
   use std::i32;
   let err = i32::abs(dy) * (x - x0);
   let off = err / adx;
   return if dy < 0  {y0 - off} else {y0 + off};
}

fn do_floor(f: &mut Vorbis, map: &Mapping, i: usize, n: usize , target: &mut [f32], final_y: &[YTYPE]) -> bool
{
   let n2 = n >> 1;

   let s : &MappingChannel = unsafe { FORCE_BORROW!( &map.chan[i] ) };
   let s = s.mux as usize;
   let floor = map.submap_floor[s] as usize;
   
   if f.floor_types[floor] == 0 {
      return error(f, VorbisError::InvalidStream);
   } else {
      let g : &Floor1 = unsafe { FORCE_BORROW!( &f.floor_config[floor].floor1 ) };
      let mut lx = 0;
      let mut ly = final_y[0] as i32 * g.floor1_multiplier as i32;
      for q in 1 .. g.values as usize {
         let j = g.sorted_order[q] as usize;
         if final_y[j] >= 0
         {
            let hy : i32 = final_y[j] as i32 * g.floor1_multiplier as i32;
            let hx : i32 = g.xlist[j] as i32;
            if lx != hx {
               draw_line(target, lx,ly, hx, hy, n2 as i32);
            }
            CHECK!(f);
            lx = hx;
            ly = hy;
         }
      }
      
      let lx = lx as usize;
      if lx < n2 {
         // optimization of: draw_line(target, lx,ly, n,ly, n2);
         for j in lx .. n2 {
            LINE_OP!(target[j], INVERSE_DB_TABLE[ly as usize]);
         }
         CHECK!(f);
      }
   }
   return true;
}

#[inline(always)]
fn draw_line(output: &mut [f32], x0: i32, y0: i32, mut x1: i32, y1: i32, n: i32)
{
    use std::i32;
    
   let dy = y1 - y0;
   let adx = x1 - x0;
   let mut ady = i32::abs(dy);
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


   ady -= i32::abs(base) * adx;
   
   if x1 > n {x1 = n;}
   if x < x1 {
      LINE_OP!(output[x as usize], INVERSE_DB_TABLE[y as usize]);
      
      x += 1;
      while x < x1 {
         err += ady;
          
         if err >= adx {
            err -= adx;
            y += sy;
         } else{
            y += base;
         }
         LINE_OP!(output[x as usize], INVERSE_DB_TABLE[y as usize]);
         
         x += 1;
      }      
   }
}


fn residue_decode(f: &mut Vorbis, book: &Codebook, target: &mut [f32], mut offset: i32, n: i32, rtype: i32) -> bool
{
   if rtype == 0 {
      let step = n / book.dimensions;
      for k in 0 .. step {
            // let mut target_slice = std::slice::from_raw_parts_mut(
            //     target.offset((offset+k) as isize), (n-offset-k) as usize);
            // NOTE(bungcip): simplify this!
            let mut target_slice = &mut target[ (offset+k) as usize .. ( (offset+k) + (n-offset-k) ) as usize ];
            if codebook_decode_step(f, book, &mut target_slice, n-offset-k, step) == false {
                return false;
            }
      }
   } else {
       let mut k = 0;
       while k < n {
            // NOTE(bungcip): simplify this!
            let mut target_slice = &mut target[ offset as usize .. (offset+n-k) as usize];
            // let mut target_slice = std::slice::from_raw_parts_mut(
            //     target.offset(offset as isize), (n-k) as usize);
                
            if codebook_decode(f, book, &mut target_slice, n-k) == false {
                return false;
            }
            k += book.dimensions;
            offset += book.dimensions;
       }
   }
   return true;
}


fn codebook_decode(f: &mut Vorbis, c: &Codebook, output: &mut [f32], len: i32 ) -> bool
{
   let mut z = codebook_decode_start(f,c);
   if z < 0 {
       return false;
   }

   let len = std::cmp::min(len, c.dimensions);

   z *= c.dimensions;
   if c.sequence_p != 0 {
      let mut last : f32 = 0.0;
      for i in 0 .. len  {
         let val : f32 = c.multiplicands[(z+i) as usize] + last;
         output[i as usize] += val;
         last = val + c.minimum_value;
      }
   } else {
      let last : f32 = 0.0;
      for i in 0 .. len  {
         output[i as usize] += c.multiplicands[(z+i) as usize] + last;
      }
   }

   return true;
}


#[inline(always)]
fn decode_raw(f: &mut Vorbis, codebook: &Codebook) -> i32 {
    let mut value = codebook_decode_scalar(f, codebook);
    if codebook.sparse == true {
        value = codebook.sorted_values[value as usize];
    }
    value
}


fn codebook_decode_start(f: &mut Vorbis, c: &Codebook) -> i32
{
   let mut z = -1;

   // type 0 is only legal in a scalar context
   if c.lookup_type == 0 {
      error(f, VorbisError::InvalidStream);
   } else {
      z = codebook_decode_scalar(f, c);
      if c.sparse == true {assert!(z < c.sorted_entries);}
      if z < 0 && f.bytes_in_seg == 0 { // check for EOP
         if f.last_seg == true {
            return z;
         }
         error(f,  VorbisError::InvalidStream);
      }
   }
   return z;
}

#[inline(always)]
fn codebook_decode_scalar(f: &mut Vorbis, c: &Codebook) -> i32
{
   if f.valid_bits < STB_FAST_HUFFMAN_LENGTH {
      prep_huffman(f);
   }
   // fast huffman table lookup
   let i = (f.acc & FAST_HUFFMAN_TABLE_MASK as u32) as usize;
   let i = c.fast_huffman[i] as i32;
   if i >= 0 {
      f.acc >>= c.codeword_lengths[i as usize];
      f.valid_bits -= c.codeword_lengths[i as usize] as i32;
      if f.valid_bits < 0 { 
          f.valid_bits = 0;
          return -1;
      }
      return i;
   }
   return codebook_decode_scalar_raw(f,c);
}

// @OPTIMIZE: primary accumulator for huffman
// expand the buffer to as many bits as possible without reading off end of packet
// it might be nice to allow f->valid_bits and f->acc to be stored in registers,
// e.g. cache them locally and decode locally
#[inline(always)]
fn prep_huffman(f: &mut Vorbis)
{
   if f.valid_bits <= 24 {
      if f.valid_bits == 0 {f.acc = 0;}
      
      while {
         if f.last_seg == true && f.bytes_in_seg == 0 {return;}
         let z : i32 = get8_packet_raw(f);
         if z == EOP {return;}
         f.acc += (z as u32) << f.valid_bits;
         f.valid_bits += 8;
          
         // condition
         f.valid_bits <= 24
      }{/* do nothing */}
   }
}

fn codebook_decode_scalar_raw(f: &mut Vorbis, c: &Codebook) -> i32
{
   prep_huffman(f);

   if c.codewords.is_empty() && c.sorted_codewords.is_empty() {
      return -1;
   }

   // cases to use binary search: sorted_codewords && !c.codewords
   //                             sorted_codewords && c.entries > 8
   let case = if c.entries > 8 {
       !c.sorted_codewords.is_empty()
   }else{
       c.codewords.is_empty()
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
         if c.sorted_codewords[m as usize] <= code {
            x = m;
            n -= n>>1;
         } else {
            n >>= 1;
         }
      }
      // x is now the sorted index
      if c.sparse == false {
          x = c.sorted_values[x as usize];
      }
      // x is now sorted index if sparse, or symbol otherwise
      len = c.codeword_lengths[x as usize] as i32;
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
   for i in 0 .. c.entries as usize  {
      if c.codeword_lengths[i] == NO_CODE {
          continue;
      }
      if c.codewords[i] == (f.acc & ((1 << c.codeword_lengths[i])-1)) {
         if f.valid_bits >= c.codeword_lengths[i] as i32 {
            f.acc >>= c.codeword_lengths[i];
            f.valid_bits -= c.codeword_lengths[i] as i32;
            return i as i32;
         }
         f.valid_bits = 0;
         return -1;
      }
   }

   error(f, VorbisError::InvalidStream);
   f.valid_bits = 0;
   return -1;
}

fn codebook_decode_step(f: &mut Vorbis, c: &Codebook, output: &mut [f32], len: i32 , step: i32 ) -> bool
{
   let mut z = codebook_decode_start(f,c);
   if z < 0 {
       return false;
   }
   
   z *= c.dimensions;
   let mut last : f32 = 0.0;
   let len = std::cmp::min(len, c.dimensions); 
   for i in 0 .. len  {
      let val : f32 = c.multiplicands[(z+i) as usize] + last;
      output[ (i*step) as usize] += val;
      if c.sequence_p != 0 {
          last = val;
      }
   }

   return true;
}

unsafe fn codebook_decode_deinterleave_repeat(f: &mut Vorbis, c: &Codebook, outputs: &mut AudioBufferSlice<f32>, 
    c_inter_p: &mut i32, p_inter_p: &mut i32, len: i32, mut total_decode: i32) -> bool
{
   let ch = outputs.channel_count() as i32;
   let mut c_inter = *c_inter_p;
   let mut p_inter = *p_inter_p;
   let mut effective = c.dimensions;
   let mut z : i32;

   // type 0 is only legal in a scalar context
   if c.lookup_type == 0 {
     return error(f, VorbisError::InvalidStream);
   } 
   while total_decode > 0 {
      let mut last : f32 = 0.0;
      z = codebook_decode_scalar(f, c);
      assert!(c.sparse == false || z < c.sorted_entries);
      if z < 0 {
         if f.bytes_in_seg == 0{
            if f.last_seg == true {
              return false;
            } 
         }
         return error(f, VorbisError::InvalidStream);
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
               let val : f32 = c.multiplicands[(z+i) as usize] + last;
               if outputs[c_inter as usize].len() > 0 {
                   outputs[c_inter as usize][p_inter as usize] += val;
               }
               c_inter += 1;
               if c_inter == ch {
                   c_inter = 0;
                   p_inter += 1;
               }
               last = val;
            }
         } else {
            for i in 0 .. effective {
               let val : f32 = c.multiplicands[(z+i) as usize] + last;
               if outputs[c_inter as usize].len() > 0 {
                   outputs[c_inter as usize][p_inter as usize] += val;
               }
               c_inter += 1;
               if c_inter == ch { 
                   c_inter = 0; 
                   p_inter += 1;
               }
            }
         }
      }

      total_decode -= effective;
   }
   *c_inter_p = c_inter;
   *p_inter_p = p_inter;

   return true;
}

fn compute_window(n: i32, window: &mut [f32])
{
   let n2 : i32 = n >> 1;
   for i in 0 .. n2 {
      window[i as usize] = 
            f64::sin(
                0.5 as f64 * 
                M_PI as f64 * 
                square(
                    f64::sin((i as f64 - 0 as f64 + 0.5) / n2 as f64 * 0.5 * M_PI as f64) as f32
                ) as f64
            ) as f32;
   }
}

fn compute_samples(mask: i32, output: &mut [i16], data: &AudioBufferSlice<f32>, len: usize)
{
   const BUFFER_SIZE : usize = 32;
   let mut buffer: [f32; BUFFER_SIZE];
   let mut n = BUFFER_SIZE;
   let mut o : usize = 0;
   let len = len as usize;
   
   while o < len {
      buffer = [0.0; BUFFER_SIZE];
      
      if o + n > len {
          n = len - o;
      }
      for j in 0 .. data.channel_count() {
         if (CHANNEL_POSITION[data.channel_count()][j] as i32 & mask) != 0 {
            for i in 0 .. n {
               buffer[i] += data[j][o+i];
            }
         }
      }
      for i in 0 .. n  {
         output[ (o+i) as usize] = convert_to_i16(buffer[i]);
      }
       
       o += BUFFER_SIZE;
   }
}

fn compute_stereo_samples(output: &mut [i16], data: &AudioBufferSlice<f32>, len: usize)
{
   const BUFFER_SIZE : usize = 32;
   
   let mut n = BUFFER_SIZE >> 1;
   let mut buffer: [f32; BUFFER_SIZE];
   // o is the offset in the source data
   let mut o = 0;
   while o < len {
      // o2 is the offset in the output data
      let o2 = o << 1;
      buffer = [0.0; BUFFER_SIZE];
      
      if o + n > len {
          n = len - o;
      }
      for j in 0 .. data.channel_count() {
         let m = CHANNEL_POSITION[data.channel_count()][j] & (PLAYBACK_LEFT | PLAYBACK_RIGHT);
         if m == (PLAYBACK_LEFT | PLAYBACK_RIGHT) {
            for i in 0 .. n as usize {
               buffer[ (i*2+0) ] += data[j][o + i];
               buffer[ (i*2+1) ] += data[j][o + i];
            }
         } else if m == PLAYBACK_LEFT {
            for i in 0 .. n as usize {
               buffer[ (i*2+0) ] += data[j][o + i];
            }
         } else if m == PLAYBACK_RIGHT {
            for i in 0 .. n as usize {
               buffer[ (i*2+1) ] += data[j][o + i];
            }
         }
      }
      
      
      for i in 0 .. n << 1 {
         let mut v : i32 = unsafe { FAST_SCALED_FLOAT_TO_INT!(buffer[i],15) };
         if (v + 32768) as u32 > 65535 {
            v = if v < 0 {-32768} else {32767};
         }
         
         output[ (o2+i) as usize ] = v as i16;
      }
       
       o += BUFFER_SIZE >> 1;
   }

}

// these functions seek in the Vorbis file to (approximately) 'sample_number'.
// after calling seek_frame(), the next call to get_frame_*() will include
// the specified sample. after calling stb_vorbis_seek(), the next call to
// stb_vorbis_get_samples_* will start with the specified sample. If you
// do not need to seek to EXACTLY the target sample when using get_samples_*,
// you can also use seek_frame().
pub fn stb_vorbis_seek_frame(f: &mut Vorbis, sample_number: u32) -> bool
{
   if f.push_mode { 
       return error(f, VorbisError::InvalidApiMixing);
   }

   // fast page-level search
   if seek_to_sample_coarse(f, sample_number) == false {
      return false;
   }

   assert!(f.current_loc_valid == true);
   assert!(f.current_loc <= sample_number);

   // linear search for the relevant packet
   let max_frame_samples = ((f.blocksize_1*3 - f.blocksize_0) >> 2) as u32;
   while f.current_loc < sample_number {
      let mut left_start = 0; 
      let mut left_end = 0;
      let mut right_start = 0;
      let mut right_end = 0;
      let mut mode = 0;
      let frame_samples: i32;
      if peek_decode_initial(f, &mut left_start, &mut left_end, &mut right_start, &mut right_end, &mut mode) == false{
         return error(f, VorbisError::SeekFailed);
      }
      // calculate the number of samples returned by the next frame
      frame_samples = right_start - left_start;
      if f.current_loc as i32 + frame_samples > sample_number as i32 {
         return true; // the next frame will contain the sample
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
   return true;
}

// get the last error detected (clears it, too)
pub fn stb_vorbis_get_error(f: &mut Vorbis) -> VorbisError
{
   let e = f.error;
   f.error = VorbisError::NoError;
   return e;
}

// this function is equivalent to stb_vorbis_seek(f,0)
pub fn stb_vorbis_seek_start(f: &mut Vorbis)
{
   if f.push_mode { 
       error(f, VorbisError::InvalidApiMixing); 
       return;
   }
   
   let offset = f.first_audio_page_offset;
   set_file_offset(f, offset);
   f.previous_length = 0;
   f.first_decode = true;
   f.next_seg = -1;
   vorbis_pump_first_frame(f);
}

// these functions return the total length of the vorbis stream
#[allow(unreachable_code, unused_variables)]
pub fn stb_vorbis_stream_length_in_seconds(f: &mut Vorbis) -> f32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   return stb_vorbis_stream_length_in_samples(f) as f32 / f.sample_rate as f32;
}

// this function returns the offset (in samples) from the beginning of the
// file that will be returned by the next decode, if it is known, or -1
// otherwise. after a flush_pushdata() call, this may take a while before
// it becomes valid again.
// NOT WORKING YET after a seek with PULLDATA API
#[allow(unreachable_code, unused_variables)]
pub fn stb_vorbis_get_sample_offset(f: &mut Vorbis) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   if f.current_loc_valid == true {
      return f.current_loc as i32;
   } else {
      return -1;
   }
}

// get general information about the file
pub fn stb_vorbis_get_info(f: &Vorbis) -> VorbisInfo
{
   VorbisInfo {
       channels: f.channels,
       sample_rate: f.sample_rate,
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
#[allow(unreachable_code, unused_variables)]
pub fn stb_vorbis_flush_pushdata(f: &mut Vorbis)
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");
   f.previous_length = 0;
   f.page_crc_tests  = 0;
   f.discard_samples_deferred = 0;
   f.current_loc_valid = false;
   f.first_decode = false; 
   f.samples_output = 0;
   f.channel_buffer_start = 0;
   f.channel_buffer_end = 0;
}

// create an ogg vorbis decoder from an ogg vorbis stream in memory (note
// this must be the entire stream!). on failure, returns NULL and sets *error
pub unsafe fn stb_vorbis_open_memory(data: &[u8], error: &mut VorbisError) -> Option<Vorbis>
{
   if data.len() == 0 {
     return None;       
   } 
   
   let mut p = Vorbis::new();
   
   p.stream_len = data.len() as u32;
   p.stream = data.as_ptr();
   p.stream_end = data.as_ptr().offset(data.len() as isize);
   p.stream_start = p.stream;
   p.push_mode = false;
   
   if start_decoder(&mut p) == true {
        vorbis_pump_first_frame(&mut p);
        return Some(p);
   }
   
    *error = p.error;
    return None;
}

// decode an entire file and output the data interleaved into a malloc()ed
// buffer stored in *output. The return value is the number of samples
// decoded, or -1 if the file could not be opened or was not an ogg vorbis file.
// When you're done with it, just free() the pointer returned in *output.
pub fn stb_vorbis_decode_memory(mem: &[u8],
     channels: &mut u32, sample_rate: &mut u32, output: &mut Vec<i16>) -> i32
{
   let mut error = VorbisError::NoError;
   let mut v : Vorbis = unsafe { match stb_vorbis_open_memory(mem, &mut error){
       None    => return -1,
       Some(v) => v
   }};
   
   *channels = v.channels as u32;
   *sample_rate = v.sample_rate;
   
   let mut offset = 0;
   let mut data_len = 0;
   let limit = v.channels as usize * 4096;
   let mut total = limit;
   
   output.resize(total as usize, 0);
   
   loop {
       let ch = v.channels as u32;
       let n = {
           let mut output_slice = output.as_mut_slice();
           stb_vorbis_get_frame_short_interleaved(
               &mut v, ch, 
               &mut output_slice[ offset .. total ]
            )
       };

      if n == 0 {
        break;  
      }
         
      data_len += n;
      offset += n as usize * v.channels as usize;
      
      if offset + limit > total {
         total *= 2;
         output.resize(total as usize, 0);
      }
   }

   // resize to fit data_len
   output.resize( (data_len * v.channels) as usize , 0);
   return data_len;

}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. DOES NOT APPLY THE COERCION RULES.
// Returns the number of samples stored per channel; it may be less than requested
// at the end of the file. If there are no more samples in the file, returns 0.
pub fn stb_vorbis_get_samples_float(f: &mut Vorbis, channels: i32 , buffer: &mut AudioBufferSlice<f32>) -> i32
{
   let mut outputs: AudioBufferSlice<f32> = AudioBufferSlice::new(0);
   let mut n = 0;
   let num_samples = buffer.len() as i32;
   let z = std::cmp::min(f.channels, channels);
   while n < num_samples {
      let mut k = f.channel_buffer_end - f.channel_buffer_start;
      if n + k >= num_samples { k = num_samples - n; }
      if k != 0 {
          let mut i = 0;
          while i < z {
              unsafe {
                std::ptr::copy_nonoverlapping(
                    f.channel_buffers[i as usize].as_ptr().offset(f.channel_buffer_start as isize),
                    buffer[i as usize].as_mut_ptr().offset(n as isize),
                    k as usize
                );
              }
            i += 1;
          }
          
          while i < channels {
              unsafe{
                std::ptr::write_bytes(
                    buffer[i as usize].as_mut_ptr().offset(n as isize),
                    0,
                    k as usize
                );        
              }
            i += 1;
          }          
      }
      n += k;
      f.channel_buffer_start += k;
      if n == num_samples{
         break;
      }
      if stb_vorbis_get_frame_float(f, None, Some(&mut outputs)) == 0 {
         break;
      }
   }
   return n;
}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. DOES NOT APPLY THE COERCION RULES.
// Returns the number of samples stored per channel; it may be less than requested
// at the end of the file. If there are no more samples in the file, returns 0.
pub fn stb_vorbis_get_samples_float_interleaved(f: &mut Vorbis, channels: i32 , mut buffer: &mut [f32]) -> i32 
{
   let mut outputs: AudioBufferSlice<f32> = AudioBufferSlice::new(0);
   let len : i32 = buffer.len() as i32 / channels;
   let mut n=0;
   let z = std::cmp::min(f.channels, channels);
   
   let mut buffer_index = 0;
   while n < len {
      let mut k = f.channel_buffer_end - f.channel_buffer_start;
      if n+k >= len {k = len - n;}
      for j in 0 .. k  {
          let mut i = 0;
          while i < z {
            buffer[buffer_index] = f.channel_buffers[i as usize][ (f.channel_buffer_start+j) as usize];
            buffer_index += 1;
            i += 1;
          }
          
          while i < channels {
            buffer[buffer_index] = 0.0;
            buffer_index += 1;
              i += 1;
          }
      }
      n += k;
      f.channel_buffer_start += k;
      if n == len{
         break;
      }
      if stb_vorbis_get_frame_float(f, None, Some(&mut outputs)) == 0{
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
pub unsafe fn stb_vorbis_get_samples_short(f: &mut Vorbis, channels: i32, buffer: &mut AudioBufferSlice<i16>) -> u32
{
   let mut outputs: AudioBufferSlice<f32> = AudioBufferSlice::new(channels as usize);
   let mut n = 0;
   let len = buffer.len();

   while n < len {
      let mut k = (f.channel_buffer_end - f.channel_buffer_start) as usize;
      if n+k >= len {k = len - n;}
      if k != 0 {
         let channel_buffers_slice = AudioBufferSlice::from(&mut f.channel_buffers);
         let channel_buffers_slice = channel_buffers_slice.range_from(f.channel_buffer_start as usize);
         let mut buffer = buffer.range_from(n as usize);
         convert_samples_short(channels, &mut buffer, &channel_buffers_slice, k); 
      }
      n += k;
      f.channel_buffer_start += k as i32;
      if n == len{ break;}
      if stb_vorbis_get_frame_float(f, None, Some(&mut outputs)) == 0 {break;}
   }
   return n as u32;
}

// gets num_samples samples, not necessarily on a frame boundary--this requires
// buffering so you have to supply the buffers. Applies the coercion rules above
// to produce 'channel_count' channels. Returns the number of samples stored per channel;
// it may be less than requested at the end of the file. If there are no more
// samples in the file, returns 0.
pub fn stb_vorbis_get_samples_short_interleaved(f: &mut Vorbis, channel_count: u32, mut buffer: &mut [i16]) -> i32
{
   let mut outputs: AudioBufferSlice<f32> = AudioBufferSlice::new(0);
   let len_per_channel = buffer.len() / channel_count as usize;
   let mut n = 0;
   let mut buffer_offset = 0;
   while n < len_per_channel {
      let k = {
        let mut k = (f.channel_buffer_end - f.channel_buffer_start) as usize;
        if n + k >= len_per_channel {
            k = len_per_channel - n;
        }
        k
      };

      if k != 0 {
          // NOTE(bungcip): create type AudioBuffer too
         let audio_buffer_slice = unsafe { AudioBufferSlice::from(&mut f.channel_buffers) };
         let audio_buffer_slice = audio_buffer_slice.range_from(f.channel_buffer_start as usize);
          
         convert_channels_short_interleaved(
             channel_count, &mut buffer[buffer_offset ..], 
             &audio_buffer_slice, 
             k);
      }
      buffer_offset += k * channel_count as usize;
      n += k as usize;
      f.channel_buffer_start += k as i32;
      if n == len_per_channel {
        break;
      }

      // NOTE(bungcip): outputs not used? maybe can be changed to None
      if stb_vorbis_get_frame_float(f, None, Some(&mut outputs)) == 0 {
          break;
      }
   }
   
   return n as i32;
}

// the same as vorbis_decode_initial, but without advancing
#[allow(unreachable_code, unused_variables)]
fn peek_decode_initial(f: &mut Vorbis, p_left_start: &mut i32, p_left_end: &mut i32, p_right_start: &mut i32, p_right_end: &mut i32, mode: &mut i32) -> bool
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   if vorbis_decode_initial(f, p_left_start, p_left_end, p_right_start, p_right_end, mode) == false {
      return false;
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
   } else {
      f.next_seg -= 1;
   }
   f.valid_bits = 0;

   return true;
}

fn set_file_offset(f: &mut Vorbis, mut loc: u32) -> bool
{
   if f.push_mode == true {return false;}
   f.eof = false;
   if !f.stream.is_null() {
      unsafe {
        if f.stream_start.offset(loc as isize)  >= f.stream_end || f.stream_start.offset(loc as isize) < f.stream_start {
            f.stream = f.stream_end;
            f.eof = true;
            return false;
        } else {
            f.stream = f.stream_start.offset(loc as isize);
            return true;
        }
      }
   }
   if loc + f.f_start < loc || loc >= 0x80000000 {
      loc = 0x7fffffff;
      f.eof = true;
   } else {
      loc += f.f_start;
   }

   let mut file = f.f.as_mut().unwrap();
   match file.seek(SeekFrom::Start(loc as u64)) {
       Ok(_)  => return true,
       Err(_) => {
           f.eof = true;
           file.seek(SeekFrom::End(f.f_start as i64)).unwrap();
           return false;
       }
   }
}

// rarely used function to seek back to the preceeding page while finding the
// start of a packet
#[allow(unreachable_code, unused_variables)]
unsafe fn go_to_page_before(f: &mut Vorbis, limit_offset: u32) -> bool
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   // now we want to seek back 64K from the limit
   let previous_safe : u32;
   if limit_offset >= 65536 && limit_offset-65536 >= f.first_audio_page_offset {
      previous_safe = limit_offset - 65536;
   } else {
      previous_safe = f.first_audio_page_offset;
   }

   set_file_offset(f, previous_safe);

   let mut end: u32 = 0;
   while vorbis_find_page(f, Some(&mut end), None) != 0 {
      if end >= limit_offset && stb_vorbis_get_file_offset(f) < limit_offset {
         return true;          
      }
      set_file_offset(f, end);
   }

   return false;
}

// NOTE(bungcip): change signature to Result
fn vorbis_find_page(f: &mut Vorbis, end: Option<&mut u32>, last: Option<&mut u32>) -> u32
{
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
            if get8(f) != OGG_PAGE_HEADER[i]{
               break;
            }
             i += 1;
         }
         if f.eof == true {return 0;}
         if i == 4 {
            let mut header: [u8; 27] = [0; 27];
            let mut i : usize = 0;
            while i < 4 {
               header[i] = OGG_PAGE_HEADER[i];
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
               crc = unsafe { crc32_update(crc, header[i]) };
            }
            let mut len = 0;
            for _ in 0 .. header[26] {
               let s = get8(f) as i32;
               crc = unsafe { crc32_update(crc, s as u8) };
               len += s;
            }
            if len != 0 && f.eof == true {return 0;}
            for _ in 0 .. len {
               crc = unsafe { crc32_update(crc, get8(f)) };
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
               if let Some(end) = end {
                  *end = stb_vorbis_get_file_offset(f);
               }
               if let Some(last) = last {
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
   return (crc << 8) ^ CRC_TABLE[ (byte as u32 ^ (crc >> 24)) as usize];
}


// seeking is implemented with a binary search, which narrows down the range to
// 64K, before using a linear search (because finding the synchronization
// pattern can be expensive, and the chance we'd find the end page again is
// relatively high for small ranges)
//
// two initial interpolation-style probes are used at the start of the search
// to try to bound either side of the binary search sensibly, while still
// working in O(log n) time if they fail.

#[allow(unreachable_code, unused_variables)]
fn get_seek_page_info(f: &mut Vorbis, z: &mut ProbedPage) -> bool
{
  panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   // record where the page starts
   z.page_start = stb_vorbis_get_file_offset(f);

   // parse the header
   let mut header: [u8; 27] = [0; 27];
   getn(f, &mut header[..]);
   if header[0] != b'O' || header[1] != b'g' || header[2] != b'g' || header[3] != b'S'{
      return false;
   }

   let mut lacing: [u8; 255] = [0; 255];
   getn(f, &mut lacing[..]);

   // determine the length of the payload
   let mut len = 0;
   for i in 0 .. header[26] as usize {
      len += lacing[i];
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
   return true;
}

// create a vorbis decoder by passing in the initial data block containing
//    the ogg&vorbis headers (you don't need to do parse them, just provide
//    the first N bytes of the file--you're told if it's not enough, see below)
// on success, returns an stb_vorbis *, does not set error, returns the amount of
//    data parsed/consumed on this call in *datablock_memory_consumed_in_bytes;
// on failure, returns NULL on error and sets *error, does not change *datablock_memory_consumed
// if returns NULL and *error is NeedMoreData, then the input block was
//       incomplete and you need to pass in a larger block from the start of the file
pub fn stb_vorbis_open_pushdata(
         data: &[u8],                      // the memory available for decoding
         data_used: &mut i32,              // only defined if result is not NULL
         error: &mut VorbisError)
         -> Option<Vorbis>
{

   let mut p = Vorbis::new();
   let start_position = data.as_ptr() as usize;
   unsafe {
        p.stream     = data.as_ptr();
        p.stream_end = p.stream.offset(data.len() as isize);
        p.push_mode  = true;
        if start_decoder(&mut p) == false {
            if p.eof == true {
                *error = VorbisError::NeedMoreData;
            } else {
                *error = p.error;
            }
            return None;
        }
   }
   
    *data_used = (p.stream as usize - start_position) as i32;
    *error = VorbisError::NoError;
    return Some(p);
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
         f: &mut Vorbis,                   // the file we're decoding
         data: &[u8] ,                     // the memory available for decoding
         channels: &mut i32,               // place to write number of float * buffers
         output: &mut AudioBufferSlice<f32>,       // place to write float ** array of float * buffers
         samples: &mut i32                 // place to write number of output samples
     ) -> i32
{

   if f.push_mode == false { 
       error(f, VorbisError::InvalidApiMixing);
       return 0;
    };
    
   let data_len = data.len() as i32;

   if f.page_crc_tests >= 0 {
      *samples = 0;
      return vorbis_search_for_page_pushdata(f, data);
   }

   f.stream     = data.as_ptr();
   f.stream_end = f.stream.offset(data_len as isize) as *mut u8;
   f.error      = VorbisError::NoError;

   // check that we have the entire packet in memory
   if is_whole_packet_present(f, false) == false {
      *samples = 0;
      return 0;
   }

   let mut len : i32 = 0;
   let mut left: i32 = 0;
   let mut right: i32 = 0;
   if vorbis_decode_packet(f, &mut len, &mut left, &mut right) == false {
      // save the actual error we encountered
      let error = f.error;
      if error == VorbisError::BadPacketType {
         // flush and resynch
         f.error = VorbisError::NoError;
         while get8_packet(f) != EOP{
            if f.eof == true {break;}
         }
         *samples = 0;
         return (f.stream as usize - data.as_ptr() as usize) as i32;
      }
      if error == VorbisError::ContinuedPacketFlagInvalid {
         if f.previous_length == 0 {
            // we may be resynching, in which case it's ok to hit one
            // of these; just discard the packet
            f.error = VorbisError::NoError;
            while get8_packet(f) != EOP{
                if f.eof == true {break;}
            }
            *samples = 0;
            return (f.stream as usize - data.as_ptr() as usize) as i32;
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
   for i in 0 .. f.channels as usize {
        f.outputs.set(i, &mut f.channel_buffers[i][left as usize ..]);
   }

   *channels = f.channels;
   *samples = len;
   
   *output = f.outputs;
    return (f.stream as usize - data.as_ptr() as usize) as i32;
}


unsafe fn is_whole_packet_present(f: &mut Vorbis, end_page: bool) -> bool
{
   // make sure that we have the packet available before continuing...
   // this requires a full ogg parse, but we know we can fetch from f->stream

   // instead of coding this out explicitly, we could save the current read state,
   // read the next packet with get8() until end-of-packet, check f->eof, then
   // reset the state? but that would be slower, esp. since we'd have over 256 bytes
   // of state to restore (primarily the page segment table)

   let mut s = f.next_seg;
   let mut first = true;
   let mut p = f.stream;

   if s != -1 { // if we're not starting the packet with a 'continue on next page' flag
      while s < f.segment_count {
         p = p.offset( f.segments[s as usize] as isize);
         if f.segments[s as usize] < 255 {              // stop at first short segment
            break;
         }
         s += 1;
      }
      // either this continues, or it ends it...
      if end_page && s < f.segment_count-1 {
         return error(f, VorbisError::InvalidStream);
      }
      if s == f.segment_count {
         s = -1; // set 'crosses page' flag
      }
      if p > f.stream_end {
        return error(f, VorbisError::NeedMoreData);
      }
      first = false;
   }
   while s == -1 {
      // check that we have the page header ready
      if p.offset(26) >= f.stream_end               {return error(f, VorbisError::NeedMoreData);}
      
      // validate the page
      {
          let p_slice = std::slice::from_raw_parts(p, 4);
          if p_slice != OGG_PAGE_HEADER {return error(f, VorbisError::InvalidStream);}
      }
      if *p.offset(4) != 0                             {return error(f, VorbisError::InvalidStream);}
      if first  { // the first segment must NOT have 'continued_packet', later ones MUST
         if f.previous_length != 0 {
            if (*p.offset(5) & PAGEFLAG_CONTINUED_PACKET) != 0  {
                return error(f, VorbisError::InvalidStream);
            }
         }
         // if no previous length, we're resynching, so we can come in on a continued-packet,
         // which we'll just drop
      } else {
        if (*p.offset(5) & PAGEFLAG_CONTINUED_PACKET) == 0 {
             return error(f, VorbisError::InvalidStream);
        }
      }
      let n = *p.offset(26) as i32; // segment counts
      let q = p.offset(27);  // q points to segment table
      p = q.offset(n as isize); // advance past header
      // make sure we've read the segment table
      if p > f.stream_end                     {return error(f, VorbisError::NeedMoreData);}
      
      s = 0;
      while s < n {
         p = p.offset( *q.offset(s as isize) as isize);
         if *q.offset(s as isize) < 255 {
            break;
         }
          s += 1;
      }
      
      if end_page && s < n-1 {
          return error(f, VorbisError::InvalidStream);
      }
      
      if s == n {
         s = -1; // set 'crosses page' flag
      }
      
      if p > f.stream_end {
          return error(f, VorbisError::NeedMoreData);
      }
      
      first = false;
   }
   
   return true;    
}

const SAMPLE_UNKNOWN : u32 = 0xffffffff;

// these functions return the total length of the vorbis stream
pub fn stb_vorbis_stream_length_in_samples(f: &mut Vorbis) -> u32
{
    use VorbisError::*;
    
    let restore_offset : u32;
    let previous_safe :u32;
    let mut end: u32 = 0;
    let mut last_page_loc :u32;

   if f.push_mode { return error(f, InvalidApiMixing) as u32; }
   if f.total_samples == 0 {
      let mut last : u32 = 0;
      let mut lo : u32;
      let hi : u32;
      let mut header: [u8; 6] = [0; 6];
    
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

      if vorbis_find_page(f, Some(&mut end), Some(&mut last)) == 0 {
         // if we can't find a page, we're hosed!
         f.error = CantFindLastPage;
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
         if vorbis_find_page(f, Some(&mut end), Some(&mut last)) == 0 {
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
      getn(f, &mut header[..]);
      // extract the absolute granule position
      lo = get32(f);
      hi = get32(f);
      if lo == 0xffffffff && hi == 0xffffffff {
         f.error = CantFindLastPage;
         f.total_samples = SAMPLE_UNKNOWN;
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
   return if f.total_samples == SAMPLE_UNKNOWN {0} else {f.total_samples};
}

// implements the search logic for finding a page and starting decoding. if
// the function succeeds, current_loc_valid will be true and current_loc will
// be less than or equal to the provided sample number (the closer the
// better).
fn seek_to_sample_coarse(f: &mut Vorbis, mut sample_number: u32) -> bool
{
   let mut start_seg_with_known_loc : i32;
   let mut end_pos : i32;
   let mut page_start : i32;
   let mut delta: u32;
   let mut offset: f64 = 0.0;
   let mut bytes_per_sample : f64 = 0.0;
   let mut probe = 0; 
   
   use VorbisError::*;

   // find the last page and validate the target sample
   let stream_length = stb_vorbis_stream_length_in_samples(f);
   if stream_length == 0            {return error(f, SeekWithoutLength);}
   if sample_number > stream_length { return error(f, SeekInvalid);}

   'error: loop {
   // this is the maximum difference between the window-center (which is the
   // actual granule position value), and the right-start (which the spec
   // indicates should be the granule position (give or take one)).
   let padding = ((f.blocksize_1 - f.blocksize_0) >> 2) as u32;
   if sample_number < padding {
      sample_number = 0;
   }else{
      sample_number -= padding;
   }
   
   let mut left = f.p_first;
   while left.last_decoded_sample == !0 {
      // (untested) the first page does not have a 'last_decoded_sample'
      set_file_offset(f, left.page_end);
      if get_seek_page_info(f, &mut left) == false {
        //   goto error;
        break 'error;
        }
   }

   let mut right = f.p_last;
   assert!(right.last_decoded_sample != !0 );

   // starting from the start is handled differently
   if sample_number <= left.last_decoded_sample {
      stb_vorbis_seek_start(f);
      return true;
   }

   let mut mid: ProbedPage = ProbedPage::default();
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

         if vorbis_find_page(f, None, None) == 0 {
            break 'error;
        }
      }

      loop {
         if get_seek_page_info(f, &mut mid) == false {
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
   if start_page(f) == false { return error(f, SeekFailed);}
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

      if start_seg_with_known_loc > 0 || (f.page_flag & PAGEFLAG_CONTINUED_PACKET) == 0{
         break;
      }

      // (untested) the final packet begins on an earlier page
      unsafe{
      if go_to_page_before(f, page_start as u32) == false {
        break 'error;
      }
      }

      page_start = stb_vorbis_get_file_offset(f) as i32;
      if start_page(f) == false {
        break 'error;
        }
        
      end_pos = f.segment_count - 1;
   }

   // prepare to start decoding
   f.current_loc_valid = false;
   f.last_seg = false;
   f.valid_bits = 0;
   f.packet_bytes = 0;
   f.bytes_in_seg = 0;
   f.previous_length = 0;
   f.next_seg = start_seg_with_known_loc;

   for i in 0 .. start_seg_with_known_loc as usize {
       let seg = f.segments[i] as i32;
      skip(f, seg);
   }

   // start decoding (optimizable - this frame is generally discarded)
   vorbis_pump_first_frame(f);
   return true;
   } // loop -- 'error
// error:
   // try to restore the file to a valid state
   stb_vorbis_seek_start(f);
   return error(f, SeekFailed);
}

#[allow(unreachable_code, unused_variables)]
unsafe fn vorbis_search_for_page_pushdata(f: &mut Vorbis, data: &[u8]) -> i32
{
   panic!("EXPECTED PANIC: need ogg sample that will trigger this panic");

   // NOTE(bungcip): change to return usize/u32?

    for i in 0 .. f.page_crc_tests as usize {
      f.scan[i].bytes_done = 0;
    } 

   let mut data_len = data.len();

   // if we have room for more scans, search for them first, because
   // they may cause us to stop early if their header is incomplete
   if f.page_crc_tests < STB_PUSHDATA_CRC_COUNT {
      if data_len < 4 {return 0;}
      data_len -= 3; // need to look for 4-byte sequence, so don't miss
                     // one that straddles a boundary
      for i in 0 .. data_len as usize {
         if data[i] == 0x4f {
            let is_ogg_page_header = data[i..].starts_with(&OGG_PAGE_HEADER);
             
            if is_ogg_page_header {
            //    let mut crc : u32;
               // make sure we have the whole page header
               if i+26 >= data_len || i + 27 + data[i+26] as usize >= data_len {
                  // only read up to this page start, so hopefully we'll
                  // have the whole page header start next time
                  data_len = i;
                  break;
               }
               // ok, we have it all; compute the length of the page
               let mut len : i32 = 27 + data[i + 26] as i32;
               for j in 0 .. data[i + 26] as usize {
                  len += data[i + 27 + j] as i32;
               }
               // scan everything up to the embedded crc (which we must 0)
               let mut crc = 0;
               for j in 0 .. 22 as usize {
                  crc = crc32_update(crc, data[i + j]);
               }
               // now process 4 0-bytes
               for _ in 0 .. 4 {
                  crc = crc32_update(crc, 0);
               }
               let j = 26;
               // len is the total number of bytes we need to scan
               let n = f.page_crc_tests as usize;
               f.page_crc_tests += 1;
               f.scan[n].bytes_left = len-j;
               f.scan[n].crc_so_far = crc;
               f.scan[n].goal_crc = data[i + 22] as u32
                    + ((data[i + 23] as u32) << 8)
                    + ((data[i + 24] as u32) <<16)
                    + ((data[i + 25] as u32) <<24);
               // if the last frame on a page is continued to the next, then
               // we can't recover the sample_loc immediately
               if data[i + 27 + data[i + 26] as usize - 1] == 255 {
                  f.scan[n].sample_loc = !0;
               }else{
                  f.scan[n].sample_loc = data[i + 6] as u32
                    + ((data[i + 7] as u32) <<  8)
                    + ((data[i + 8] as u32) << 16)
                    + ((data[i + 9] as u32) << 24);
               }
               f.scan[n].bytes_done = (i+j as usize) as i32;
               if f.page_crc_tests == STB_PUSHDATA_CRC_COUNT {
                  break;
               }
               // keep going if we still have room for more
            }
         }
      }
   }

   let mut i : usize = 0;
   while i < f.page_crc_tests as usize {
      let mut crc : u32;
      let n = f.scan[i].bytes_done as usize;
      let m = std::cmp::min(f.scan[i].bytes_left as usize, data_len - n);
      // m is the bytes to scan in the current chunk
      crc = f.scan[i].crc_so_far;
      for j in 0 .. m {
         crc = crc32_update(crc, data[n + j]);
      }
      f.scan[i].bytes_left -= m as i32;
      f.scan[i].crc_so_far = crc;
      if f.scan[i].bytes_left == 0 {
         // does it match? 
         if f.scan[i].crc_so_far == f.scan[i].goal_crc {
            // Houston, we have page
            data_len = n+m; // consumption amount is wherever that scan ended
            f.page_crc_tests = -1; // drop out of page scan mode
            f.previous_length = 0; // decode-but-don't-output one frame
            f.next_seg = -1;       // start a new page
            f.current_loc = f.scan[i].sample_loc; // set the current sample location
                                    // to the amount we'd have decoded had we decoded this page
            f.current_loc_valid = true; 
            f.current_loc != !0;
            return data_len as i32;
         }
         // delete entry
         f.page_crc_tests -= 1;
         f.scan[i] = f.scan[f.page_crc_tests as usize];
      } else {
         i += 1;
      }
   }

   return data_len as i32;
}

// if the fast table above doesn't work, we want to binary
// search them... need to reverse the bits

unsafe fn compute_sorted_huffman(c: &mut Codebook, lengths: &mut [u8], values: &[u32])
{
   // build a list of all the entries
   // OPTIMIZATION: don't include the short ones, since they'll be caught by FAST_HUFFMAN.
   // this is kind of a frivolous optimization--I don't see any performance improvement,
   // but it's like 4 extra lines of code, so.
   if c.sparse == false {
      let mut k = 0;
      for i in 0 .. c.entries {
         if include_in_sort(c, lengths[i as usize]) == true {
            c.sorted_codewords[k as usize] = bit_reverse(
                c.codewords[i as usize]);
            k += 1;
         }
      }
      assert!(k == c.sorted_entries);
   } else {
      for i in 0 .. c.sorted_entries {
         c.sorted_codewords[i as usize] = bit_reverse(
                c.codewords[i as usize]);
      }
   }

   // NOTE(bungcip): sorted_codewords length is c.sorted_entries + 1
   //                we only need to sort except last element
   c.sorted_codewords[0 .. c.sorted_entries as usize].sort();
   c.sorted_codewords[c.sorted_entries as usize] = 0xffffffff;

   let len = if c.sparse == true  { c.sorted_entries } else { c.entries };
   // now we need to indicate how they correspond; we could either
   //   #1: sort a different data structure that says who they correspond to
   //   #2: for each sorted entry, search the original list to find who corresponds
   //   #3: for each original entry, find the sorted entry
   // #1 requires extra storage, #2 is slow, #3 can use binary search!
   for i in 0 .. len {
      let huff_len = if c.sparse == true {
          lengths[values[i as usize] as usize]
      } else {
          lengths[i as usize]
      };

      if include_in_sort(c,huff_len) == true {
         let code: u32 = bit_reverse(c.codewords[i as usize]);
         let mut x : i32 = 0;
         let mut n : i32 = c.sorted_entries;
         while n > 1 {
            // invariant: sc[x] <= code < sc[x+n]
            let m : i32 = x + (n >> 1);
            if c.sorted_codewords[m as usize] <= code {
               x = m;
               n -= n >> 1;
            } else {
               n >>= 1;
            }
         }
         assert!(c.sorted_codewords[x as usize] == code);
         if c.sparse == true {
            c.sorted_values[x as usize] = values[i as usize] as i32;
            c.codeword_lengths[x as usize] = huff_len;
         } else {
            c.sorted_values[x as usize] = i;
         }
      }

   }
}

// NOTE(bungcip): reduce parameter count?
unsafe fn vorbis_decode_packet_rest(f: &mut Vorbis, len: &mut i32, m: &Mode, 
    mut left_start: i32, _: i32, right_start: i32, right_end: i32, p_left: &mut i32) -> bool
{
// WINDOWING

    let n = f.blocksize[m.blockflag as usize] as i32;
    let map: &Mapping = FORCE_BORROW!( &f.mapping[ m.mapping as usize ] );

// FLOORS
   let n2 : i32 = n >> 1;

   CHECK!(f);

   use VorbisError::*;

    let mut zero_channel: [bool; 256] = [false; 256];
    let mut really_zero_channel : [bool; 256] = [false; 256];

   for i in 0 .. f.channels as usize {
      let s: i32 = map.chan[i].mux as i32;
      zero_channel[i] = false;
      let floor : i32 = map.submap_floor[s as usize] as i32;
      if f.floor_types[floor as usize] == 0 {
         return error(f, InvalidStream);
      } else {
          let g : &Floor1 = FORCE_BORROW!( &f.floor_config[floor as usize].floor1);
         if get_bits(f, 1) != 0 {
            static RANGE_LIST: [i32; 4] = [ 256, 128, 86, 64 ];
            let range = RANGE_LIST[ (g.floor1_multiplier-1) as usize];
            let mut offset = 2;
            let mut final_y : &mut [YTYPE] = FORCE_BORROW_MUT!( f.final_y[i].as_mut_slice());
            final_y[0] = get_bits(f, ilog(range)-1) as i16;
            final_y[1] = get_bits(f, ilog(range)-1) as i16;
            for j in 0 .. g.partitions as usize {
               let pclass = g.partition_class_list[j] as usize;
               let cdim = g.class_dimensions[pclass];
               let cbits = g.class_subclasses[pclass];
               let csub = (1 << cbits)-1;
               let mut cval = 0;
               if cbits != 0 {
                  let c: &mut Codebook = FORCE_BORROW_MUT!( &mut f.codebooks[ g.class_masterbooks[pclass] as usize]);
                  cval = decode_raw(f, c);
               }
               for _ in 0 .. cdim {
                  let book = g.subclass_books[pclass][ (cval & csub) as usize];
                  cval = cval >> cbits;
                  if book >= 0 {
                     let c: &mut Codebook = FORCE_BORROW_MUT!( &mut f.codebooks[book as usize] );
                     let temp : i32 = decode_raw(f, c);
                     final_y[offset] = temp as i16;
                  } else {
                     final_y[offset] = 0;
                  }
                    offset += 1;
               }
            }

            if f.valid_bits == INVALID_BITS {
                // goto error;
                zero_channel[i as usize] = true;
                continue;
            } // behavior according to spec
            
            let mut step2_flag: [u8; 256] = [0; 256];
            step2_flag[0] = 1; 
            step2_flag[1] = 1;
            for j in 2 .. g.values as usize {
               let low = g.neighbors[j][0] as usize;
               let high = g.neighbors[j][1] as usize;
               let pred = predict_point(
                   g.xlist[j] as i32, 
                   g.xlist[low] as i32,
                   g.xlist[high] as i32, 
                   final_y[low] as i32, 
                   final_y[high] as i32
               );
               let val = final_y[j];
               let highroom = range - pred;
               let lowroom = pred;
               let room;
               if highroom < lowroom {
                  room = highroom * 2;
               }else{
                  room = lowroom * 2;
               }
               if val != 0 {
                  step2_flag[low] = 1;
                  step2_flag[high] = 1;
                  step2_flag[j] = 1;
                  
                  if val >= room as i16 {
                     if highroom > lowroom {
                        final_y[j] = (val - lowroom as i16 + pred as i16) as i16;
                     } else {
                        final_y[j] = (pred as i16 - val + highroom as i16 - 1) as i16;
                     }
                  } else {
                     if (val & 1) != 0 {
                        final_y[j] = pred as i16 - ((val+1)>>1);
                     } else {
                        final_y[j] = pred as i16+ (val>>1);
                     }
                  }
               } else {
                  step2_flag[j] = 0;
                  final_y[j] = pred as i16;
               }
            }

            // defer final floor computation until _after_ residue
            for j in 0 .. g.values as usize {
               if step2_flag[j] == 0 {
                  final_y[j] = -1;
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

   // re-enable coupled channels if necessary
   CHECK!(f);
   std::ptr::copy_nonoverlapping(zero_channel.as_ptr(), really_zero_channel.as_mut_ptr(), f.channels as usize);

   for i in 0 .. map.coupling_steps as usize {
      let magnitude = map.chan[i].magnitude as usize;
      let angle = map.chan[i].angle as usize;
      
      if zero_channel[magnitude] == false || zero_channel[angle] == false {
         zero_channel[magnitude] = false;
         zero_channel[angle] = false;
      }
   }

   CHECK!(f);
// RESIDUE DECODE
   for i in 0 .. map.submaps as usize {
      let mut residue_buffers: AudioBufferSlice<f32> = AudioBufferSlice::new(0);
      let mut do_not_decode: Vec<bool> = Vec::with_capacity(16);
      for j in 0 .. f.channels as usize {
         if map.chan[j].mux as usize == i {
            if zero_channel[j] {
               do_not_decode.push(true);
               residue_buffers.push_channel(&mut []);
            } else {
               do_not_decode.push(false);
               residue_buffers.push_channel(&mut f.channel_buffers[j]);
            }
         }
      }
      let r = map.submap_residue[i];
      decode_residue(f, &mut residue_buffers, n2, r as i32, &do_not_decode);
   }

   CHECK!(f);

// INVERSE COUPLING
   let mut i : i32 = map.coupling_steps as i32 - 1; 
   while i >= 0 {
      let n2 = n >> 1;
      let ref c = map.chan[i as usize];
      let m : *mut f32 = f.channel_buffers[c.magnitude as usize].as_mut_ptr();
      let a : *mut f32 = f.channel_buffers[c.angle  as usize].as_mut_ptr();
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
   for i in 0 .. f.channels as usize {
      if really_zero_channel[i] {
          std::ptr::write_bytes(f.channel_buffers[i].as_mut_ptr(), 0, n2 as usize);
      } else {
          let cb : &mut [f32] = FORCE_BORROW!( f.channel_buffers[i].as_mut_slice() );
          let fy : &[YTYPE] = FORCE_BORROW!( f.final_y[i].as_slice() ); 
          do_floor(f, map, i, n as usize, cb, fy);
      }
   }

// INVERSE MDCT
   CHECK!(f);
   for i in 0 .. f.channels as usize {
      let cb : &mut Vec<f32> = FORCE_BORROW_MUT!(&mut f.channel_buffers[i]);
      inverse_mdct(cb, n, f, m.blockflag as i32);
   }
   CHECK!(f);

   // this shouldn't be necessary, unless we exited on an error
   // and want to flush to get to the next packet
   flush_packet(f);

   if f.first_decode == true {
      // assume we start so first non-discarded sample is sample 0
      // this isn't to spec, but spec would require us to read ahead
      // and decode the size of all current frames--could be done,
      // but presumably it's not a commonly used feature
      // NOTE(bungcip): maybe this is bug?
      f.current_loc = -n2 as u32; // start of first frame is positioned for discard
      // we might have to discard samples "from" the next frame too,
      // if we're lapping a large block then a small at the start?
      f.discard_samples_deferred = n - right_end;
      f.current_loc_valid = true;
      f.first_decode = false; 
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
   } else if f.previous_length == 0 && f.current_loc_valid == true {
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
      if f.current_loc_valid == true && (f.page_flag & PAGEFLAG_LAST_PAGE) != 0 {
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
            return true;
         }
      }
      // otherwise, just set our sample loc
      // guess that the ogg granule pos refers to the _middle_ of the
      // last frame?
      // set f.current_loc to the position of left_start
      f.current_loc = f.known_loc_for_packet - (n2-left_start) as u32;
      f.current_loc_valid = true;
   }

   if f.current_loc_valid == true {
       let temp_1 = (right_start - left_start) as u32;
      // NOTE(bungcip): maybe this is bug?
      f.current_loc = f.current_loc.wrapping_add(temp_1);
   }

   *len = right_end;  // ignore samples after the window goes to 0
   CHECK!(f);

   return true;
}


unsafe fn decode_residue(f: &mut Vorbis, residue_buffers: &mut AudioBufferSlice<f32>, n: i32, rn: i32, do_not_decode: &[bool])
{
   let ch = residue_buffers.channel_count() as i32;
   let r: &Residue = FORCE_BORROW!( &f.residue_config[rn as usize] );
   let rtype : i32 = f.residue_types[rn as usize] as i32;
   let c : i32 = r.classbook as i32;
   let classwords = f.codebooks[c as usize].dimensions as usize;
   let n_read : i32 = (r.end - r.begin) as i32;
   let part_read : i32 = n_read / r.part_size as i32;
   
   // NOTE(bungcip): optimize?
   let mut part_classdata = Vec::with_capacity(f.channels as usize);
   for _ in 0 .. f.channels {
       let mut temp_1: Vec<Vec<u8>> = Vec::with_capacity(part_read as usize);
       temp_1.resize(part_read as usize, Vec::new());
       part_classdata.push( temp_1 );
   }

   CHECK!(f);

   for i in 0 .. residue_buffers.channel_count() {
      if do_not_decode[i] == false {
          std::ptr::write_bytes(residue_buffers[i].as_mut_ptr(), 0, n as usize);
      }
   }
   
   // note(bungcip): simulate goto
   'done: loop {

   if rtype == 2 && residue_buffers.channel_count() != 1 {
       let mut j = 0;
       while j < residue_buffers.channel_count() {
         if do_not_decode[j as usize] == false {
            break;
         }
         j += 1;
       }
       
      if j == residue_buffers.channel_count() {
        //  goto done;
        break 'done;
      }

      for pass in 0 .. 8 {
         let mut pcount = 0;
         let mut class_set = 0;
         if residue_buffers.channel_count() == 2 {
            while pcount < part_read {
               let z : i32 = r.begin as i32 + (pcount*r.part_size as i32);
               let mut c_inter : i32 = z & 1;
               let mut p_inter : i32 = z>>1;
               if pass == 0 {
                  let c: &Codebook = FORCE_BORROW!(&f.codebooks[r.classbook as usize]);
                  let q = decode_raw(f,c);
                  if q == EOP {
                    // goto done;
                    break 'done;  
                  } 
                  
                  // NOTE(bungcip): remove .clone() !!!
                  part_classdata[0][class_set] = r.classdata[q as usize].clone();
               }
               
               let mut i = 0;
               while i < classwords && pcount < part_read {
                  let mut z : i32 = r.begin as i32 + (pcount*r.part_size as i32);
                  let c : i32 = part_classdata[0][class_set][i] as i32;
                  let b : i32 = r.residue_books[c as usize][pass as usize] as i32;
                  if b >= 0 {
                    let book : &Codebook = FORCE_BORROW!( &f.codebooks[b as usize] );
                     // saves 1%
                     if codebook_decode_deinterleave_repeat(f, book, residue_buffers, &mut c_inter, &mut p_inter, n, r.part_size as i32) == false {
                        // goto done;
                        break 'done;
                     }
                  } else {
                     z += r.part_size as i32;
                     c_inter = z & 1;
                     p_inter = z >> 1;
                  }
                    i += 1; pcount += 1;
               }
               class_set += 1;
            }
         } else if residue_buffers.channel_count() == 1 {
            while pcount < part_read {
               let z : i32 = r.begin as i32 + pcount as i32 * r.part_size as i32;
               let mut c_inter : i32 = 0;
               let mut p_inter : i32 = z as i32;
               if pass == 0 {
                  let c : &Codebook = FORCE_BORROW!( &f.codebooks[r.classbook as usize] );
                  let q = decode_raw(f,c);
                  if q == EOP{
                    // goto done;
                    break 'done; 
                  } 
                  // NOTE: remove .clone() !!!
                  part_classdata[0][class_set as usize] = r.classdata[q as usize].clone();
               }
               let mut i = 0;
               while i < classwords && pcount < part_read {
                  let mut z : i32 = r.begin as i32 + pcount*r.part_size as i32;
                  let c : i32 = part_classdata[0][class_set as usize][i as usize] as i32;
                  let b : i32 = r.residue_books[c as usize][pass as usize] as i32;
                  if b >= 0 {
                     let book : &Codebook = FORCE_BORROW!( &f.codebooks[b as usize] );
                     if codebook_decode_deinterleave_repeat(f, book, residue_buffers, &mut c_inter, &mut p_inter, n, r.part_size as i32) == false {
                        // goto done;
                        break 'done;
                     }
                  } else {
                     z += r.part_size as i32;
                     c_inter = 0;
                     p_inter = z;
                  }
                    i += 1; pcount += 1;
               }
               class_set += 1;
            }
         } else {
            while pcount < part_read {
               let z : i32 = r.begin as i32 + pcount as i32 * r.part_size as i32;
               let mut c_inter : i32 = z % ch;
               let mut p_inter : i32 = z / ch;
               if pass == 0 {
                  let c : &Codebook = FORCE_BORROW!( &f.codebooks[r.classbook as usize] );
                  let q = decode_raw(f,c);
                  if q == EOP{
                    // goto done;
                    break 'done;  
                  } 
                  
                  // NOTE(bungcip): remove .clone() !!
                  part_classdata[0][class_set as usize] = r.classdata[q as usize].clone();
               }
               let mut i = 0;
               while i < classwords && pcount < part_read {
                  let mut z : i32 = r.begin as i32 + pcount as i32 * r.part_size as i32;
                  let c : i32 = part_classdata[0][class_set as usize][i as usize] as i32;
                  let b : i32 = r.residue_books[c as usize][pass as usize] as i32;
                  if b >= 0 {
                      let book : &Codebook = FORCE_BORROW!( &f.codebooks[b as usize] );
                     if codebook_decode_deinterleave_repeat(f, book, residue_buffers, &mut c_inter, &mut p_inter, n, r.part_size as i32) == false {
                        // goto done;
                        break 'done;
                     }
                  } else {
                     z += r.part_size as i32;
                     c_inter = z % ch;
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
      let mut pcount = 0;
      let mut class_set = 0;
      while pcount < part_read {
         if pass == 0 {
            for j in 0 .. residue_buffers.channel_count() as usize {
               if do_not_decode[j] == false {
                  let c : &Codebook = FORCE_BORROW!( &f.codebooks[r.classbook as usize]);
                  let temp = decode_raw(f,c);
                  if temp == EOP {
                    //   goto done;
                    break 'done;
                  }
                  
                  // NOTE(bungcip): remove .clone() !!!
                  part_classdata[j][class_set as usize] = r.classdata[temp as usize].clone();
               }
            }
         }
            let mut i = 0;
            while i < classwords && pcount < part_read {
            for j in 0 .. residue_buffers.channel_count() {
               if do_not_decode[j] == false {
                  let c : i32 = part_classdata[j][class_set as usize][i as usize] as i32;
                  let b : i32 = r.residue_books[c as usize][pass as usize] as i32;
                  if b >= 0 {
                      let mut target = &mut residue_buffers[j];
                      let offset : i32 =  r.begin as i32 + pcount*r.part_size as i32;
                      let n : i32 = r.part_size as i32;
                      let book : &Codebook = FORCE_BORROW!( &f.codebooks[b as usize] );
                      if residue_decode(f, book, &mut target, offset, n, rtype) == false {
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
}

// the following were split out into separate functions while optimizing;
// they could be pushed back up but eh. __forceinline showed no change;
// they're probably already being inlined.

unsafe fn imdct_step3_iter0_loop(n: i32, e: *mut f32, i_off: i32, k_off: i32 , mut a: *mut f32)
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
      *ee2.offset(-0) = k00_20 * *a.offset(0) - k01_21 * *a.offset(1);
      *ee2.offset(-1) = k01_21 * *a.offset(0) + k00_20 * *a.offset(1);
      a = a.offset(8);

      k00_20  = *ee0.offset(-2) - *ee2.offset(-2);
      k01_21  = *ee0.offset(-3) - *ee2.offset(-3);
      *ee0.offset(-2) += *ee2.offset(-2);
      *ee0.offset(-3) += *ee2.offset(-3);
      *ee2.offset(-2) = k00_20 * *a.offset(0) - k01_21 * *a.offset(1);
      *ee2.offset(-3) = k01_21 * *a.offset(0) + k00_20 * *a.offset(1);
      a = a.offset(8);

      k00_20  = *ee0.offset(-4) - *ee2.offset(-4);
      k01_21  = *ee0.offset(-5) - *ee2.offset(-5);
      *ee0.offset(-4) += *ee2.offset(-4);
      *ee0.offset(-5) += *ee2.offset(-5);
      *ee2.offset(-4) = k00_20 * *a.offset(0) - k01_21 * *a.offset(1);
      *ee2.offset(-5) = k01_21 * *a.offset(0) + k00_20 * *a.offset(1);
      a = a.offset(8);

      k00_20  = *ee0.offset(-6) - *ee2.offset(-6);
      k01_21  = *ee0.offset(-7) - *ee2.offset(-7);
      *ee0.offset(-6) += *ee2.offset(-6);
      *ee0.offset(-7) += *ee2.offset(-7);
      *ee2.offset(-6) = k00_20 * *a.offset(0) - k01_21 * *a.offset(1);
      *ee2.offset(-7) = k01_21 * *a.offset(0) + k00_20 * *a.offset(1);
      a = a.offset(8);
      ee0 = ee0.offset(-8);
      ee2 = ee2.offset(-8);

        i -= 1;
   }
}


unsafe fn imdct_step3_inner_r_loop(lim: i32, e: *mut f32, d0: i32 , k_off: i32 , mut a: *mut f32, k1: i32)
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
      *e2.offset(-0) = (k00_20)**a.offset(0) - (k01_21) * *a.offset(1);
      *e2.offset(-1) = (k01_21)**a.offset(0) + (k00_20) * *a.offset(1);

      a = a.offset(k1 as isize);

      k00_20 = *e0.offset(-2) - *e2.offset(-2);
      k01_21 = *e0.offset(-3) - *e2.offset(-3);
      *e0.offset(-2) += *e2.offset(-2);
      *e0.offset(-3) += *e2.offset(-3);
      *e2.offset(-2) = (k00_20)**a.offset(0) - (k01_21) * *a.offset(1);
      *e2.offset(-3) = (k01_21)**a.offset(0) + (k00_20) * *a.offset(1);

      a = a.offset(k1 as isize);

      k00_20 = *e0.offset(-4) - *e2.offset(-4);
      k01_21 = *e0.offset(-5) - *e2.offset(-5);
      *e0.offset(-4) += *e2.offset(-4);
      *e0.offset(-5) += *e2.offset(-5);
      *e2.offset(-4) = (k00_20)**a.offset(0) - (k01_21) * *a.offset(1);
      *e2.offset(-5) = (k01_21)**a.offset(0) + (k00_20) * *a.offset(1);

      a = a.offset(k1 as isize);

      k00_20 = *e0.offset(-6) - *e2.offset(-6);
      k01_21 = *e0.offset(-7) - *e2.offset(-7);
      *e0.offset(-6) += *e2.offset(-6);
      *e0.offset(-7) += *e2.offset(-7);
      *e2.offset(-6) = (k00_20)**a.offset(0) - (k01_21) * *a.offset(1);
      *e2.offset(-7) = (k01_21)**a.offset(0) + (k00_20) * *a.offset(1);

      e0 = e0.offset(-8);
      e2 = e2.offset(-8);

      a = a.offset(k1 as isize);
    
        i -= 1;
   }
}


unsafe fn imdct_step3_inner_s_loop(n: i32, e: *mut f32, i_off: i32, k_off: i32, a: *mut f32, a_off: i32 , k0: i32)
{
   let mut i : i32;
   let a_off = a_off as isize;
   
   let a0 = *a.offset(0);
   let a1 = *a.offset(1);
   let a2 = *a.offset(a_off);
   let a3 = *a.offset(a_off+1);
   let a4 = *a.offset(a_off*2);
   let a5 = *a.offset(a_off*2+1);
   let a6 = *a.offset(a_off*3);
   let a7 = *a.offset(a_off*3+1);

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
      *ee2.offset(0) = (k00) * a0 - (k11) * a1;
      *ee2.offset(-1) = (k11) * a0 + (k00) * a1;

      k00     = *ee0.offset(-2) - *ee2.offset(-2);
      k11     = *ee0.offset(-3) - *ee2.offset(-3);
      *ee0.offset(-2) =  *ee0.offset(-2) + *ee2.offset(-2);
      *ee0.offset(-3) =  *ee0.offset(-3) + *ee2.offset(-3);
      *ee2.offset(-2) = (k00) * a2 - (k11) * a3;
      *ee2.offset(-3) = (k11) * a2 + (k00) * a3;

      k00     = *ee0.offset(-4) - *ee2.offset(-4);
      k11     = *ee0.offset(-5) - *ee2.offset(-5);
      *ee0.offset(-4) =  *ee0.offset(-4) + *ee2.offset(-4);
      *ee0.offset(-5) =  *ee0.offset(-5) + *ee2.offset(-5);
      *ee2.offset(-4) = (k00) * a4 - (k11) * a5;
      *ee2.offset(-5) = (k11) * a4 + (k00) * a5;

      k00     = *ee0.offset(-6) - *ee2.offset(-6);
      k11     = *ee0.offset(-7) - *ee2.offset(-7);
      *ee0.offset(-6) =  *ee0.offset(-6) + *ee2.offset(-6);
      *ee0.offset(-7) =  *ee0.offset(-7) + *ee2.offset(-7);
      *ee2.offset(-6) = (k00) * a6 - (k11) * a7;
      *ee2.offset(-7) = (k11) * a6 + (k00) * a7;

      ee0 = ee0.offset(-k0 as isize);
      ee2 = ee2.offset(-k0 as isize);

        i -= 1;
   }
}


unsafe fn imdct_step3_inner_s_loop_ld654(n: i32, e: *mut f32, i_off: i32, a: *mut f32, base_n: i32)
{
   let a_off = base_n >> 3;
   let a2 = *a.offset( a_off as isize);
   let mut z = e.offset(i_off as isize);
   let base = z.offset(- (16 * n) as isize);

   while z > base {
      let k00   = *z.offset(-0) - *z.offset(-8);
      let k11   = *z.offset(-1) - *z.offset(-9);
      *z.offset(-0) = *z.offset(-0) + *z.offset(-8);
      *z.offset(-1) = *z.offset(-1) + *z.offset(-9);
      *z.offset(-8) =  k00;
      *z.offset(-9) =  k11 ;

      let k00    = *z.offset(-2) - *z.offset(-10);
      let k11    = *z.offset(-3) - *z.offset(-11);
      *z.offset(-2) = *z.offset(-2) + *z.offset(-10);
      *z.offset(-3) = *z.offset(-3) + *z.offset(-11);
      *z.offset(-10) = (k00+k11) * a2;
      *z.offset(-11) = (k11-k00) * a2;

      let k00    = *z.offset(-12) - *z.offset(-4);  // reverse to avoid a unary negation
      let k11    = *z.offset(-5) - *z.offset(-13);
      *z.offset(-4) = *z.offset(-4) + *z.offset(-12);
      *z.offset(-5) = *z.offset(-5) + *z.offset(-13);
      *z.offset(-12) = k11;
      *z.offset(-13) = k00;

      let k00    = *z.offset(-14) - *z.offset(-6);  // reverse to avoid a unary negation
      let k11    = *z.offset(-7) - *z.offset(-15);
      *z.offset(-6) = *z.offset(-6) + *z.offset(-14);
      *z.offset(-7) = *z.offset(-7) + *z.offset(-15);
      *z.offset(-14) = (k00+k11) * a2;
      *z.offset(-15) = (k00-k11) * a2;

      iter_54(z);
      iter_54(z.offset(-8));
      z = z.offset(-16);
   }
}

#[inline(always)]
unsafe fn iter_54(z: *mut f32)
{
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



unsafe fn inverse_mdct(buffer: &mut [f32], n: i32, f: &mut Vorbis, blocktype: i32)
{
   let n2 : i32 = n >> 1;
   let n4 : i32 = n >> 2; 
   let n8 : i32 = n >> 3;

   let buffer = buffer.as_mut_ptr();

   // @OPTIMIZE: reduce register pressure by using fewer variables?
   
   // NOTE(bungcip): need resize() ?
   let mut buf2 : Vec<f32> = Vec::with_capacity(n2 as usize);
   buf2.resize(n2 as usize, 0.0);
   
//    twiddle factors
   let a: *mut f32 = f.a[blocktype as usize].as_mut_ptr();

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
      let mut d  = buf2.as_mut_ptr().offset( (n2-2) as isize);
      let mut aa = a;

      let e_stop = buffer.offset(n2 as isize);
      let mut e = buffer.offset(0);
      while e != e_stop {
         *d.offset(1) = *e.offset(0) * *aa.offset(0) - *e.offset(2) * *aa.offset(1);
         *d.offset(0) = *e.offset(0) * *aa.offset(1) + *e.offset(2) * *aa.offset(0);
         d = d.offset(-2);
         aa = aa.offset(2);
         e = e.offset(4);
      }

      let mut e = buffer.offset( (n2-3) as isize);
      while d >= buf2.as_mut_ptr() {
         *d.offset(1) = -*e.offset(2) * *aa.offset(0) - -*e.offset(0) * *aa.offset(1);
         *d.offset(0) = -*e.offset(2) * *aa.offset(1) + -*e.offset(0) * *aa.offset(0);
         d = d.offset(-2);
         aa = aa.offset(2);
         e = e.offset(-4);
      }
   }

   // now we use symbolic names for these, so that we can
   // possibly swap their meaning as we change which operations
   // are in place

   let u = buffer;
   let v = buf2.as_mut_ptr();

   // step 2    (paper output is w, now u)
   // this could be in place, but the data ends up in the wrong
   // place... _somebody_'s got to swap it, so this is nominated
   {
      let mut aa = a.offset( (n2-8) as isize);

      let mut e0 = v.offset(n4 as isize);
      let mut e1 = v.offset(0);

      let mut d0 = u.offset(n4 as isize);
      let mut d1 = u.offset(0);

      while aa >= a {
         {
            let v41_21 = *e0.offset(1) - *e1.offset(1);
            let v40_20 = *e0.offset(0) - *e1.offset(0);
            *d0.offset(1)  = *e0.offset(1) + *e1.offset(1);
            *d0.offset(0)  = *e0.offset(0) + *e1.offset(0);
            *d1.offset(1)  = v41_21 * *aa.offset(4) - v40_20 * *aa.offset(5);
            *d1.offset(0)  = v40_20 * *aa.offset(4) + v41_21 * *aa.offset(5);
         }

         {
            let v41_21 = *e0.offset(3) - *e1.offset(3);
            let v40_20 = *e0.offset(2) - *e1.offset(2);
            *d0.offset(3)  = *e0.offset(3) + *e1.offset(3);
            *d0.offset(2)  = *e0.offset(2) + *e1.offset(2);
            *d1.offset(3)  = v41_21 * *aa.offset(0) - v40_20 * *aa.offset(1);
            *d1.offset(2)  = v40_20 * *aa.offset(0) + v41_21 * *aa.offset(1);
         }

         aa = aa.offset(-8);

         d0 = d0.offset(4);
         d1 = d1.offset(4);
         e0 = e0.offset(4);
         e1 = e1.offset(4);
      }
   }

   // step 3
   let ld: i32 = ilog(n) - 1; // ilog is off-by-one from normal definitions

   // optimized step 3:

   // the original step3 loop can be nested r inside s or s inside r;
   // it's written originally as s inside r, but this is dumb when r
   // iterates many times, and s few. So I have two copies of it and
   // switch between them halfway.

   // this is iteration 0 of step 3
   imdct_step3_iter0_loop(n >> 4, u, n2-1-n4*0, -(n >> 3), a);
   imdct_step3_iter0_loop(n >> 4, u, n2-1-n4, -(n >> 3), a);

   // this is iteration 1 of step 3
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*0, -(n >> 4), a, 16);
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8, -(n >> 4), a, 16);
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*2, -(n >> 4), a, 16);
   imdct_step3_inner_r_loop(n >> 5, u, n2-1 - n8*3, -(n >> 4), a, 16);

   let mut l : i32 = 2;
   while l < (ld-3)>>1 {
      let k0   = n  >> (l+2);
      let k0_2 = k0 >> 1;
      let lim  = 1  << (l+1);
      for i in 0 .. lim {
         imdct_step3_inner_r_loop(n >> (l+4), u, n2-1 - k0*i, -k0_2, a, 1 << (l+3));
      }
      l += 1;
   }

   while l < ld-6 {
      let k0 = n >> (l+2);
      let k1 = 1 << (l+3);
      let k0_2 = k0>>1;
      let rlim = n >> (l+6);
      let lim : i32 = 1 << (l+1);
      let mut a0 : *mut f32 = a;
      let mut i_off : i32 = n2-1;
      let mut r : i32 = rlim;
      while r > 0 {
         imdct_step3_inner_s_loop(lim, u, i_off, -k0_2, a0, k1, k0);
         a0 = a0.offset( (k1*4) as isize);
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
   imdct_step3_inner_s_loop_ld654(n >> 5, u, n2-1, a, n);

   // output is u

   // step 4, 5, and 6
   // cannot be in-place because of step 5
   {
      // weirdly, I'd have thought reading sequentially and writing
      // erratically would have been better than vice-versa, but in
      // fact that's not what my testing showed. (That is, with
      // j = bitreverse(i), do you read i and write j, or read j and write i.)

      let mut d0 : *mut f32 = v.offset( (n4-4) as isize);
      let mut d1 : *mut f32 = v.offset( (n2-4) as isize);
      let mut bitrev = f.bit_reverse[blocktype as usize].iter();

      while d0 >= v {
         let k4 = *bitrev.next().unwrap();
         *d1.offset(3) = *u.offset((k4) as isize);
         *d1.offset(2) = *u.offset((k4+1) as isize);
         *d0.offset(3) = *u.offset((k4+2) as isize);
         *d0.offset(2) = *u.offset((k4+3) as isize);

         let k4 = *bitrev.next().unwrap();
         *d1.offset(1) = *u.offset((k4) as isize);
         *d1.offset(0) = *u.offset((k4+1) as isize);
         *d0.offset(1) = *u.offset((k4+2) as isize);
         *d0.offset(0) = *u.offset((k4+3) as isize);
         
         d0 = d0.offset(-4);
         d1 = d1.offset(-4);
        //  bitrev = bitrev[2..];
      }
   }
   // (paper output is u, now v)


   // data must be in buf2
   assert!(v == buf2.as_mut_ptr());

   // step 7   (paper output is v, now v)
   // this is now in place
   {
      let mut c = f.c[blocktype as usize].as_mut_ptr();
      let mut d = v;
      let mut e = v.offset( (n2 - 4) as isize );

      while d < e {
         let mut a02 = *d.offset(0) - *e.offset(2);
         let mut a11 = *d.offset(1) + *e.offset(3);

         let mut b0 = *c.offset(1) * a02 + *c.offset(0)*a11;
         let mut b1 = *c.offset(1) * a11 - *c.offset(0)*a02;

         let mut b2 = *d.offset(0) + *e.offset( 2);
         let mut b3 = *d.offset(1) - *e.offset( 3);

         *d.offset(0) = b2 + b0;
         *d.offset(1) = b3 + b1;
         *e.offset(2) = b2 - b0;
         *e.offset(3) = b1 - b3;

         a02 = *d.offset(2) - *e.offset(0);
         a11 = *d.offset(3) + *e.offset(1);

         b0 = *c.offset(3)*a02 + *c.offset(2)*a11;
         b1 = *c.offset(3)*a11 - *c.offset(2)*a02;

         b2 = *d.offset(2) + *e.offset( 0);
         b3 = *d.offset(3) - *e.offset( 1);

         *d.offset(2) = b2 + b0;
         *d.offset(3) = b3 + b1;
         *e.offset(0) = b2 - b0;
         *e.offset(1) = b1 - b3;

         c = c.offset(4);
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
      let mut b = f.b[blocktype as usize].as_mut_ptr().offset( (n2 - 8) as isize);
      let mut e = buf2.as_mut_ptr().offset( (n2 - 8) as isize );
      let mut d0 = buffer.offset(0);
      let mut d1 = buffer.offset( (n2-4) as isize);
      let mut d2 = buffer.offset( n2 as isize);
      let mut d3 = buffer.offset( (n-4) as isize);
      while e >= v {
         let mut p3 =  *e.offset(6)* *b.offset(7) - *e.offset(7) * *b.offset(6);
         let mut p2 = -*e.offset(6)* *b.offset(6) - *e.offset(7) * *b.offset(7); 

         *d0.offset(0) =   p3;
         *d1.offset(3) = - p3;
         *d2.offset(0) =   p2;
         *d3.offset(3) =   p2;

         let mut p1 =  *e.offset(4)**b.offset(5) - *e.offset(5)**b.offset(4);
         let mut p0 = -*e.offset(4)**b.offset(4) - *e.offset(5)**b.offset(5); 

         *d0.offset(1) =   p1;
         *d1.offset(2) = - p1;
         *d2.offset(1) =   p0;
         *d3.offset(2) =   p0;

         p3 =  *e.offset(2)**b.offset(3) - *e.offset(3)**b.offset(2);
         p2 = -*e.offset(2)**b.offset(2) - *e.offset(3)**b.offset(3); 

         *d0.offset(2) =   p3;
         *d1.offset(1) = - p3;
         *d2.offset(2) =   p2;
         *d3.offset(1) =   p2;

         p1 =  *e.offset(0)**b.offset(1) - *e.offset(1)**b.offset(0);
         p0 = -*e.offset(0)**b.offset(0) - *e.offset(1)**b.offset(1); 

         *d0.offset(3) =   p1;
         *d1.offset(0) = - p1;
         *d2.offset(3) =   p0;
         *d3.offset(0) =   p0;

         b = b.offset(-8);
         e = e.offset(-8);
         d0 = d0.offset(4);
         d2 = d2.offset(4);
         d1 = d1.offset(-4);
         d3 = d3.offset(-4);
      }
   }

}

unsafe fn start_decoder(f: &mut Vorbis) -> bool
{
   let mut header : [u8; 6] = [0; 6];
   let mut longest_floorlist = 0;
   use VorbisError::*;

   // first page, first packet

   if start_page(f) == false                              {return false;} 
   // validate page flag
   if (f.page_flag & PAGEFLAG_FIRST_PAGE) == 0       {return error(f, InvalidFirstPage)}
   if (f.page_flag & PAGEFLAG_LAST_PAGE) != 0           {return error(f, InvalidFirstPage);}
   if (f.page_flag & PAGEFLAG_CONTINUED_PACKET) != 0   {return error(f, InvalidFirstPage);}
   // check for expected packet length
   if f.segment_count != 1                       {return error(f, InvalidFirstPage);}
   if f.segments[0] != 30                        {return error(f, InvalidFirstPage);}
   // read packet
   // check packet header
   if get8(f) != PACKET_ID                 {return error(f, InvalidFirstPage);}
   if getn(f, &mut header[..]) == false                         {return error(f, UnexpectedEof);}
   if vorbis_validate(&header) == false                    {return error(f, InvalidFirstPage);}
   // vorbis_version
   if get32(f) != 0                               {return error(f, InvalidFirstPage);}
   f.channels = get8(f) as i32; if f.channels == 0        { return error(f, InvalidFirstPage);}
   if f.channels > STB_VORBIS_MAX_CHANNELS       {return error(f, TooManyChannels);}
   f.sample_rate = get32(f); if f.sample_rate == 0  {return error(f, InvalidFirstPage);}
   get32(f); // bitrate_maximum
   get32(f); // bitrate_nominal
   get32(f); // bitrate_minimum

   let mut x : u8 = get8(f);
   {
      let log0 : i32 = (x & 15) as i32;
      let log1 : i32 = (x >> 4) as i32;
      f.blocksize_0 = 1 << log0;
      f.blocksize_1 = 1 << log1;
      if log0 < 6 || log0 > 13                       {return error(f, InvalidSetup);}
      if log1 < 6 || log1 > 13                       {return error(f, InvalidSetup);}
      if log0 > log1                                 {return error(f, InvalidSetup);}
   }
   // framing_flag
   x = get8(f);
   if (x & 1) == 0                                    {return error(f, InvalidFirstPage);}

   // second packet!
   if start_page(f) == false                              {return false;} 
   if start_packet(f) == false                            {return false;} 
   
   let mut len;
   while {
      len = next_segment(f);
      skip(f, len);
      f.bytes_in_seg = 0;
      len != 0
   } {/* do nothing */}

   // third packet!
   if start_packet(f) == false                            {return false;} 

   if f.push_mode && is_whole_packet_present(f, true) == false {
        // convert error in ogg header to write type
        if f.error == InvalidStream {
            f.error = InvalidSetup;
        }
        return false;
   }

   crc32_init(); // always init it, to avoid multithread race conditions

   if get8_packet(f) != PACKET_SETUP as i32       {return error(f, InvalidSetup);}
   for item in header.iter_mut().take(6){
       *item = get8_packet(f) as u8;
   }
   if vorbis_validate(&header) == false                    {return error(f, InvalidSetup);}

   // codebooks
   let mut y : u8;

   f.codebook_count = (get_bits(f,8) + 1) as i32;
   f.codebooks.resize(f.codebook_count as usize, Codebook::default());
   // NOTE(bungcip): no need to resize f.codebooks? just push...
   for i in 0 .. f.codebook_count {
      let mut c : &mut Codebook= FORCE_BORROW_MUT!( &mut f.codebooks[i as usize] );
      CHECK!(f);
      x = get_bits(f, 8) as u8; if x != 0x42            {return error(f, InvalidSetup);}
      x = get_bits(f, 8) as u8; if x != 0x43            {return error(f, InvalidSetup);}
      x = get_bits(f, 8) as u8; if x != 0x56            {return error(f, InvalidSetup);}
      x = get_bits(f, 8) as u8;
      c.dimensions = ((get_bits(f, 8) << 8) as i32 + x as i32) as i32;
      x = get_bits(f, 8) as u8;
      y = get_bits(f, 8) as u8;
      c.entries = ((get_bits(f, 8)<<16) + ( (y as u32) <<8) + x as u32) as i32;
      let is_ordered = get_bits(f,1) != 0;
      c.sparse = if is_ordered { false } else { get_bits(f,1) != 0 };

      if c.dimensions == 0 && c.entries != 0    {return error(f, InvalidSetup);}

      let mut _lengths: Vec<u8> = Vec::new(); // NOTE(bungcip): just temporary
      let mut lengths: &mut [u8] = if c.sparse == true {
          _lengths.resize(c.entries as usize, 0);
          FORCE_BORROW_MUT!( &mut _lengths[..] )
      }else{
          c.codeword_lengths.resize(c.entries as usize, 0);
          FORCE_BORROW_MUT!( &mut c.codeword_lengths[..] )
      };

      let mut total = 0;
      if is_ordered  {
         let mut current_entry = 0;
         let mut current_length = (get_bits(f,5) + 1) as i32;
         while current_entry < c.entries {
            let limit : i32 = c.entries - current_entry;
            let n : i32 = get_bits(f, ilog(limit)) as i32;
            if current_entry + n > c.entries as i32 { return error(f, InvalidSetup); }
            std::ptr::write_bytes(
                lengths[current_entry as usize ..].as_mut_ptr(), 
                current_length as u8, 
                n as usize);
            current_entry += n;
            current_length += 1;
         }
      } else {
         for item in lengths.iter_mut().take(c.entries as usize){
            let present = if c.sparse == true { get_bits(f,1) } else { 1 };
            
            if present != 0 {
               *item = ( get_bits(f, 5) + 1) as u8;
               total += 1;
               if *item == 32 {
                  return error(f, InvalidSetup);
               }
            } else {
               *item = NO_CODE;
            }
         }
      }

      if c.sparse == true && total >= c.entries >> 2 {
         // convert sparse items to non-sparse!
         c.codeword_lengths = _lengths.clone();
         lengths = FORCE_BORROW_MUT!( &mut c.codeword_lengths[..] );
         c.sparse = false;
      }

      // compute the size of the sorted tables
      let mut sorted_count: i32;
      if c.sparse == true {
         sorted_count = total;
      } else {
         sorted_count = 0;
         for &item in lengths.iter().take(c.entries as usize){
            if item > STB_FAST_HUFFMAN_LENGTH as u8 && item != NO_CODE {
               sorted_count += 1;
            }
         }
      }

      c.sorted_entries = sorted_count;
      let mut values: Vec<u32> = Vec::new();

      CHECK!(f);
      if c.sparse == false {
         c.codewords.resize(c.entries as usize, 0);
      } else if c.sorted_entries != 0 {
         c.codeword_lengths.resize(c.sorted_entries as usize, 0);
         c.codewords.resize(c.sorted_entries as usize, 0);
         values.resize(c.sorted_entries as usize, 0);
      }

      if compute_codewords(c, lengths, &mut values) == false {
        return error(f, InvalidSetup);
      }

      if c.sorted_entries != 0 {
         // allocate an extra slot for sentinels
         c.sorted_codewords.resize( (c.sorted_entries+1) as usize, 0);
         c.sorted_values.resize(c.sorted_entries as usize, 0);
         
         compute_sorted_huffman(c, &mut lengths, &values);
      }

      if c.sparse == true {
         values.clear();
         c.codewords.clear();
      }

      compute_accelerated_huffman(c);

      CHECK!(f);
      c.lookup_type = get_bits(f, 4) as u8;
      if c.lookup_type > 2 {
          return error(f, InvalidSetup);
      }
      if c.lookup_type > 0 {
         c.minimum_value = float32_unpack(get_bits(f, 32));
         c.delta_value = float32_unpack(get_bits(f, 32));
         c.value_bits = ( get_bits(f, 4)+1 ) as u8;
         c.sequence_p = ( get_bits(f,1) ) as u8;
         if c.lookup_type == 1 {
            c.lookup_values = lookup1_values(c.entries, c.dimensions) as u32;
         } else {
            c.lookup_values = c.entries as u32 * c.dimensions as u32;
         }
         if c.lookup_values == 0 {
             return error(f, InvalidSetup);
         }
         
         let mut mults : Vec<u16> = Vec::with_capacity(c.lookup_type as usize);
         for _ in 0 .. c.lookup_values {
            let q = get_bits(f, c.value_bits as i32);
            if q == EOP as u32 { 
                return error(f, InvalidSetup); 
            }
            mults.push(q as u16);
         }
         
         'skip: loop {
         if c.lookup_type == 1 {
            let sparse = c.sparse;
            // pre-expand the lookup1-style multiplicands, to avoid a divide in the inner loop
            if sparse {
               if c.sorted_entries == 0 { 
                //    goto skip;
                break 'skip;
                }
               c.multiplicands.resize( (c.sorted_entries * c.dimensions) as usize, 0.0);
            } else{
               c.multiplicands.resize( (c.entries * c.dimensions) as usize, 0.0);
            }
            
            len = if sparse  { c.sorted_entries } else {c.entries};
            let mut last : f32 = 0.0;
            for j in 0 .. len {
               let z : u32 = if sparse  { c.sorted_values[j as usize] } else {j} as u32;
               let mut div: u32 = 1;
               for k in 0 .. c.dimensions {
                  let off: i32 = (z / div) as i32 % c.lookup_values as i32;
                //   let mut val: f32 = *mults.offset(off as isize) as f32; // NOTE(bungcip) : maybe bugs?
                  let val = mults[off as usize] as f32 * c.delta_value + c.minimum_value + last;
                  c.multiplicands[ (j * c.dimensions + k) as usize] = val;
                  if c.sequence_p !=0 {
                     last = val;
                  }
                  if k + 1 < c.dimensions {
                     use std::u32;
                     if div > u32::MAX / c.lookup_values as u32 {
                        return error(f, InvalidSetup);
                     }
                     div *= c.lookup_values;
                  }
               }
            }
            c.lookup_type = 2;
         }
         else
         {
            let mut last = 0.0;
            CHECK!(f);
            c.multiplicands.resize(c.lookup_values as usize, 0.0);
            for (j, item) in c.multiplicands.iter_mut().enumerate().take(c.lookup_values as usize){
               let val : f32 = mults[j] as f32 * c.delta_value + c.minimum_value + last;
               *item = val;
               if c.sequence_p != 0 {
                  last = val;
               }
            }
         }
         
         // NOTE(bungcip): maybe we can remove loop-break now?
         break;
         } // loop 'skip
//         skip:;
         CHECK!(f);
      }
      CHECK!(f);
   }

   // time domain transfers (notused)

   x = ( get_bits(f, 6) + 1) as u8;
   for _ in 0 .. x {
      let z = get_bits(f, 16);
      if z != 0 { return error(f, InvalidSetup); }
   }

   // Floors
   f.floor_count = (get_bits(f, 6)+1) as i32;
   f.floor_config.resize(f.floor_count as usize, Floor::default());
   // NOTE(bungcip): no need to resize? just push...
   for i in 0 .. f.floor_count as usize {
      f.floor_types[i] = get_bits(f, 16) as u16;
      if f.floor_types[i] > 1 {
          return error(f, InvalidSetup);
      }
      if f.floor_types[i] == 0 {
      // NOTE(bungcip): using transmute because rust don't have support for union yet.. 
      //                transmute floor0 to floor1
         let g: &mut Floor0 = mem::transmute(&mut f.floor_config[i].floor1);
         g.order = get_bits(f,8) as u8;
         g.rate = get_bits(f,16) as u16;
         g.bark_map_size = get_bits(f,16) as u16;
         g.amplitude_bits = get_bits(f,6) as u8;
         g.amplitude_offset = get_bits(f,8) as u8;
         g.number_of_books = (get_bits(f,4) + 1) as u8;
         for j in 0 .. g.number_of_books as usize {
            g.book_list[j] = get_bits(f,8) as u8;
         }
         return error(f, FeatureNotSupported);
      } else {
         let mut g : &mut Floor1 = FORCE_BORROW_MUT!( &mut f.floor_config[i].floor1 );
         let mut max_class : i32 = -1; 
         g.partitions = get_bits(f, 5) as u8;
         for j in 0 .. g.partitions as usize {
            g.partition_class_list[j] = get_bits(f, 4) as u8;
            if g.partition_class_list[j] as i32 > max_class {
               max_class = g.partition_class_list[j] as i32;
            }
         }
         for j in 0 .. (max_class + 1) as usize {
            g.class_dimensions[j] = get_bits(f, 3) as u8 + 1;
            g.class_subclasses[j] = get_bits(f, 2) as u8;
            if g.class_subclasses[j] != 0 {
               g.class_masterbooks[j] = get_bits(f, 8) as u8;
               if g.class_masterbooks[j] >= f.codebook_count as u8 {
                   return error(f, InvalidSetup);
               }
            }
            for k in 0 ..  (1 << g.class_subclasses[j]) as usize {
               g.subclass_books[j][k] = get_bits(f,8) as i16 -1;
               if g.subclass_books[j][k] >= f.codebook_count as i16 {
                   return error(f, InvalidSetup);
               }
            }
         }
         g.floor1_multiplier = (get_bits(f,2) +1) as u8;
         g.rangebits = get_bits(f,4) as u8;
         g.xlist[0] = 0;
         g.xlist[1] = 1 << g.rangebits;
         g.values = 2;
         for j in 0 .. g.partitions as usize {
            let c = g.partition_class_list[j] as usize;
            for _ in 0 .. g.class_dimensions[c] {
               g.xlist[g.values as usize] = get_bits(f, g.rangebits as i32) as u16;
               g.values += 1;
            }
         }

         // precompute the sorting
         {
            let mut points : Vec<Point> = Vec::with_capacity(31*8+2);
            for (j, item) in g.xlist.iter().enumerate().take(g.values as usize) {
                points.push(Point{ x: *item, y: j as u16});
            }
            
            points.sort();
            
            for (j, item) in points.iter().enumerate() {
                g.sorted_order[j] = item.y as u8;
            }
         }

         // precompute the neighbors
         for j in 2 .. g.values as usize {
            let mut low = 0;
            let mut hi = 0;
            neighbors(&g.xlist, j, &mut low, &mut hi);
            g.neighbors[j][0] = low as u8;
            g.neighbors[j][1] = hi as u8;
         }

         if g.values > longest_floorlist{
            longest_floorlist = g.values;
         }
      }
   }

   // Residue
   f.residue_count = get_bits(f, 6) as i32 + 1;
   f.residue_config.resize(f.residue_count as usize, Residue::default());
   for i in 0 .. f.residue_count as usize {
      let mut residue_cascade: [u8; 64] = mem::zeroed();
      let mut r : &mut Residue = FORCE_BORROW_MUT!( &mut f.residue_config[i] );
      f.residue_types[i] = get_bits(f, 16) as u16;
      if f.residue_types[i] > 2 {
          return error(f, InvalidSetup);
      }
      r.begin = get_bits(f, 24);
      r.end = get_bits(f, 24);
      if r.end < r.begin {
          return error(f, InvalidSetup);
      }
      r.part_size = get_bits(f,24)+1;
      r.classifications = get_bits(f,6) as u8 + 1;
      r.classbook = get_bits(f,8) as u8;
      if r.classbook as i32 >= f.codebook_count {
          return error(f, InvalidSetup);
      }
      for item in residue_cascade.iter_mut().take(r.classifications as usize){
         let mut high_bits: u8 = 0;
         let low_bits: u8 = get_bits(f,3) as u8;
         if get_bits(f,1) != 0 {
            high_bits = get_bits(f,5) as u8;
         }
         *item = high_bits*8 + low_bits;
      }
      r.residue_books.resize(r.classifications as usize, [0; 8]);
      for (j, item) in residue_cascade.iter_mut().enumerate().take(r.classifications as usize){
         for k in 0usize .. 8 {
            if (*item & (1 << k)) != 0 {
               r.residue_books[j][k] = get_bits(f, 8) as i16;
               if r.residue_books[j][k] as i32 >= f.codebook_count {
                   return error(f, InvalidSetup);
                }
            } else {
               r.residue_books[j][k] = -1;
            }
         }
      }
      // precompute the classifications[] array to avoid inner-loop mod/divide
      // call it 'classdata' since we already have r.classifications
      // NOTE(bungcip): remove resize?
      r.classdata.resize(f.codebooks[r.classbook as usize].entries as usize, Vec::new());
      for j in 0 .. f.codebooks[r.classbook as usize].entries as usize {
         let mut temp = j as i32;
         let classwords_size = f.codebooks[r.classbook as usize].dimensions;
         r.classdata[j].resize(classwords_size as usize, 0);
         for item in r.classdata[j].iter_mut().rev() {
            *item = (temp % r.classifications as i32) as u8;
            temp /= r.classifications as i32;
         }
      }
   }

   let mut max_submaps = 0;
   f.mapping_count = get_bits(f,6) as i32 +1;
   f.mapping.resize(f.mapping_count as usize, Mapping::default());
   for i in 0 .. f.mapping_count as usize {
      let m : &mut Mapping = FORCE_BORROW_MUT!( &mut f.mapping[i]);
      let mapping_type : i32 = get_bits(f,16) as i32;
      if mapping_type != 0 {
          return error(f, InvalidSetup);
      }

      m.chan.resize(f.channels as usize, MappingChannel::default());
      
      if get_bits(f,1) != 0 {
         m.submaps = get_bits(f,4) as u8 + 1;
      } else {
         m.submaps = 1;
      }
      max_submaps = std::cmp::max(max_submaps, m.submaps);
      
      if get_bits(f,1) != 0 {
         m.coupling_steps = get_bits(f,8) as u16 + 1;
         for k in 0 .. m.coupling_steps as usize {
            // satify borrow checker
            let ilog_result = ilog(f.channels-1);
            m.chan[k].magnitude = get_bits(f, ilog_result) as u8;
            let ilog_result = ilog(f.channels-1);
            m.chan[k].angle = get_bits(f, ilog_result) as u8;
            if m.chan[k].magnitude as i32 >= f.channels        {return error(f, InvalidSetup);}
            if m.chan[k].angle     as i32 >= f.channels        {return error(f, InvalidSetup);}
            if m.chan[k].magnitude == m.chan[k].angle   {return error(f, InvalidSetup);}
         }
      } else{
         m.coupling_steps = 0;
      }

      // reserved field
      if get_bits(f,2) != 0 {
          return error(f, InvalidSetup);
      }
      if m.submaps > 1 {
         for j in 0 .. f.channels as usize {
            m.chan[j].mux = get_bits(f, 4) as u8;
            if m.chan[j].mux >= m.submaps {
                return error(f, InvalidSetup);
            }
         }
      } else {
         // @SPECIFICATION: this case is missing from the spec
         for j in 0 .. f.channels as usize {
            m.chan[j].mux = 0;
         }
      }

      for j in 0 .. m.submaps as usize {
         get_bits(f,8); // discard
         m.submap_floor[j] = get_bits(f,8) as u8;
         m.submap_residue[j] = get_bits(f,8) as u8;
         if m.submap_floor[j] as i32 >= f.floor_count      {return error(f, InvalidSetup);}
         if m.submap_residue[j] as i32 >= f.residue_count  {return error(f, InvalidSetup);}
      }
   }

   // Modes
   f.mode_count = get_bits(f, 6) as i32 + 1;
   for i in 0 .. f.mode_count as usize {
      let m: &mut Mode = FORCE_BORROW_MUT!(&mut f.mode_config[i]);
      m.blockflag = get_bits(f,1) as u8;
      m.windowtype = get_bits(f,16) as u16;
      m.transformtype = get_bits(f,16) as u16;
      m.mapping = get_bits(f,8) as u8;
      if m.windowtype != 0                 {return error(f, InvalidSetup);}
      if m.transformtype != 0              {return error(f, InvalidSetup);}
      if m.mapping as i32 >= f.mapping_count     {return error(f, InvalidSetup);}
   }

   flush_packet(f);

   f.previous_length = 0;
   
   f.channel_buffers.resize(f.channels as usize, Vec::new());
   f.previous_window.resize(f.channels as usize, Vec::new());
   f.final_y.resize(f.channels as usize, Vec::new());
   
   for i in 0 .. f.channels as usize {
      let block_size_1 = f.blocksize_1;
      f.channel_buffers[i].resize(block_size_1 as usize, 0.0);
      f.previous_window[i].resize( (block_size_1/2) as usize, 0.0);
      f.final_y[i].resize(longest_floorlist as usize, 0);
   }

   {  
       let blocksize_0 = f.blocksize_0;
       let blocksize_1 = f.blocksize_1;
       init_blocksize(f, 0, blocksize_0); 
       init_blocksize(f, 1, blocksize_1); 
       f.blocksize[0] = blocksize_0;
       f.blocksize[1] = blocksize_1;
   }


   f.first_decode = true;
   f.first_audio_page_offset = stb_vorbis_get_file_offset(f);
   return true;
}
