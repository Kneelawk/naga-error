#![cfg(feature = "use-wgsl-src")]

use std::borrow::Cow;
use wgpu::ShaderSource;

const TEMPLATE_SOURCE: &'static str = include_str!("template.wgsl");

pub fn use_wgsl_src() -> ShaderSource<'static> {
    ShaderSource::Wgsl(Cow::Borrowed(TEMPLATE_SOURCE))
}
