#![cfg(feature = "use-naga-lib")]

use core::slice;
use naga::{
    back, front,
    valid::{ValidationFlags, Validator},
};
use std::{borrow::Cow, io, mem::size_of};
use tokio::{fs::File, io::AsyncWriteExt};
use wgpu::ShaderSource;

const DEBUG_TXT_OUTPUT_LOCATION: &'static str = "debug.txt";
const DEBUG_SPV_OUTPUT_LOCATION: &'static str = "debug.spv";

const TEMPLATE_SOURCE: &'static str = include_str!("template.wgsl");

pub async fn use_naga_lib() -> ShaderSource<'static> {
    ShaderSource::SpirV(Cow::Owned(wgsl_to_spv().await))
}

async fn wgsl_to_spv() -> Vec<u32> {
    info!("Loading WGSL...");
    let module = front::wgsl::parse_str(TEMPLATE_SOURCE).unwrap();

    info!("Validating module...");
    let mut validator = Validator::new(ValidationFlags::all());
    let module_info = validator.validate(&module).unwrap();

    info!("Writing module as text...");
    let mut file = File::create(DEBUG_TXT_OUTPUT_LOCATION).await.unwrap();
    file.write_all(format!("{:#?}", &module).as_bytes())
        .await
        .unwrap();
    file.write_all(b"\n\n").await.unwrap();
    file.write_all(format!("{:#?}", &module_info).as_bytes())
        .await
        .unwrap();

    info!("Translating spirv...");
    let mut spirv = vec![];
    let options = back::spv::Options::default();
    let mut writer = back::spv::Writer::new(&options).unwrap();
    writer.write(&module, &module_info, &mut spirv).unwrap();

    info!("Writing spirv...");
    let mut spirv_file = File::create(DEBUG_SPV_OUTPUT_LOCATION).await.unwrap();
    write_spirv(&mut spirv_file, &spirv).await.unwrap();

    spirv
}

async fn write_spirv(writer: &mut File, spirv: &[u32]) -> io::Result<()> {
    let u8slice = unsafe {
        slice::from_raw_parts(spirv.as_ptr() as *const u8, spirv.len() * size_of::<u32>())
    };
    writer.write_all(u8slice).await
}
