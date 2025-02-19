use ChunkerImpl;

/// A chunker that implements sequential content‐defined chunking using a
/// configurable threshold of consecutive increasing or decreasing bytes.
/// For this implementation, we focus on the _increasing_ case.
/// If the desired sequence is not met after observing a given number of
/// "opposing" slopes, the algorithm jumps forward a fixed number of bytes.
///
/// Source: Sreeharsha Udayashankar, Abdelrahman Baba, Samer Al-Kiswany: SeqCDC: Hashless Content-Defined 
/// Chunking for Data Deduplication. MIDDLEWARE '24: Proceedings of the 25th International Middleware 
/// Conference, Pages 292-298, https://doi.org/10.1145/3652892.3700766
#[derive(Clone, Debug, Default)]
struct SeqChunkerState {
    /// The length of the current incrementing run.
    curr_seq_length: usize,

    /// Current opposing slope length.
    opposing_slope_count: usize,

    /// The previously ingested byte, if any.
    previous_value: Option<u8>,

    /// The position _relative to the last chunk boundary_.
    pos: usize,
}

impl SeqChunkerState {
    fn reset(&mut self) {
        self.curr_seq_length = 0;
        self.opposing_slope_count = 0;
        self.previous_value = None;
        self.pos = 0;
    }
}

#[derive(Clone, Debug)]
pub struct SeqChunker {
    /// Length of incrementing run required to trigger a chunk boundary.
    seq_length: usize,

    /// Length of opposing slope required to trigger a chunk boundary.
    skip_trigger: usize,

    /// The number of bytes to skip ahead when the skip trigger is met.
    skip_size: usize,

    /// The current state of the chunker.
    state: SeqChunkerState,
}

impl SeqChunker {
    pub fn new(seq_length: usize, skip_trigger: usize, skip_size: usize) -> Self {
        SeqChunker {
            seq_length,
            skip_trigger,
            skip_size,
            state: Default::default(),
        }
    }
}

impl ChunkerImpl for SeqChunker {
    /// Searches for a chunk boundary in the provided data block.
    /// If a boundary is found that is not the very end of the data block,
    /// its index is returned within an Option.
    fn find_boundary(&mut self, data: &[u8]) -> Option<usize> {
        let mut i = self.state.pos;
        
        while i < data.len() {
            let b = data[i];
            
            if let Some(previous_value) = self.state.previous_value {
                // Compare current and previous byte
                let cmp_result = b as i16 - previous_value as i16;
                
                // Low Entropy Absorption - skip identical bytes
                if cmp_result == 0 {
                    i += 1;
                    continue;
                }
                
                let cmp_sign = cmp_result.is_negative();
                
                // Increment opposing slope count when current < previous (cmp_sign is true)
                self.state.opposing_slope_count += cmp_sign as usize;
                
                // Reset or increment sequence length
                self.state.curr_seq_length = (self.state.curr_seq_length * (!cmp_sign as usize)) + (!cmp_sign as usize);
                
                // Check if we've found a boundary
                if self.state.curr_seq_length == self.seq_length {
                    return Some(i);
                }
                
                // Check if we need to skip ahead
                if self.state.opposing_slope_count == self.skip_trigger {
                    // advance 1 as usual, subtract 1 to save previous bytes, add skip_size
                    i += self.skip_size;
                    self.state.reset();
                    continue;
                }
            }
            
            self.state.previous_value = Some(b);
            i += 1;
        }
        
        // No cut-point found within the current data block
        self.state.pos = i - data.len();
        None
    }

    fn reset(&mut self) {
        self.state.reset();
    }
}