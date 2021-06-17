#![cfg(feature = "use-tint-spv")]

use wgpu::{util::make_spirv, ShaderSource};

const TEMPLATE_TINT_SPV: &'static [u8] = include_bytes!("template.tint.spv");

pub fn use_tint_spv() -> ShaderSource<'static> {
    make_spirv(TEMPLATE_TINT_SPV)
}
