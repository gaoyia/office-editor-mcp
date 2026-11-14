//! ooxp：用领域 JSON 对 OOXML 做 inspect / create / apply / exportMedia / exportChartData / exportChartImage / exportSlideImage / exportPageImage / help。
// apply / inspect 会同时传入包、脏部件和领域字段，参数经常超过 7 个，不另包一层上下文结构。
#![allow(clippy::too_many_arguments)]

mod chart;
mod dispatch;
mod docx;
mod drawing_geom;
mod drawing_math;
mod drawing_text;
mod error;
mod help;
mod host_env;
mod host_files;
mod image;
mod ink;
mod json;
mod model3d;
mod ole;
mod op_fields;
mod opc;
mod password;
mod pptx;
mod props;
mod shape_runs;
mod smartart;
mod svg;
mod text_span;
mod theme;
mod xlsx;
mod xml;

#[cfg(not(target_arch = "wasm32"))]
pub use dispatch::run;
pub use dispatch::{run_bytes, ByteOutput};
pub use error::{io_path, peek_lang, Error, ErrorCode, Lang};
pub use host_env::{install as install_clock, EnvGuard};
pub use json::{dry_run_json, dry_run_value, envelope_fields, written_json, Action, RunOutput};
pub use model3d::sample_glb;

#[cfg(test)]
mod facade_tests;
