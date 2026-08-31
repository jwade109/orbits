pub trait IntoGpu {
    const LAYOUT_SIZE: usize;
    fn into_gpu(&self) -> Vec<u8>;
}
