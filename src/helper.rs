/// Helper Module
/// 
use ::std::ptr;
use ::std::ops::Index;
use ::std::slice;


/// this basically slice but for vorbis audio buffer and not owned.
/// just work around so I don't need to add lifetime annotation to ALL function that use
/// Vorbis struct.
#[derive(Copy, Clone)]
pub struct AudioBufferSlice {
    channel_count: usize,
    buffers: [*const f32; 16],
    sizes: [usize; 16],
}

impl AudioBufferSlice {
    pub fn new(channel_count: usize) -> Self {
        let buffers = [ptr::null::<f32>(); 16];
        let sizes = [0usize; 16];
        
        AudioBufferSlice {
            channel_count: channel_count,
            buffers: buffers,
            sizes: sizes
        }
    }
    
    // FIXME:(change this to trait From)
    pub unsafe fn from(value: &Vec<Vec<f32>>) -> Self {
        let mut buffers: [*const f32; 16] = [ptr::null::<f32>(); 16];
        let mut sizes: [usize; 16] = [0usize; 16];
        
        let channel_count = value.len();
        for i in 0 .. value.len() {
            buffers[i] = value[i].as_ptr();
            sizes[i] = value[i].len();
        }
        
        AudioBufferSlice {
            channel_count: channel_count,
            buffers: buffers,
            sizes: sizes
        }
    }
    
    // set content buffer in audio buffer slice, you must ensure
    // that lifetime of content outlive AudioBufferSlice
    pub unsafe fn set(&mut self, channel_index: usize, values: &[f32]){
        debug_assert!(channel_index < self.channel_count);
        
        self.buffers[channel_index] = values.as_ptr();
        self.sizes[channel_index] = values.len();
    }
    
    pub fn as_ptr(&self) -> *const *const f32 {
        self.buffers.as_ptr()
    }
    

}


impl Index<(usize, usize)> for AudioBufferSlice {
    type Output = f32;

    fn index<'a>(&'a self, _index: (usize, usize)) -> &'a f32 {
        assert!(_index.0 < self.channel_count);
        assert!(_index.1 < self.sizes[_index.0]);
        
        unsafe {
            &*self.buffers[_index.0].offset(_index.1 as isize)
        }
    }    
}

impl Index<usize> for AudioBufferSlice {
    type Output = [f32];

    fn index<'a>(&'a self, _index: usize) -> &'a [f32] {
        unsafe {
            slice::from_raw_parts(self.buffers[_index], self.sizes[_index])
        }
    }    
}