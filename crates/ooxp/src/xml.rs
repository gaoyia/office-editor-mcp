//! 小型 XML 树：给 OPC 关系文件和 SpreadsheetML 部件做手术刀式改写。

use std::io::Cursor;

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::error::{Error, ErrorKind, XmlIssue};

#[derive(Clone, Debug)]
pub struct Document {
    pub decl: Option<String>,
    /// 根元素之前的注释和处理指令，写回时原样接在声明后面。
    pub prefix: Vec<String>,
    pub root: Element,
    /// 根元素之后的注释和处理指令。
    pub suffix: Vec<String>,
}

impl Document {
    pub fn new(decl: Option<String>, root: Element) -> Self {
        Self {
            decl,
            prefix: Vec::new(),
            root,
            suffix: Vec::new(),
        }
    }

    /// 把根交出去。空壳留给 `Drop`，由 `dismantle` 在堆上拆掉。
    pub fn take_root(mut self) -> Element {
        std::mem::replace(&mut self.root, Element::new("empty"))
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        let root = std::mem::replace(&mut self.root, Element::new("empty"));
        dismantle(root);
    }
}

/// 在堆上拆掉子元素，使这一棵树离开作用域时不再按层数递归析构。
pub(crate) fn dismantle(mut el: Element) {
    let mut stack = vec![std::mem::take(&mut el.children)];
    while let Some(children) = stack.pop() {
        for node in children {
            if let Node::Element(mut child) = node {
                stack.push(std::mem::take(&mut child.children));
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Element(Element),
    Text(String),
    /// 注释或处理指令的完整标记，例如 `<!--note-->`、`<?mso-info ?>`。
    Raw(String),
}

#[derive(Clone, Debug)]
pub struct Element {
    /// 带前缀的名字，例如 `r:id` 用在属性上；元素则是 `workbook` 或 `r:id` 这种写成 `sheet`。
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Drop for Element {
    fn drop(&mut self) {
        if self.children.is_empty() {
            return;
        }
        let mut stack = vec![std::mem::take(&mut self.children)];
        while let Some(children) = stack.pop() {
            for node in children {
                if let Node::Element(mut child) = node {
                    if !child.children.is_empty() {
                        stack.push(std::mem::take(&mut child.children));
                    }
                }
            }
        }
    }
}

impl Element {
    pub fn take_children(&mut self) -> Vec<Node> {
        std::mem::take(&mut self.children)
    }

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
            Node::Text(_) | Node::Raw(_) => None,
        })
    }

    pub fn element_children_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.children.iter_mut().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) | Node::Raw(_) => None,
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

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.set_text(text);
        self
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
    let mut stack: Vec<&Node> = el.children.iter().rev().collect();
    while let Some(node) = stack.pop() {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Raw(_) => {}
            Node::Element(child) => {
                for inner in child.children.iter().rev() {
                    stack.push(inner);
                }
            }
        }
    }
}

/// 先序走过元素及其子元素。栈在堆上。`enter` 返回 false 时不走进该元素的子节点，也不再对其调用方做额外处理。
pub(crate) fn walk_elements_if<'a>(root: &'a Element, mut enter: impl FnMut(&'a Element) -> bool) {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if !enter(el) {
            continue;
        }
        for child in el.children.iter().rev() {
            if let Node::Element(inner) = child {
                stack.push(inner);
            }
        }
    }
}

/// 先序走过元素及其子元素。栈在堆上。
pub(crate) fn walk_elements<'a>(root: &'a Element, f: &mut impl FnMut(&'a Element)) {
    walk_elements_if(root, |el| {
        f(el);
        true
    });
}

/// 先序查找第一个 local name 相符的元素。栈在堆上。
pub(crate) fn find_by_local<'a>(root: &'a Element, local: &str) -> Option<&'a Element> {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        if el.local_name() == local {
            return Some(el);
        }
        for child in el.children.iter().rev() {
            if let Node::Element(inner) = child {
                stack.push(inner);
            }
        }
    }
    None
}

/// 先序查找，命中后返回可变引用。栈在堆上。
pub(crate) fn find_by_local_mut<'a>(root: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    if root.local_name() == local {
        return Some(root);
    }
    let mut stack: Vec<Vec<usize>> = Vec::new();
    push_element_children(root, &[], &mut stack);
    while let Some(path) = stack.pop() {
        if element_at_ref(root, &path).local_name() == local {
            return Some(element_at_mut(root, &path));
        }
        push_element_children(root, &path, &mut stack);
    }
    None
}

