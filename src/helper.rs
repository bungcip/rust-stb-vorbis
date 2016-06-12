/// Helper Module
/// 
use ::std::ptr;
use ::std::ops::{Index, IndexMut};
use ::std::slice;


/// this basically slice but for vorbis audio buffer and not owned.
/// just work around so I don't need to add lifetime annotation to ALL function that use
/// Vorbis struct.
#[derive(Copy, Clone)]
pub struct AudioBufferSlice<T> {
    pub channel_count: usize,
    buffers: [*mut T; 16],
    sizes: [usize; 16],
}

impl<T> AudioBufferSlice<T> {
    pub fn new(channel_count: usize) -> Self {
        let buffers = [ptr::null_mut::<T>(); 16];
        let sizes = [0usize; 16];
        
        AudioBufferSlice {
            channel_count: channel_count,
            buffers: buffers,
            sizes: sizes
        }
    }

    pub unsafe fn from(value: &mut Vec<Vec<T>>) -> Self {
        let mut buffers = [ptr::null_mut::<T>(); 16];
        let mut sizes: [usize; 16] = [0usize; 16];
        
        let channel_count = value.len();
        for i in 0 .. value.len() {
            buffers[i] = value[i].as_mut_ptr();
            sizes[i] = value[i].len();
        }
        
        AudioBufferSlice {
            channel_count: channel_count,
            buffers: buffers,
            sizes: sizes
        }
    }

    pub unsafe fn from_single_channel(value: &mut [T]) -> Self {
        let mut buffers = [ptr::null_mut::<T>(); 16];
        let mut sizes: [usize; 16] = [0usize; 16];
        
        let channel_count = 1;
        buffers[0] = value.as_mut_ptr();
        sizes[0] = value.len();
        
        AudioBufferSlice {
            channel_count: channel_count,
            buffers: buffers,
            sizes: sizes
        }
    }
    
    /// set content buffer in audio buffer slice, you must ensure
    /// that lifetime of content outlive AudioBufferSlice
    pub unsafe fn set(&mut self, channel_index: usize, values: &mut [T]){
        debug_assert!(channel_index < self.channel_count);
        
        self.buffers[channel_index] = values.as_mut_ptr();
        self.sizes[channel_index] = values.len();
    }
    
    /// add new channel data. increase channel_count.
    /// you must ensure that lifetime of content outlive AudioBufferSlice
    pub unsafe fn push_channel(&mut self, values: &mut [T]){
        let channel_index = self.channel_count;
        self.channel_count += 1;

        self.set(channel_index, values);
    }

    pub fn as_ptr(&self) -> *const *mut T {
        self.buffers.as_ptr()
    }
    
    /// get length of first channel data, use channel_count if you need to count the number of channel
    pub fn len(&self) -> usize {
        self.sizes[0]
    }

}


impl<T> Index<(usize, usize)> for AudioBufferSlice<T> {
    type Output = T;

    fn index<'a>(&'a self, _index: (usize, usize)) -> &'a T {
        assert!(_index.0 < self.channel_count);
        assert!(_index.1 < self.sizes[_index.0]);
        
        unsafe {
            &*self.buffers.get_unchecked(_index.0).offset(_index.1 as isize)
        }
    }    
}

impl<T> Index<usize> for AudioBufferSlice<T> {
    type Output = [T];

    fn index<'a>(&'a self, _index: usize) -> &'a [T] {
        assert!(_index < self.channel_count);
        
        unsafe {
            slice::from_raw_parts(
                *self.buffers.get_unchecked(_index), 
                *self.sizes.get_unchecked(_index)
            )
        }
    }    
}

impl<T> IndexMut<usize> for AudioBufferSlice<T> {
    fn index_mut(&mut self, index: usize) -> &mut [T] {
        assert!(index < self.channel_count);
        
        unsafe {
            slice::from_raw_parts_mut(
                *self.buffers.get_unchecked_mut(index), 
                *self.sizes.get_unchecked_mut(index)
            )
        }
    }    
}