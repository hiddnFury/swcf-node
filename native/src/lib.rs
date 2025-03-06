pub mod extractors;
pub mod networking;
pub mod transformers;
pub mod utils;
pub mod vm;
use crate::{context::ModuleContext, handle::Handle, result::NeonResult, types::JsValue};

use neon::prelude::*;
use std::fs;

fn deobfuscate(src: &str) -> String {
    let mut cnfg = extractors::config_builder::VMConfig::default();
    let out: String = utils::deobfuscate_script::deobfuscate(&mut cnfg, src);
    cnfg.find_all_enc(&src);
    out
}

register_module!(mut cx, {
    cx.export_function("read_file_to_string", read_file_to_string)?;
    cx.export_function("deobfuscate_scopes", deobfuscate_scopes)?;
});

fn read_file_to_string(mut cx: FunctionContext) -> JsResult<JsString> {
    let path_prm = cx.argument::<JsString>(0)?;
    let path_str = path_prm.value(&mut cx).to_string();
    let contents = fs::read_to_string(path_str).expect("Error: Unable to read the file.");
    Ok(cx.string(contents))
}

fn deobfuscate_scopes(mut cx: FunctionContext) -> JsResult<JsString> {
    let raw_js_prm = cx.argument::<JsString>(0)?;
    let raw_js_str = raw_js_prm.value(&mut cx).to_string();
    Ok(cx.string(deobfuscate(&raw_js_str)))
}