/// 不可变先序查找的一步。`Kids` 按从左到右。
pub(crate) enum Search<'a, T> {
    Hit(T),
    Skip,
    Only(&'a Element),
    Kids(Vec<&'a Element>),
}

impl<'a, T> Search<'a, T> {
    pub(crate) fn down(el: &'a Element) -> Self {
        Search::Kids(el.element_children().collect())
    }
}

/// 可变先序查找的一步。下标是 `children` 里的位置。`Kids` 按从左到右。
pub(crate) enum SearchMut {
    Hit,
    HitChild(usize),
    Skip,
    Only(usize),
    Kids(Vec<usize>),
}

impl SearchMut {
    pub(crate) fn down(el: &Element) -> Self {
        SearchMut::Kids(element_indexes(el))
    }
}

pub(crate) fn element_indexes(el: &Element) -> Vec<usize> {
    el.children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| matches!(node, Node::Element(_)).then_some(index))
        .collect()
}

/// 先序查找。栈在堆上。
pub(crate) fn search<'a, T>(
    root: &'a Element,
    mut step: impl FnMut(&'a Element) -> Search<'a, T>,
) -> Option<T> {
    let mut stack = vec![root];
    while let Some(el) = stack.pop() {
        match step(el) {
            Search::Hit(value) => return Some(value),
            Search::Skip => {}
            Search::Only(child) => stack.push(child),
            Search::Kids(children) => {
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }
        }
    }
    None
}

/// 先序查找，命中后返回可变引用。栈在堆上。
pub(crate) fn search_mut(
    root: &mut Element,
    mut step: impl FnMut(&Element) -> SearchMut,
) -> Option<&mut Element> {
    let mut stack: Vec<Vec<usize>> = vec![Vec::new()];
    while let Some(path) = stack.pop() {
        let decision = step(element_at_ref(root, &path));
        match decision {
            SearchMut::Hit => return Some(element_at_mut(root, &path)),
            SearchMut::HitChild(index) => {
                let mut child = path;
                child.push(index);
                return Some(element_at_mut(root, &child));
            }
            SearchMut::Skip => {}
            SearchMut::Only(index) => {
                let mut child = path;
                child.push(index);
                stack.push(child);
            }
            SearchMut::Kids(indexes) => {
                for index in indexes.into_iter().rev() {
                    let mut child = path.clone();
                    child.push(index);
                    stack.push(child);
                }
            }
        }
    }
    None
}

pub(crate) fn element_at_ref<'a>(root: &'a Element, path: &[usize]) -> &'a Element {
    let mut cur = root;
    for &index in path {
        cur = match &cur.children[index] {
            Node::Element(child) => child,
            Node::Text(_) | Node::Raw(_) => unreachable!("路径只指向元素"),
        };
    }
    cur
}

fn push_element_children(root: &Element, path: &[usize], stack: &mut Vec<Vec<usize>>) {
    let indexes: Vec<usize> = element_at_ref(root, path)
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| matches!(node, Node::Element(_)).then_some(index))
        .collect();
    for index in indexes.into_iter().rev() {
        let mut child = path.to_vec();
        child.push(index);
        stack.push(child);
    }
}

pub(crate) fn element_at_mut<'a>(root: &'a mut Element, path: &[usize]) -> &'a mut Element {
    let mut cur = root;
    for &index in path {
        cur = match &mut cur.children[index] {
            Node::Element(child) => child,
            Node::Text(_) | Node::Raw(_) => unreachable!("路径只指向元素"),
        };
    }
    cur
}

pub fn parse(bytes: &[u8]) -> Result<Document, Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    let mut decl: Option<String> = None;
    let mut prefix = Vec::new();
    let mut suffix = Vec::new();

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
                    .map_err(|e| Error::xml_detail(XmlIssue::TextDecode, e))?
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
            Ok(Event::Comment(body)) => {
                let markup = format!("<!--{}-->", String::from_utf8_lossy(&body));
                push_raw(&mut stack, root.is_some(), &mut prefix, &mut suffix, markup);
            }
            Ok(Event::PI(body)) => {
                let markup = format!("<?{}?>", String::from_utf8_lossy(&body));
                push_raw(&mut stack, root.is_some(), &mut prefix, &mut suffix, markup);
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::xml_detail(XmlIssue::Parse, "DOCTYPE"));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::xml_detail(XmlIssue::Parse, e));
            }
        }
        buf.clear();
    }

    let root = root.ok_or_else(|| Error::xml(XmlIssue::NoRoot))?;
    let mut doc = Document::new(decl, root);
    doc.prefix = prefix;
    doc.suffix = suffix;
    Ok(doc)
}

