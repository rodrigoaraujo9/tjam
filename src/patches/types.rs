use rodio::Source;
pub type DynSrc = Box<dyn Source<Item = f32> + Send>;
