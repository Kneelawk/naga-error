#![cfg(feature = "use-naga-spv")]

use wgpu::{util::make_spirv, ShaderSource};

const TEMPLATE_NAGA_SPV: &'static [u8] = include_bytes!("template.naga.spv");

pub fn use_naga_spv() -> ShaderSource<'static> {
    make_spirv(TEMPLATE_NAGA_SPV)
}
