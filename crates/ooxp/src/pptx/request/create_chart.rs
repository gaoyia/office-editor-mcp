use serde_json::Value;

use crate::error::{Error, ErrorKind, TypeName};

use super::CreateChart;

pub(crate) fn parse_charts(v: Option<&Value>, slide_i: usize) -> Result<Vec<CreateChart>, Error> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    let prefix = format!("presentation.slides[{slide_i}].charts");
    let arr = v.as_array().ok_or_else(|| {
        Error::with_path(
            ErrorKind::WrongType {
                field: "charts".into(),
                expected: TypeName::Array,
            },
            prefix.clone(),
        )
    })?;
    let mut out = Vec::new();
    for (j, item) in arr.iter().enumerate() {
        out.push(super::parse_create_chart(
            item,
            &format!("{prefix}[{j}]"),
            slide_i + 1,
        )?);
    }
    Ok(out)
}
