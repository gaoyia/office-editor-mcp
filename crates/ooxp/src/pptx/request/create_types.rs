//! create 解析出来的中间结构。

use super::super::geom;
use super::super::ink;
use super::super::model3d;
use super::super::ole;
use super::super::omml;
use super::super::zoom;
use super::{
    CreateAnim, CreateChart, CreateConnector, CreateField, CreateHyperlink, CreatePicture,
    CreateSmartArt, CreateTable, InsertCommentOp, InsertMediaOp, SetTransitionOp,
};

pub(crate) struct NewShape {
    pub(crate) text: String,
    pub(crate) link: Option<CreateHyperlink>,
    pub(crate) format: Option<crate::drawing_text::TextFormat>,
    pub(crate) geom: Option<Vec<geom::Path>>,
    pub(crate) fields: Vec<CreateField>,
}

pub(crate) struct NewNotes {
    pub(crate) text: String,
    pub(crate) anims: Vec<CreateAnim>,
    pub(crate) maths: Vec<omml::InsertMathOp>,
    pub(crate) model3ds: Vec<model3d::InsertModel3dOp>,
    pub(crate) inks: Vec<ink::InsertInkOp>,
}

pub(crate) struct NewSlide {
    pub(crate) shapes: Vec<NewShape>,
    pub(crate) pictures: Vec<CreatePicture>,
    pub(crate) videos: Vec<InsertMediaOp>,
    pub(crate) audios: Vec<InsertMediaOp>,
    pub(crate) charts: Vec<CreateChart>,
    pub(crate) smart_arts: Vec<CreateSmartArt>,
    pub(crate) tables: Vec<CreateTable>,
    pub(crate) notes: Option<NewNotes>,
    pub(crate) comments: Vec<InsertCommentOp>,
    pub(crate) transition: Option<SetTransitionOp>,
    pub(crate) anims: Vec<CreateAnim>,
    pub(crate) model3ds: Vec<model3d::InsertModel3dOp>,
    pub(crate) inks: Vec<ink::InsertInkOp>,
    pub(crate) maths: Vec<omml::InsertMathOp>,
    pub(crate) ole_objects: Vec<ole::InsertOleOp>,
    pub(crate) connectors: Vec<CreateConnector>,
    pub(crate) zooms: Vec<zoom::InsertZoomOp>,
}