fn push_raw(
    stack: &mut [Element],
    have_root: bool,
    prefix: &mut Vec<String>,
    suffix: &mut Vec<String>,
    markup: String,
) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(Node::Raw(markup));
    } else if have_root {
        suffix.push(markup);
    } else {
        prefix.push(markup);
    }
}

fn start_to_element(e: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<Element, Error> {
    let name = qname_to_string(e.name().as_ref());
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|err| Error::xml_detail(XmlIssue::AttrRead, err))?;
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
        for raw in &self.prefix {
            out.extend_from_slice(raw.as_bytes());
        }
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        write_element(&mut writer, &self.root)?;
        out.extend_from_slice(&writer.into_inner().into_inner());
        for raw in &self.suffix {
            out.extend_from_slice(raw.as_bytes());
        }
        Ok(out)
    }
}

fn write_element(writer: &mut Writer<Cursor<Vec<u8>>>, el: &Element) -> Result<(), Error> {
    enum Frame<'a> {
        Start(&'a Element),
        End(&'a Element),
        Text(&'a str),
        Raw(&'a str),
    }
    let mut stack = vec![Frame::Start(el)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Raw(raw) => {
                let cursor = writer.get_mut();
                cursor.get_mut().extend_from_slice(raw.as_bytes());
                let len = cursor.get_ref().len() as u64;
                cursor.set_position(len);
            }
            Frame::Text(text) => {
                let escaped = escape_text_node(text);
                writer
                    .write_event(Event::Text(BytesText::from_escaped(escaped.as_str())))
                    .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
            }
            Frame::End(node) => {
                writer
                    .write_event(Event::End(BytesEnd::new(node.name.as_str())))
                    .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
            }
            Frame::Start(node) => {
                let mut start = BytesStart::new(node.name.as_str());
                let escaped_attrs: Vec<String> = node
                    .attrs
                    .iter()
                    .map(|(_, value)| escape_attr_node(value))
                    .collect();
                for ((key, _), value) in node.attrs.iter().zip(escaped_attrs.iter()) {
                    // 段落序号只活在这次 apply 的内存里，不能写进文件。
                    if key == "ooxp:pin" {
                        continue;
                    }
                    // 字节形式不再转义。`(&str, &str)` 会把 `&#10;` 里的 `&` 再转成 `&amp;`。
                    start.push_attribute(Attribute::from((key.as_bytes(), value.as_bytes())));
                }
                if node.children.is_empty() {
                    writer
                        .write_event(Event::Empty(start))
                        .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
                    continue;
                }
                writer
                    .write_event(Event::Start(start))
                    .map_err(|e| Error::xml_detail(XmlIssue::Write, e))?;
                stack.push(Frame::End(node));
                for child in node.children.iter().rev() {
                    match child {
                        Node::Text(text) => stack.push(Frame::Text(text)),
                        Node::Raw(raw) => stack.push(Frame::Raw(raw)),
                        Node::Element(inner) => stack.push(Frame::Start(inner)),
                    }
                }
            }
        }
    }
    Ok(())
}

/// 进入 `AlternateContent` 的哪一支。下标指向 `Choice` 或 `Fallback`。
/// 没有可进的支时返回 `Ok(None)`。`Requires` 拆不开时返回 `XmlBroken`。
pub(crate) fn alternate_child(el: &Element) -> Result<Option<usize>, Error> {
    let mut fallback = None;
    for (index, node) in el.children.iter().enumerate() {
        let Node::Element(child) = node else {
            continue;
        };
        match child.local_name() {
            "Choice" => {
                if choice_understood(el, child)? {
                    return Ok(Some(index));
                }
            }
            "Fallback" if fallback.is_none() => fallback = Some(index),
            _ => {}
        }
    }
    Ok(fallback)
}

fn choice_understood(ac: &Element, choice: &Element) -> Result<bool, Error> {
    let Some(raw) = choice.attr_local("Requires") else {
        return Err(alternate_broken());
    };
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(alternate_broken());
    }
    for token in &tokens {
        if !is_prefix_token(token) {
            return Err(alternate_broken());
        }
        bound_ok(ac, choice, token)?;
    }
    Ok(tokens.iter().all(|token| prefix_understood(token)))
}

fn is_prefix_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn bound_ok(ac: &Element, choice: &Element, prefix: &str) -> Result<(), Error> {
    for el in [choice, ac] {
        if let Some(uri) = xmlns_uri(el, prefix) {
            if uri.is_empty() {
                return Err(alternate_broken());
            }
            return Ok(());
        }
    }
    Ok(())
}

fn xmlns_uri<'a>(el: &'a Element, prefix: &str) -> Option<&'a str> {
    el.attrs.iter().find_map(|(key, value)| {
        let rest = key.strip_prefix("xmlns:")?;
        (rest == prefix).then_some(value.as_str())
    })
}

