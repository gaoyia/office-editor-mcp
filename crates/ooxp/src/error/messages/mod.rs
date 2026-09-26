//! 按语言渲染 `ErrorKind`。每种语言一个穷尽 match，漏译是编译错误。

use super::kind::ErrorKind;
use super::Lang;

mod en;
mod zh_cn;

pub(super) fn render(lang: Lang, kind: &ErrorKind) -> String {
    match lang {
        Lang::En => en::render(kind),
        Lang::ZhCn => zh_cn::render(kind),
    }
}
