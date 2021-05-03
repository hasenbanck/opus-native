/* Ideas for the interface:
fn max_encoded_size(&self, samples: usize) -> usize;
fn encode<T: Sample>(&self, samples: &[T], out: &mut [u8]) -> usize;

pub struct EncodeResult {
    pub samples_consumed: usize,
    pub bytes_written: usize,
};
fn encode<T: Sample>(&self, samples: &[T], out: &mut [u8]) -> EncodeResult;
 */