fn prefix_understood(prefix: &str) -> bool {
    matches!(
        prefix,
        "a14"
            | "am3d"
            | "p14"
            | "p15"
            | "pslz"
            | "psez"
            | "psuz"
            | "sle15"
            | "tsle"
            | "v"
            | "wpc"
            | "wpg"
            | "wps"
            | "x14"
            | "xdr14"
    )
}

fn alternate_broken() -> Error {
    Error::new(ErrorKind::XmlBroken {
        issue: XmlIssue::Parse,
        detail: Some("AlternateContent".into()),
    })
}

struct EscapeSet {
    gt: bool,
    apos: bool,
    quot: bool,
    tab: bool,
    lf: bool,
    cr: bool,
}

fn escape_chars(s: &str, set: EscapeSet) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' if set.gt => out.push_str("&gt;"),
            '\'' if set.apos => out.push_str("&apos;"),
            '"' if set.quot => out.push_str("&quot;"),
            '\t' if set.tab => out.push_str("&#9;"),
            '\n' if set.lf => out.push_str("&#10;"),
            '\r' if set.cr => out.push_str("&#13;"),
            other => out.push(other),
        }
    }
    out
}

/// 元素树属性。与 quick-xml 的 `escape` 同一套五个字符，再补上会被属性值规整掉的空白。
fn escape_attr_node(s: &str) -> String {
    escape_chars(
        s,
        EscapeSet {
            gt: true,
            apos: true,
            quot: true,
            tab: true,
            lf: true,
            cr: true,
        },
    )
}

/// 元素树文本。与 `BytesText::new` 同一套五个字符，再把 CR 写成字符引用。
fn escape_text_node(s: &str) -> String {
    escape_chars(
        s,
        EscapeSet {
            gt: true,
            apos: true,
            quot: true,
            tab: false,
            lf: false,
            cr: true,
        },
    )
}

pub fn escape_attr(s: &str) -> String {
    escape_chars(
        s,
        EscapeSet {
            gt: true,
            apos: false,
            quot: true,
            tab: true,
            lf: true,
            cr: true,
        },
    )
}

pub fn escape_text(s: &str) -> String {
    escape_chars(
        s,
        EscapeSet {
            gt: true,
            apos: false,
            quot: false,
            tab: false,
            lf: false,
            cr: true,
        },
    )
}

/// `1` / `true` / `TRUE` / `on`。不裁空白。
pub fn attr_is_true(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name),
        Some("1") | Some("true") | Some("TRUE") | Some("on")
    )
}

