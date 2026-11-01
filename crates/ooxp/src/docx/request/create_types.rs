//! create 解析出来的中间结构。

use super::super::charts::{ChartType, NewSeries};
use super::super::field;
use super::super::hf::HfType;
use super::super::math::MathKind;
use super::super::math_expr::MathBody;
use super::super::picture::{PicKind, WrapKind, WrapPos};
use super::super::sdt::SdtKind;
use super::super::sect::SectionBreakType;
use super::super::smartart::InsertSpec;
use super::doc_extra::{
    CitationSpec, CrossRefSpec, DrawingHlink, EditRangeSpec, IndexEntrySpec, RubySpec, ShapeSpec,
    TableExtra,
};
use super::para_remain::{GroupSpec, ParaRemain, RunSpec};

pub(crate) struct NewBox {
    pub(crate) text: String,
    pub(crate) cx: i64,
    pub(crate) cy: i64,
    pub(crate) doc_pr: u32,
    pub(crate) inner_para: u32,
    pub(crate) hyperlink: Option<DrawingHlink>,
    pub(crate) tables: Vec<NewTable>,
    pub(crate) hyperlinks: Vec<NewHyperlink>,
    pub(crate) bookmarks: Vec<NewBookmark>,
    pub(crate) fields: Vec<NewField>,
    pub(crate) maths: Vec<NewMath>,
    pub(crate) content_controls: Vec<NewSdt>,
    pub(crate) rubies: Vec<RubySpec>,
}

impl NewBox {
    pub(crate) fn has_follow_ups(&self) -> bool {
        self.hyperlink.is_some()
            || !self.tables.is_empty()
            || !self.hyperlinks.is_empty()
            || !self.bookmarks.is_empty()
            || !self.fields.is_empty()
            || !self.maths.is_empty()
            || !self.content_controls.is_empty()
            || !self.rubies.is_empty()
    }
}

pub(crate) struct NewPic {
    pub(crate) file: String,
    pub(crate) cx: Option<i64>,
    pub(crate) cy: Option<i64>,
    pub(crate) kind: PicKind,
    pub(crate) wrap: Option<WrapKind>,
    pub(crate) pos: WrapPos,
    pub(crate) doc_pr: u32,
    pub(crate) hyperlink: Option<DrawingHlink>,
}

pub(crate) struct CreateSmartArt {
    pub(crate) spec: InsertSpec,
    pub(crate) hyperlink: Option<DrawingHlink>,
}

pub(crate) struct CreateChart {
    pub(crate) chart_type: ChartType,
    pub(crate) of_pie_type: Option<String>,
    pub(crate) extra: crate::chart::ChartExtra,
    pub(crate) title: Option<String>,
    pub(crate) series: Vec<NewSeries>,
    pub(crate) kind: PicKind,
    pub(crate) wrap: Option<WrapKind>,
    pub(crate) cx: i64,
    pub(crate) cy: i64,
    pub(crate) hyperlink: Option<DrawingHlink>,
}

pub(crate) struct NewComment {
    pub(crate) text: String,
    pub(crate) author: String,
    pub(crate) initials: String,
    pub(crate) date: Option<String>,
    pub(crate) replies: Vec<NewComment>,
}

pub(crate) struct NewHyperlink {
    pub(crate) text: String,
    pub(crate) url: Option<String>,
    pub(crate) anchor: Option<String>,
    pub(crate) tooltip: Option<String>,
}

pub(crate) struct NewBookmark {
    pub(crate) name: Option<String>,
    pub(crate) end: Option<u32>,
}

pub(crate) struct NewField {
    pub(crate) instr: String,
    pub(crate) result: Option<String>,
    pub(crate) form: Option<field::FormWrite>,
}

pub(crate) struct NewMath {
    pub(crate) body: MathBody,
    pub(crate) kind: MathKind,
    pub(crate) jc: Option<String>,
}

pub(crate) struct NewSdt {
    pub(crate) text: String,
    pub(crate) kind: SdtKind,
    pub(crate) tag: Option<String>,
    pub(crate) alias: Option<String>,
    pub(crate) items: Vec<String>,
    pub(crate) full_date: Option<String>,
    pub(crate) file: Option<String>,
    pub(crate) checked: bool,
}

pub(crate) struct NewToc {
    pub(crate) after: Option<u32>,
    pub(crate) instr: String,
}

pub(crate) struct NewInkCanvas {
    pub(crate) file: String,
    pub(crate) cx: Option<i64>,
    pub(crate) cy: Option<i64>,
    pub(crate) hyperlink: Option<DrawingHlink>,
}

pub(crate) struct NewModel3d {
    pub(crate) file: String,
    pub(crate) cx: Option<i64>,
    pub(crate) cy: Option<i64>,
    pub(crate) kind: super::super::picture::PicKind,
    pub(crate) wrap: Option<super::super::picture::WrapKind>,
    pub(crate) pos: super::super::picture::WrapPos,
    pub(crate) hyperlink: Option<DrawingHlink>,
}

