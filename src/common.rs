#[derive(Clone, Copy, Debug)]
pub enum ShimFormat<'global> {
	EsModule,
	Classic { global: &'global str },
}