/// 先裁空白，再按 `attr_is_true` 认。
pub fn attr_is_true_trimmed(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name).map(str::trim),
        Some("1" | "true" | "TRUE" | "on")
    )
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
        doc.take_root()
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
        let mut el = doc.take_root();
        el.remove_attr("space");
        assert_eq!(el.attr("xml:space"), Some("preserve"));
        el.remove_attr("xml:space");
        assert_eq!(el.attr("xml:space"), None);
    }

    #[test]
    fn whitespace_roundtrips_as_char_refs() {
        let src = br#"<a x="1&#10;2&#9;3&#13;4">a&#13;b</a>"#;
        let out = String::from_utf8(parse(src).unwrap().to_bytes().unwrap()).unwrap();
        assert!(out.contains(r#"x="1&#10;2&#9;3&#13;4""#), "{out}");
        assert!(out.contains(">a&#13;b<"), "{out}");
        assert!(!out.contains('\n'), "{out}");
        assert!(!out.contains('\t'), "{out}");
        assert!(!out.contains('\r'), "{out}");
    }

    #[test]
    fn escape_helpers_keep_their_character_sets() {
        assert_eq!(escape_attr("a\tb\nc\rd\"'"), "a&#9;b&#10;c&#13;d&quot;'");
        assert_eq!(escape_text("a\tb\nc\rd<"), "a\tb\nc&#13;d&lt;");
        assert!(!escape_text("a\nb").contains("&#10;"));
        assert!(!escape_attr("it's").contains("&apos;"));
    }

    #[test]
    fn text_ampersand_roundtrips_escaped() {
        let doc = parse(b"<oddHeader>&amp;Cfoo</oddHeader>").unwrap();
        assert_eq!(doc.root.text_content(), "&Cfoo");
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("&amp;Cfoo"), "{out}");
        assert!(!out.contains("&amp;amp;"), "{out}");
    }

    #[test]
    fn deep_tree_walks_on_the_heap() {
        let mut el = Element::new("a").with_text("z");
        for _ in 0..8_000 {
            let mut parent = Element::new("a");
            parent.push_element(el);
            el = parent;
        }
        let doc = Document::new(None, el);
        assert_eq!(doc.root.text_content(), "z");
        let mut seen = 0u32;
        walk_elements(&doc.root, &mut |_| seen += 1);
        assert_eq!(seen, 8_001);
        assert!(find_by_local(&doc.root, "missing").is_none());
        let bytes = doc.to_bytes().unwrap();
        assert!(bytes.windows(8).any(|window| window == b"<a>z</a>"));
    }

    fn branch(reqs: &[&str]) -> Element {
        let mut ac = Element::new("mc:AlternateContent");
        for req in reqs {
            let mut choice = Element::new("mc:Choice").with_attr("Requires", *req);
            choice.push_element(Element::new(*req));
            ac.push_element(choice);
        }
        let mut fallback = Element::new("mc:Fallback");
        fallback.push_element(Element::new("poster"));
        ac.push_element(fallback);
        ac
    }

    fn picked(el: &Element) -> &str {
        let index = alternate_child(el).unwrap().unwrap();
        match &el.children[index] {
            Node::Element(child) => child.local_name(),
            Node::Text(_) | Node::Raw(_) => "",
        }
    }

    #[test]
    fn understood_prefix_enters_choice() {
        let el = branch(&["a14"]);
        assert_eq!(picked(&el), "Choice");
    }

    #[test]
    fn unknown_prefix_enters_fallback() {
        let el = branch(&["w99"]);
        assert_eq!(picked(&el), "Fallback");
    }

    #[test]
    fn multiple_choices_take_the_first_fully_understood() {
        let el = branch(&["w99", "a14 wps", "p14"]);
        let index = alternate_child(&el).unwrap().unwrap();
        match &el.children[index] {
            Node::Element(child) => {
                assert_eq!(child.local_name(), "Choice");
                assert_eq!(child.attr_local("Requires"), Some("a14 wps"));
            }
            Node::Text(_) | Node::Raw(_) => panic!("text"),
        }
    }

    #[test]
    fn one_unknown_token_rejects_that_choice() {
        let el = branch(&["a14 w99"]);
        assert_eq!(picked(&el), "Fallback");
    }

    #[test]
    fn unknown_without_fallback_skips() {
        let mut choice = Element::new("mc:Choice").with_attr("Requires", "w99");
        choice.push_element(Element::new("new"));
        let mut ac = Element::new("mc:AlternateContent");
        ac.push_element(choice);
        assert!(alternate_child(&ac).unwrap().is_none());
    }

    #[test]
    fn missing_requires_is_xml_broken() {
        let mut choice = Element::new("mc:Choice");
        choice.push_element(Element::new("x"));
        let mut ac = Element::new("mc:AlternateContent");
        ac.push_element(choice);
        let err = alternate_child(&ac).unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::XmlBroken);
    }

    #[test]
    fn empty_xmlns_is_xml_broken() {
        let mut choice = Element::new("mc:Choice")
            .with_attr("Requires", "a14")
            .with_attr("xmlns:a14", "");
        choice.push_element(Element::new("math"));
        let mut ac = Element::new("mc:AlternateContent");
        ac.push_element(choice);
        let err = alternate_child(&ac).unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::XmlBroken);
    }

    #[test]
    fn comment_and_pi_survive_a_rewrite() {
        let src = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><!--keep--><?mso-info yes?><a x="1"><b>t</b><!--in--></a>"#;
        let mut doc = parse(src).unwrap();
        doc.root.set_attr("x", "2");
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("<!--keep-->"), "{out}");
        assert!(out.contains("<?mso-info yes?>"), "{out}");
        assert!(out.contains("<!--in-->"), "{out}");
        assert!(out.contains(r#"x="2""#), "{out}");
        assert!(
            !out.contains("<!--keep-->")
                || out.find("<!--keep-->").unwrap() < out.find("<a").unwrap()
        );
    }

    #[test]
    fn doctype_is_rejected() {
        let err = parse(br#"<?xml version="1.0"?><!DOCTYPE a [<!ENTITY x "y">]><a/>"#).unwrap_err();
        assert_eq!(err.code(), crate::error::ErrorCode::XmlBroken);
    }

    #[test]
    fn deep_take_root_does_not_overflow() {
        let mut el = Element::new("a");
        for _ in 0..8_000 {
            let mut parent = Element::new("a");
            parent.push_element(el);
            el = parent;
        }
        let doc = Document::new(None, el);
        let root = doc.take_root();
        drop(root);
    }
}
