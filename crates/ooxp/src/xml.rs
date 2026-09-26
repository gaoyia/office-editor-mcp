//! 小型 XML 树：给 OPC 关系文件和 SpreadsheetML 部件做手术刀式改写。

use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::error::{Error, XmlIssue};

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

    /// 按属性全名精确匹配，区分大小写。XML 的属性名本来就区分大小写，
    /// OOXML 的拼写由 schema 定死，所以这里不做任何模糊。
    /// 前缀可变的属性（`w:val`、`w14:paraId`、`r:id`）用 [`Element::attr_local`]。
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find_map(|(k, v)| (k == name).then_some(v.as_str()))
    }

    /// 只比 local name，忽略前缀，区分大小写。给前缀可变的属性用：
    /// `r:id` 的前缀只是习惯，任何映射到 relationships 命名空间的前缀都合法。
    ///
    /// 同一元素上既有无前缀又有带前缀的同名属性时（例如 `p:sldId` 同时带 `id` 与 `r:id`），
    /// 本方法命中先出现的那个，这种场合必须改用精确名。
    pub fn attr_local(&self, local: &str) -> Option<&str> {
        self.attrs.iter().find_map(|(k, v)| {
            let key_local = k.rsplit(':').next().unwrap_or(k);
            (key_local == local).then_some(v.as_str())
        })
    }

    /// 按属性全名精确匹配；有则改值，无则追加。
    pub fn set_attr(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        if let Some((_, v)) = self.attrs.iter_mut().find(|(k, _)| k == name) {
            *v = value;
            return;
        }
        self.attrs.push((name.to_string(), value));
    }

    /// 按属性全名精确匹配删除。要删 `xml:space` 就写全名，别指望写 `space` 也能删掉。
    pub fn remove_attr(&mut self, name: &str) {
        self.attrs.retain(|(k, _)| k != name);
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
                    .ok_or_else(|| Error::xml(XmlIssue::UnbalancedEnd))?;
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
                        Error::xml_detail(XmlIssue::TextDecode, e)
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
                return Err(Error::xml_detail(XmlIssue::Parse, e));
            }
        }
        buf.clear();
    }

    let root = root.ok_or_else(|| Error::xml(XmlIssue::NoRoot))?;
    Ok(Document { decl, root })
}

fn start_to_element(e: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<Element, Error> {
    let name = qname_to_string(e.name().as_ref());
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        let attr = attr
            .map_err(|err| Error::xml_detail(XmlIssue::AttrRead, err))?;
        let key = qname_to_string(attr.key.as_ref());
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(|err| Error::xml_detail(XmlIssue::AttrDecode, err))?
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
            .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
        return Ok(());
    }
    writer
        .write_event(Event::Start(start))
        .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
    for child in &el.children {
        match child {
            Node::Text(t) => {
                writer
                    .write_event(Event::Text(BytesText::new(t)))
                    .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
            }
            Node::Element(e) => write_element(writer, e)?,
        }
    }
    writer
        .write_event(Event::End(BytesEnd::new(el.name.as_str())))
        .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
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

    /// `p:sldId` 是最容易踩的一个：无前缀的 `id` 是幻灯片编号，`r:id` 是关系，
    /// local name 都叫 `id`。精确名必须能把两者分开。
    fn sld_id() -> Element {
        let doc = parse(br#"<p:sldId id="256" r:id="rId2"/>"#).unwrap();
        doc.root
    }

    #[test]
    fn attr_matches_full_name_only() {
        let el = sld_id();
        assert_eq!(el.attr("id"), Some("256"));
        assert_eq!(el.attr("r:id"), Some("rId2"));
    }

    #[test]
    fn attr_is_case_sensitive() {
        let el = sld_id();
        assert_eq!(el.attr("ID"), None);
        assert_eq!(el.attr("Id"), None);
    }

    #[test]
    fn attr_local_ignores_prefix() {
        let doc = parse(br#"<w:p w14:paraId="0A1B" w:rsidR="00FF"/>"#).unwrap();
        assert_eq!(doc.root.attr_local("paraId"), Some("0A1B"));
        assert_eq!(doc.root.attr("paraId"), None);
    }

    #[test]
    fn set_attr_does_not_hijack_prefixed_twin() {
        let mut el = sld_id();
        el.set_attr("id", "257");
        assert_eq!(el.attr("id"), Some("257"));
        assert_eq!(el.attr("r:id"), Some("rId2"));
    }

    #[test]
    fn remove_attr_spares_prefixed_twin() {
        let mut el = sld_id();
        el.remove_attr("id");
        assert_eq!(el.attr("id"), None);
        assert_eq!(el.attr("r:id"), Some("rId2"));
    }

    #[test]
    fn remove_attr_needs_the_prefix_spelled_out() {
        let doc = parse(br#"<a:t xml:space="preserve"> x </a:t>"#).unwrap();
        let mut el = doc.root;
        el.remove_attr("space");
        assert_eq!(el.attr("xml:space"), Some("preserve"));
        el.remove_attr("xml:space");
        assert_eq!(el.attr("xml:space"), None);
    }
}
