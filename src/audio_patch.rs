use rodio::Source;

pub type SynthSource = Box<dyn Source<Item = f32> + Send>;

pub trait AudioSource: Send + Sync {
    fn create_source(&self, frequency: f32) -> SynthSource;
    fn name(&self) -> &'static str;
}

pub trait Node: Send + Sync {
    fn apply(&self, input: SynthSource) -> SynthSource;
    fn name(&self) -> &'static str;
}

pub trait Generator: Send + Sync {
    fn create(&self, frequency: f32) -> SynthSource;
    fn name(&self) -> &'static str;
}

pub struct PatchSource {
    generator: Box<dyn Generator>,
    nodes: Vec<Box<dyn Node>>,
}

impl PatchSource {
    pub fn new(generator: Box<dyn Generator>) -> Self {
        Self { generator, nodes: vec![] }
    }

    pub fn push_node(mut self, node: Box<dyn Node>) -> Self {
        self.nodes.push(node);
        self
    }
}

impl AudioSource for PatchSource {
    fn create_source(&self, frequency: f32) -> SynthSource {
        let mut src = self.generator.create(frequency);
        for n in &self.nodes {
            src = n.apply(src);
        }
        src
    }

    fn name(&self) -> &'static str {
        self.generator.name()
    }
}
