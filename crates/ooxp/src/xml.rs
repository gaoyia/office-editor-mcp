//! 小型 XML 树：给 OPC 关系文件和 SpreadsheetML 部件做手术刀式改写。

use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::error::{Error, ErrorCode};

#[derive(Clone, Debug)]
pub struct Document {
    pub decl: Option<String>,
    pub root: Element,
}

#[derive(Clone, Debug)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Clone, Debug)]
pub struct Element {
    /// 带前缀的名字，例如 `r:id` 用在属性上；元素则是 `workbook` 或 `r:id` 这种写成 `sheet`。
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.push((key.into(), value.into()));
        self
    }

    pub fn local_name(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }

    pub fn attr(&self, local: &str) -> Option<&str> {
        self.attrs.iter().find_map(|(k, v)| {
            let key_local = k.rsplit(':').next().unwrap_or(k);
            if key_local.eq_ignore_ascii_case(local) || k.eq_ignore_ascii_case(local) {
                Some(v.as_str())
            } else {
                None
            }
        })
    }

    pub fn set_attr(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some((_, v)) = self
            .attrs
            .iter_mut()
            .find(|(k, _)| k == key || k.rsplit(':').next().unwrap_or(k).eq_ignore_ascii_case(key))
        {
            *v = value;
            return;
        }
        self.attrs.push((key.to_string(), value));
    }

    pub fn remove_attr(&mut self, local: &str) {
        self.attrs.retain(|(k, _)| {
            let key_local = k.rsplit(':').next().unwrap_or(k);
            !key_local.eq_ignore_ascii_case(local) && !k.eq_ignore_ascii_case(local)
        });
    }

    pub fn element_children(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    pub fn element_children_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.children.iter_mut().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    pub fn child_by_local(&self, local: &str) -> Option<&Element> {
        self.element_children().find(|e| e.local_name() == local)
    }

    pub fn child_by_local_mut(&mut self, local: &str) -> Option<&mut Element> {
        self.element_children_mut()
            .find(|e| e.local_name() == local)
    }

    pub fn children_by_local(&self, local: &str) -> Vec<&Element> {
        self.element_children()
            .filter(|e| e.local_name() == local)
            .collect()
    }

    pub fn text_content(&self) -> String {
        let mut out = String::new();
        collect_text(self, &mut out);
        out
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.children = vec![Node::Text(text.into())];
    }

    pub fn push_element(&mut self, el: Element) {
        self.children.push(Node::Element(el));
    }

    /// 前序遍历本元素及子孙。
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Element)) {
        f(self);
        for n in &self.children {
            if let Node::Element(e) = n {
                e.walk(f);
            }
        }
    }

    /// 在 `name` 带前缀时（如 `w:p`）给子元素用同一前缀。
    pub fn prefixed(&self, local: &str) -> String {
        match self.name.split_once(':') {
            Some((pfx, _)) => format!("{pfx}:{local}"),
            None => local.to_string(),
        }
    }
}

fn collect_text(el: &Element, out: &mut String) {
    for n in &el.children {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => collect_text(e, out),
        }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Document, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    let mut decl: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(d)) => {
                decl = Some(decl_to_string(&d));
            }
            Ok(Event::Start(e)) => {
                stack.push(start_to_element(&e, &reader)?);
            }
            Ok(Event::Empty(e)) => {
                let el = start_to_element(&e, &reader)?;
                if let Some(parent) = stack.last_mut() {
                    parent.push_element(el);
                } else {
                    root = Some(el);
                }
            }
            Ok(Event::End(_)) => {
                let finished = stack
                    .pop()
                    .ok_or_else(|| Error::new(ErrorCode::XmlBroken, "XML 结束标签多于开始标签"))?;
                if let Some(parent) = stack.last_mut() {
                    parent.push_element(finished);
                } else {
                    root = Some(finished);
                }
            }
            Ok(Event::Text(t)) => {
                let text = t
                    .unescape()
                    .map_err(|e| {
                        Error::new(ErrorCode::XmlBroken, format!("XML 文本无法解码：{e}"))
                    })?
                    .into_owned();
                if text.is_empty() {
                    buf.clear();
                    continue;
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(text));
                }
            }
            Ok(Event::CData(t)) => {
                let text = String::from_utf8_lossy(&t).into_owned();
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Text(text));
                }
            }
            Ok(Event::Comment(_)) | Ok(Event::PI(_)) | Ok(Event::DocType(_)) => {}
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::new(
                    ErrorCode::XmlBroken,
                    format!("XML 解析失败：{e}"),
                ));
            }
        }
        buf.clear();
    }

    let root = root.ok_or_else(|| Error::new(ErrorCode::XmlBroken, "XML 没有根元素"))?;
    Ok(Document { decl, root })
}