pub(crate) struct NewPara {
    pub(crate) text: String,
    pub(crate) style: Option<&'static str>,
    pub(crate) boxes: Vec<NewBox>,
    pub(crate) pictures: Vec<NewPic>,
    pub(crate) smart_arts: Vec<CreateSmartArt>,
    pub(crate) charts: Vec<CreateChart>,
    pub(crate) ole_objects: Vec<String>,
    pub(crate) ink_canvases: Vec<NewInkCanvas>,
    pub(crate) model3ds: Vec<NewModel3d>,
    pub(crate) footnote: Option<String>,
    pub(crate) endnote: Option<String>,
    pub(crate) comments: Vec<NewComment>,
    pub(crate) hyperlinks: Vec<NewHyperlink>,
    pub(crate) bookmarks: Vec<NewBookmark>,
    pub(crate) fields: Vec<NewField>,
    pub(crate) maths: Vec<NewMath>,
    pub(crate) content_controls: Vec<NewSdt>,
    pub(crate) shapes: Vec<ShapeSpec>,
    pub(crate) connectors: Vec<ShapeSpec>,
    pub(crate) rubies: Vec<RubySpec>,
    pub(crate) citations: Vec<CitationSpec>,
    pub(crate) index_entries: Vec<IndexEntrySpec>,
    pub(crate) cross_refs: Vec<CrossRefSpec>,
    pub(crate) edit_range: Option<EditRangeSpec>,
    pub(crate) num: Option<u32>,
    pub(crate) ilvl: u32,
    pub(crate) section_type: Option<SectionBreakType>,
    pub(crate) remain: ParaRemain,
    pub(crate) runs: Option<Vec<RunSpec>>,
}

impl NewPara {
    pub(crate) fn empty() -> Self {
        Self {
            text: String::new(),
            style: None,
            boxes: Vec::new(),
            pictures: Vec::new(),
            smart_arts: Vec::new(),
            charts: Vec::new(),
            ole_objects: Vec::new(),
            ink_canvases: Vec::new(),
            model3ds: Vec::new(),
            footnote: None,
            endnote: None,
            comments: Vec::new(),
            hyperlinks: Vec::new(),
            bookmarks: Vec::new(),
            fields: Vec::new(),
            maths: Vec::new(),
            content_controls: Vec::new(),
            shapes: Vec::new(),
            connectors: Vec::new(),
            rubies: Vec::new(),
            citations: Vec::new(),
            index_entries: Vec::new(),
            cross_refs: Vec::new(),
            edit_range: None,
            num: None,
            ilvl: 0,
            section_type: None,
            remain: ParaRemain::empty(),
            runs: None,
        }
    }

    pub(crate) fn has_pictures(&self) -> bool {
        !self.pictures.is_empty()
    }

    pub(crate) fn has_smartarts(&self) -> bool {
        !self.smart_arts.is_empty()
    }

    pub(crate) fn has_follow_ups(&self) -> bool {
        !self.charts.is_empty()
            || !self.ole_objects.is_empty()
            || !self.ink_canvases.is_empty()
            || !self.model3ds.is_empty()
            || self.footnote.is_some()
            || self.endnote.is_some()
            || !self.comments.is_empty()
            || !self.hyperlinks.is_empty()
            || !self.bookmarks.is_empty()
            || !self.fields.is_empty()
            || !self.maths.is_empty()
            || !self.content_controls.is_empty()
            || !self.shapes.is_empty()
            || !self.connectors.is_empty()
            || !self.rubies.is_empty()
            || !self.citations.is_empty()
            || !self.index_entries.is_empty()
            || !self.cross_refs.is_empty()
            || self.edit_range.is_some()
            || self.num.is_some()
            || self.remain.has_follow_ups()
            || self
                .runs
                .as_ref()
                .is_some_and(|rs| rs.iter().any(|r| r.format.is_some()))
            || self.boxes.iter().any(|b| b.has_follow_ups())
            || self.pictures.iter().any(|p| p.hyperlink.is_some())
            || self.smart_arts.iter().any(|s| s.hyperlink.is_some())
            || self.charts.iter().any(|c| c.hyperlink.is_some())
            || self.ink_canvases.iter().any(|c| c.hyperlink.is_some())
            || self.model3ds.iter().any(|m| m.hyperlink.is_some())
            || self.shapes.iter().any(|s| s.hyperlink.is_some())
            || self.connectors.iter().any(|s| s.hyperlink.is_some())
    }
}

pub(crate) enum ParaKind {
    Body,
    HeaderFooter,
}

pub(crate) struct HfCreateItem {
    pub(crate) hf_type: HfType,
    pub(crate) paras: Vec<NewPara>,
    pub(crate) tables: Vec<NewTable>,
    pub(crate) groups: Vec<GroupSpec>,
}

pub(crate) struct NewTable {
    pub(crate) after: u32,
    pub(crate) cells: Vec<Vec<String>>,
    pub(crate) extra: TableExtra,
}
