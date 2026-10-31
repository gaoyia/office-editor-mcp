//! create 解析出来的中间结构。

use super::super::a1::CellRef;
use super::super::ink;
use super::super::model3d;
use super::super::omml;
use super::{
    HfSlot, InsertChartOp, InsertChartSheetOp, InsertCommentOp, InsertConnectorOp,
    InsertHyperlinkOp, InsertPictureOp, InsertShapeOp, InsertSmartArtOp, InsertTextBoxOp,
    SetCellValue,
};

pub(crate) struct NewCell {
    pub(crate) refer: CellRef,
    pub(crate) a1: String,
    pub(crate) value: Option<SetCellValue>,
    pub(crate) formula: Option<String>,
}

pub(crate) struct NewHfSlot {
    pub(crate) slot: HfSlot,
    pub(crate) text: Option<String>,
    pub(crate) pictures: Vec<InsertPictureOp>,
}

pub(crate) struct NewSheet {
    pub(crate) name: String,
    pub(crate) chartsheet: Option<InsertChartSheetOp>,
    pub(crate) cells: Vec<NewCell>,
    pub(crate) pictures: Vec<InsertPictureOp>,
    pub(crate) charts: Vec<InsertChartOp>,
    pub(crate) header_footers: Vec<NewHfSlot>,
    pub(crate) comments: Vec<InsertCommentOp>,
    pub(crate) hyperlinks: Vec<InsertHyperlinkOp>,
    pub(crate) text_boxes: Vec<InsertTextBoxOp>,
    pub(crate) shapes: Vec<InsertShapeOp>,
    pub(crate) connectors: Vec<InsertConnectorOp>,
    pub(crate) smartarts: Vec<InsertSmartArtOp>,
    pub(crate) model3ds: Vec<model3d::InsertModel3dOp>,
    pub(crate) inks: Vec<ink::InsertInkOp>,
    pub(crate) maths: Vec<omml::InsertMathOp>,
}