fn start_to_element(e: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<Element, Error> {
    let name = qname_to_string(e.name().as_ref());
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        let attr = attr
            .map_err(|err| Error::new(ErrorCode::XmlBroken, format!("XML 属性无法读取：{err}")))?;
        let key = qname_to_string(attr.key.as_ref());
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(|err| Error::new(ErrorCode::XmlBroken, format!("XML 属性值无法解码：{err}")))?
            .into_owned();
        attrs.push((key, value));
    }
    Ok(Element {
        name,
        attrs,
        children: Vec::new(),
    })
}

fn qname_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn decl_to_string(d: &BytesDecl) -> String {
    let mut s = String::from("<?xml");
    if let Ok(v) = d.version() {
        s.push_str(&format!(" version=\"{}\"", String::from_utf8_lossy(&v)));
    }
    if let Some(Ok(enc)) = d.encoding() {
        s.push_str(&format!(" encoding=\"{}\"", String::from_utf8_lossy(&enc)));
    }
    if let Some(Ok(st)) = d.standalone() {
        s.push_str(&format!(" standalone=\"{}\"", String::from_utf8_lossy(&st)));
    }
    s.push_str("?>");
    s
}

pub const DEFAULT_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

impl Document {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        // 声明单独拼在前面：写进 Writer 内部的 Cursor 会被后续 write_event 从头覆盖。
        let mut out = Vec::new();
        out.extend_from_slice(self.decl.as_deref().unwrap_or(DEFAULT_DECL).as_bytes());
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        write_element(&mut writer, &self.root)?;
        out.extend_from_slice(&writer.into_inner().into_inner());
        Ok(out)
    }
}

fn write_element(writer: &mut Writer<Cursor<Vec<u8>>>, el: &Element) -> Result<(), Error> {
    let mut start = BytesStart::new(el.name.as_str());
    for (k, v) in &el.attrs {
        start.push_attribute((k.as_str(), v.as_str()));
    }
    if el.children.is_empty() {
        writer
            .write_event(Event::Empty(start))
            .map_err(|e| Error::new(ErrorCode::XmlBroken, format!("写出 XML 失败：{e}")))?;
        return Ok(());
    }
    writer
        .write_event(Event::Start(start))
        .map_err(|e| Error::new(ErrorCode::XmlBroken, format!("写出 XML 失败：{e}")))?;
    for child in &el.children {
        match child {
            Node::Text(t) => {
                writer
                    .write_event(Event::Text(BytesText::new(t)))
                    .map_err(|e| Error::new(ErrorCode::XmlBroken, format!("写出 XML 失败：{e}")))?;
            }
            Node::Element(e) => write_element(writer, e)?,
        }
    }
    writer
        .write_event(Event::End(BytesEnd::new(el.name.as_str())))
        .map_err(|e| Error::new(ErrorCode::XmlBroken, format!("写出 XML 失败：{e}")))?;
    Ok(())
}

pub fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_survives_roundtrip() {
        let src =
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a x="1"><b>t</b></a>"#;
        let out = parse(src).unwrap().to_bytes().unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#));
        assert!(text.ends_with(r#"<a x="1"><b>t</b></a>"#));
    }

    #[test]
    fn missing_declaration_gets_default() {
        let out = parse(b"<a/>").unwrap().to_bytes().unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text, format!("{DEFAULT_DECL}<a/>"));
    }
}
