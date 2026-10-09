//! 按 id 改单元格或工作表图，改单元格时标记打开时重算。

use super::formula;
use super::hyperlink;
use super::request::{Patch, SetCellOp, SetCellValue, SetProtectionOp, XlsxOp};
use super::sparkline;
use crate::error::{Error, ErrorKind, IdReason, RelSource};
use crate::opc::{self, ContentTypes, Package, Relationships};
use crate::xml::{self, Document, Element, Node};

use super::a1;
use super::chart;
use super::chartsheet;
use super::comments;
use super::connection;
use super::custom_view;
use super::extlink;
use super::hf;
use super::inspect;
use super::layout;
use super::model3d;
use super::ole;
use super::omml;
use super::picture;
use super::shape;
use super::sheets;
use super::slicer;
use super::smartart;
use super::style;
use super::REL_CALC_CHAIN;

pub fn apply(bytes: &[u8], ops: &[XlsxOp]) -> Result<Vec<u8>, Error> {
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    let wb_part = opc::office_document_part(&pkg)?;
    let mut wb_doc = xml::parse(pkg.get(&wb_part)?)?;
    let wb_rels = Relationships::from_package(&pkg, &wb_part)?;

    let mut sheet_index = sheet_index(&wb_doc.root, &wb_rels, &wb_part)?;

    let mut dirty_sheets: Vec<(String, Document)> = Vec::new();
    let mut styles_doc: Option<(String, Document)> = None;
    let mut touched_cells = false;
    let mut workbook_dirty = false;

    for (i, op) in ops.iter().enumerate() {
        match op {
            XlsxOp::SetCell(op) => {
                touched_cells = true;
                let (sheet_name, a1) = parse_cell_id(&op.id, i)?;
                let meta = sheet_index
                    .iter()
                    .find(|(n, _, k)| n.eq_ignore_ascii_case(&sheet_name) && *k == "worksheet")
                    .ok_or_else(|| {
                        if sheet_index
                            .iter()
                            .any(|(n, _, _)| n.eq_ignore_ascii_case(&sheet_name))
                        {
                            Error::with_path(
                                ErrorKind::SetCellWorksheetOnly,
                                format!("ops[{i}].id"),
                            )
                        } else {
                            Error::with_path(
                                ErrorKind::SheetNotFound {
                                    name: sheet_name.clone(),
                                },
                                format!("ops[{i}].id"),
                            )
                        }
                    })?;
                let part = meta.1.clone();
                let cell = a1::parse_cell(&a1)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                apply_set_cell(&mut doc.root, cell, op, i)?;
            }
            XlsxOp::SetCellStyle(op) => {
                let (sheet_name, a1) = parse_cell_id(&op.id, i)?;
                let meta = sheet_index
                    .iter()
                    .find(|(n, _, k)| n.eq_ignore_ascii_case(&sheet_name) && *k == "worksheet")
                    .ok_or_else(|| {
                        if sheet_index
                            .iter()
                            .any(|(n, _, _)| n.eq_ignore_ascii_case(&sheet_name))
                        {
                            Error::with_path(
                                ErrorKind::SetCellWorksheetOnly,
                                format!("ops[{i}].id"),
                            )
                        } else {
                            Error::with_path(
                                ErrorKind::SheetNotFound {
                                    name: sheet_name.clone(),
                                },
                                format!("ops[{i}].id"),
                            )
                        }
                    })?;
                let part = meta.1.clone();
                let cell = a1::parse_cell(&a1)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let styles_root = style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                style::apply_set_cell_style(&mut doc.root, styles_root, cell, op, i)?;
            }
            XlsxOp::SetHeaderFooterText(op) => {
                let (part, _kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                hf::set_text(&mut doc.root, op);
            }
            XlsxOp::ReplacePicture(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.target.sheet, i)?;
                reject_hf_pic_sheet(kind, op.target.hf, i)?;
                picture::replace_picture(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertPicture(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind != "worksheet" {
                    return Err(Error::with_path(
                        ErrorKind::InsertPictureWorksheetOnly,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                if op.hf.is_some() {
                    hf::insert_picture(&mut pkg, doc, &part, op, &format!("ops[{i}].file"))?;
                } else {
                    picture::insert_picture(&mut pkg, doc, &part, op, &format!("ops[{i}].file"))?;
                }
            }
            XlsxOp::DeletePicture(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.target.sheet, i)?;
                reject_hf_pic_sheet(kind, op.target.hf, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                picture::delete_picture(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::SetPictureSize(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.target.sheet, i)?;
                reject_hf_pic_sheet(kind, op.target.hf, i)?;
                picture::set_picture_size(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetPictureAnchor(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.target.sheet, i)?;
                reject_hf_pic_sheet(kind, op.target.hf, i)?;
                picture::set_picture_anchor(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertChart(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind != "worksheet" {
                    return Err(Error::with_path(
                        ErrorKind::InsertChartWorksheetOnly,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                chart::insert_chart(&mut pkg, &part, &op.sheet, &mut dirty_sheets, op, i)?;
                touched_cells = true;
            }
            XlsxOp::InsertChartSheet(op) => {
                chartsheet::insert(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &mut sheet_index,
                    &mut dirty_sheets,
                    op,
                    &format!("ops[{i}]"),
                    i,
                )?;
                workbook_dirty = true;
                touched_cells = true;
            }
            XlsxOp::DeleteChartSheet(op) => {
                chartsheet::delete(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &mut sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )?;
                workbook_dirty = true;
            }
            XlsxOp::InsertSheet(op) => {
                sheets::insert(&mut pkg, &mut wb_doc, &wb_part, &mut sheet_index, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::DeleteSheet(op) => {
                sheets::delete(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &mut sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )?;
                workbook_dirty = true;
            }
            XlsxOp::RenameSheet(op) => {
                sheets::rename(&pkg, &mut wb_doc, &mut sheet_index, &dirty_sheets, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::MoveSheet(op) => {
                sheets::move_sheet(&mut wb_doc, &mut sheet_index, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::SetSheetHidden(op) => {
                sheets::set_hidden(&mut wb_doc, &sheet_index, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::SetColWidth(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                layout::apply_set_col_width(&mut doc.root, op)?;
            }
            XlsxOp::SetRowHeight(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                layout::apply_set_row_height(&mut doc.root, op)?;
            }
            XlsxOp::MergeRange(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                layout::apply_merge_range(&mut doc.root, op, i)?;
            }
            XlsxOp::UnmergeRange(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                layout::apply_unmerge_range(&mut doc.root, op);
            }
            XlsxOp::InsertDataValidation(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::validation::apply_insert(&mut doc.root, op, i)?;
            }
            XlsxOp::SetDataValidation(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::validation::apply_set(&mut doc.root, op, i)?;
            }
            XlsxOp::DeleteDataValidation(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::validation::apply_delete(&mut doc.root, op, i)?;
            }
            XlsxOp::InsertConditionalFormat(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let dxf_id = match &op.dxf {
                    Some(dxf) => {
                        let styles_root =
                            style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                        style::intern_dxf(styles_root, dxf)
                    }
                    None => None,
                };
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::cf::apply_insert(&mut doc.root, op, dxf_id, i)?;
            }
            XlsxOp::SetConditionalFormat(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let dxf_id = match &op.dxf {
                    Patch::Absent => None,
                    Patch::Clear => Some(None),
                    Patch::Set(dxf) => {
                        let styles_root =
                            style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                        Some(style::intern_dxf(styles_root, dxf))
                    }
                };
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::cf::apply_set(&mut doc.root, op, dxf_id, i)?;
            }
            XlsxOp::DeleteConditionalFormat(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::cf::apply_delete(&mut doc.root, op, i)?;
            }
            XlsxOp::InsertSheetTable(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                super::table::apply_insert(
                    &mut pkg,
                    &wb_doc,
                    &wb_part,
                    &sheet_index,
                    &mut dirty_sheets,
                    &part,
                    op,
                    i,
                )?;
            }
            XlsxOp::SetSheetTable(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                super::table::apply_set(
                    &mut pkg,
                    &wb_doc,
                    &sheet_index,
                    &mut dirty_sheets,
                    &part,
                    op,
                    i,
                )?;
            }
            XlsxOp::DeleteSheetTable(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                super::table::apply_delete(&mut pkg, &mut dirty_sheets, &part, op, i)?;
            }
            XlsxOp::InsertPivotTable(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                super::pivot::apply_insert(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &sheet_index,
                    &mut dirty_sheets,
                    &part,
                    op,
                    i,
                )?;
                workbook_dirty = true;
            }
            XlsxOp::SetPivotTable(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                if super::pivot::apply_set(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &sheet_index,
                    &mut dirty_sheets,
                    &part,
                    op,
                    i,
                )? {
                    workbook_dirty = true;
                }
            }
            XlsxOp::DeletePivotTable(op) => {
                let part = require_layout_worksheet(&sheet_index, &op.sheet, i)?;
                super::pivot::apply_delete(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &sheet_index,
                    &part,
                    op,
                    i,
                )?;
                workbook_dirty = true;
            }
            XlsxOp::InsertDefinedName(op) => {
                super::names::apply_insert(&pkg, &mut wb_doc, &sheet_index, &dirty_sheets, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::SetDefinedName(op) => {
                super::names::apply_set(&pkg, &mut wb_doc, &sheet_index, &dirty_sheets, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::DeleteDefinedName(op) => {
                super::names::apply_delete(&mut wb_doc, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::SetPrintSetup(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind == "dialogsheet" || kind == "macrosheet" {
                    return Err(Error::with_path(
                        ErrorKind::DialogOrMacroSheet,
                        format!("ops[{i}].id"),
                    ));
                }
                let sheet_id = sheet_index
                    .iter()
                    .position(|(n, _, _)| n.eq_ignore_ascii_case(&op.sheet))
                    .expect("require_named_sheet 已核对表名") as u32;
                let exact = sheet_index[sheet_id as usize].0.clone();
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                if super::print::apply(&mut doc.root, &mut wb_doc, sheet_id, &exact, kind, op, i)? {
                    workbook_dirty = true;
                }
            }
            XlsxOp::SetSheetView(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind == "dialogsheet" || kind == "macrosheet" {
                    return Err(Error::with_path(
                        ErrorKind::DialogOrMacroSheet,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::view::apply_view(&mut doc.root, kind, op, i)?;
            }
            XlsxOp::SetAutoFilter(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind == "dialogsheet" || kind == "macrosheet" {
                    return Err(Error::with_path(
                        ErrorKind::DialogOrMacroSheet,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let overlap = match op.refer.as_deref() {
                    Some(raw) => {
                        let (start, end) = super::a1::parse_range(raw)
                            .map_err(|e| e.at(format!("ops[{i}].ref")))?;
                        super::table::range_overlaps_table(
                            &pkg,
                            &wb_doc,
                            &sheet_index,
                            &dirty_sheets,
                            &part,
                            start,
                            end,
                        )?
                    }
                    None => false,
                };
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::view::apply_filter(&mut doc.root, kind, op, overlap, i)?;
            }
            XlsxOp::SetSortState(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                if kind == "dialogsheet" || kind == "macrosheet" {
                    return Err(Error::with_path(
                        ErrorKind::DialogOrMacroSheet,
                        format!("ops[{i}].id"),
                    ));
                }
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::view::apply_sort(&mut doc.root, kind, op, i)?;
            }
            XlsxOp::InsertCustomView(op) => {
                load_content_sheets(&pkg, &sheet_index, &mut dirty_sheets)?;
                custom_view::insert(&mut wb_doc.root, &sheet_index, &mut dirty_sheets, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::DeleteCustomView(op) => {
                load_content_sheets(&pkg, &sheet_index, &mut dirty_sheets)?;
                custom_view::delete(&mut wb_doc.root, &sheet_index, &mut dirty_sheets, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::ShowCustomView(op) => {
                load_content_sheets(&pkg, &sheet_index, &mut dirty_sheets)?;
                custom_view::show(&wb_doc.root, &sheet_index, &mut dirty_sheets, op, i)?;
            }
            XlsxOp::GroupRows(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_group_rows(&mut doc.root, op, i)?;
            }
            XlsxOp::UngroupRows(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_ungroup_rows(&mut doc.root, op, i)?;
            }
            XlsxOp::GroupCols(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_group_cols(&mut doc.root, op, i)?;
            }
            XlsxOp::UngroupCols(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_ungroup_cols(&mut doc.root, op, i)?;
            }
            XlsxOp::SetOutline(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_set_outline(&mut doc.root, op, i)?;
            }
            XlsxOp::InsertComment(op) => {
                let cell = a1::parse_cell(&op.a1)?;
                let part = require_comments_worksheet(&sheet_index, &op.sheet, i)?;
                comments::apply_insert(&mut pkg, &wb_part, &mut dirty_sheets, &part, cell, op)?;
            }
            XlsxOp::ReplyComment(op) => {
                let target = comments::parse_comment_id(&op.id, &format!("ops[{i}].id"))?;
                let part = require_comments_worksheet(&sheet_index, &target.sheet, i)?;
                comments::apply_reply(&mut pkg, &wb_part, &mut dirty_sheets, &part, op, i)?;
            }
            XlsxOp::DeleteComment(op) => {
                let target = comments::parse_comment_id(&op.id, &format!("ops[{i}].id"))?;
                let part = require_comments_worksheet(&sheet_index, &target.sheet, i)?;
                comments::apply_delete(&mut pkg, &mut dirty_sheets, &part, op, i)?;
            }
            XlsxOp::SetCommentText(op) => {
                let target = comments::parse_comment_id(&op.id, &format!("ops[{i}].id"))?;
                let part = require_comments_worksheet(&sheet_index, &target.sheet, i)?;
                comments::apply_set_text(&mut pkg, &mut dirty_sheets, &part, op, i)?;
            }
            XlsxOp::SetCommentMeta(op) => {
                let target = comments::parse_comment_id(&op.id, &format!("ops[{i}].id"))?;
                let part = require_comments_worksheet(&sheet_index, &target.sheet, i)?;
                comments::apply_set_meta(&mut pkg, &wb_part, &mut dirty_sheets, &part, op, i)?;
            }
            XlsxOp::SetCommentDone(op) => {
                let target = comments::parse_comment_id(&op.id, &format!("ops[{i}].id"))?;
                let part = require_comments_worksheet(&sheet_index, &target.sheet, i)?;
                comments::apply_set_done(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertHyperlink(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertHyperlinkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                hyperlink::apply_insert(
                    &mut pkg,
                    &mut doc.root,
                    &part,
                    op,
                    &format!("ops[{i}].ref"),
                )?;
            }
            XlsxOp::SetHyperlink(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertHyperlinkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                hyperlink::apply_set(&mut pkg, &mut doc.root, &part, op, i)?;
            }
            XlsxOp::DeleteHyperlink(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertHyperlinkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                hyperlink::apply_delete(&mut pkg, &mut doc.root, &part, op, i)?;
            }
            XlsxOp::InsertSparkline(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertSparklineWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                sparkline::apply_insert(&mut doc.root, &op.sheet, op, i)?;
            }
            XlsxOp::SetSparkline(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertSparklineWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                sparkline::apply_set(&mut doc.root, &op.sheet, op, i)?;
            }
            XlsxOp::DeleteSparkline(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertSparklineWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                sparkline::apply_delete(&mut doc.root, op, i)?;
            }
            XlsxOp::SetProtection(op) => match op {
                SetProtectionOp::Sheet(op) => {
                    let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                    load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                    let doc = dirty_of(&mut dirty_sheets, &part);
                    super::protect::apply_sheet(&mut doc.root, kind, op, i)?;
                }
                SetProtectionOp::Workbook(op) => {
                    super::protect::apply_workbook(&mut wb_doc.root, op, i)?;
                    workbook_dirty = true;
                }
            },
            XlsxOp::InsertTextBox(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::insert_textbox(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::InsertShape(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::insert_shape(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::InsertConnector(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::insert_connector(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::DeleteTextBox(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::delete_textbox(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::DeleteShape(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::delete_shape(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::DeleteConnector(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::delete_connector(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::GroupShapes(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::group::group_shapes(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::Ungroup(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::group::ungroup(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::InsertOle(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ole::insert(&mut pkg, doc, &part, op, &format!("ops[{i}].file"))?;
            }
            XlsxOp::DeleteOle(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ole::delete(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::InsertModel3d(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertModel3dWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                model3d::insert(&mut pkg, doc, &part, op, &format!("ops[{i}].file"))?;
            }
            XlsxOp::ReplaceModel3d(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertModel3dWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                model3d::replace(&mut pkg, &part, op, i)?;
            }
            XlsxOp::DeleteModel3d(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertModel3dWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                model3d::delete(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::InsertMath(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertMathWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                omml::insert(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::SetMath(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertMathWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                omml::set(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::DeleteMath(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertMathWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                omml::delete(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::InsertSlicer(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                slicer::insert(&mut pkg, &mut wb_doc, &wb_part, doc, &part, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::DeleteSlicer(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                slicer::delete(&mut pkg, &mut wb_doc, &wb_part, doc, &part, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::InsertConnection(op) => {
                connection::insert(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::DeleteConnection(op) => {
                connection::delete(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::InsertExternalLink(op) => {
                extlink::insert(&mut pkg, &mut wb_doc, &wb_part, op)?;
                workbook_dirty = true;
            }
            XlsxOp::DeleteExternalLink(op) => {
                extlink::delete(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &sheet_index,
                    &dirty_sheets,
                    op,
                    i,
                )?;
                workbook_dirty = true;
            }
            XlsxOp::SetTextBoxSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::set_textbox_size(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetTextBoxAnchor(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::set_textbox_anchor(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetShapeText(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::set_shape_text(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::InsertSmartArt(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::insert_smartart(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::DeleteSmartArt(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::delete_smartart(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::SetSmartArtText(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.diagram.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::apply_text(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::AddSmartArtPoint(op) => {
                let sheet = match &op.parent {
                    smartart::AddParent::Diagram(t) => t.sheet.as_str(),
                    smartart::AddParent::Point(p) => p.diagram.sheet.as_str(),
                };
                let part = require_drawing_worksheet(
                    &sheet_index,
                    sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::apply_add_point(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::DeleteSmartArtPoint(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.diagram.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::apply_delete_point(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSmartArtSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::set_smartart_size(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSmartArtAnchor(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::set_smartart_anchor(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSmartArtColors(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::apply_colors(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSmartArtStyle(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::apply_style(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSmartArtLayout(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertSmartArtWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                smartart::apply_layout(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::DeleteChart(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.target.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                chart::delete_chart(&mut pkg, doc, &part, kind, op, i)?;
            }
            XlsxOp::SetChartTitle(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_title(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartSeries(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                if chart::set_chart_series(
                    &mut pkg,
                    &part,
                    &op.target.sheet,
                    &sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )? {
                    touched_cells = true;
                }
            }
            XlsxOp::AddChartSeries(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                if chart::add_chart_series(
                    &mut pkg,
                    &part,
                    &op.target.sheet,
                    &sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )? {
                    touched_cells = true;
                }
            }
            XlsxOp::DeleteChartSeries(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::delete_chart_series(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartType(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_type(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartAxisTitle(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_axis_title(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartLegend(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_legend(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartDataLabels(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_data_labels(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartSize(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_size(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartAnchor(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_chart_anchor(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertTrendline(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::insert_trendline(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetTrendline(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_trendline(&mut pkg, &part, op, i)?;
            }
            XlsxOp::DeleteTrendline(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::delete_trendline(&mut pkg, &part, op, i)?;
            }
            XlsxOp::InsertErrorBars(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::insert_error_bars(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetErrorBars(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_error_bars(&mut pkg, &part, op, i)?;
            }
            XlsxOp::DeleteErrorBars(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::delete_error_bars(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetChartSeriesAxis(op) => {
                let part = require_sheet_part(&sheet_index, &op.target.sheet, i)?;
                chart::set_series_axis(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetProps(op) => {
                crate::props::apply_set_props(&mut pkg, op, i)?;
            }
            XlsxOp::SetCustomProp(op) => {
                crate::props::apply_set_custom_prop(&mut pkg, op, i)?;
            }
            XlsxOp::DeleteCustomProp(op) => {
                crate::props::apply_delete_custom_prop(&mut pkg, op, i)?;
            }
        }
    }

    for (part, doc) in dirty_sheets {
        pkg.set(&part, doc.to_bytes()?)?;
    }
    if let Some((part, doc)) = styles_doc {
        pkg.set(&part, doc.to_bytes()?)?;
    }

    if touched_cells {
        mark_full_calc(&mut wb_doc.root);
        remove_calc_chain(&mut pkg, &wb_part)?;
        workbook_dirty = true;
    }
    if workbook_dirty {
        pkg.set(&wb_part, wb_doc.to_bytes()?)?;
    }

    pkg.save_to_vec()
}

fn require_named_sheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<(String, &'static str), Error> {
    index
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, part, kind)| (part.clone(), *kind))
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: name.to_string(),
                },
                format!("ops[{op_index}].id"),
            )
        })
}

fn require_outline_worksheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    let (part, kind) = require_named_sheet(index, name, op_index)?;
    if kind == "dialogsheet" || kind == "macrosheet" {
        return Err(Error::with_path(
            ErrorKind::DialogOrMacroSheet,
            format!("ops[{op_index}].id"),
        ));
    }
    if kind != "worksheet" {
        return Err(Error::with_path(
            ErrorKind::InvalidOutline { field: "id".into() },
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(part)
}

fn require_sheet_part(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    Ok(require_named_sheet(index, name, op_index)?.0)
}

fn require_comments_worksheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    require_drawing_worksheet(index, name, op_index, ErrorKind::CommentsWorksheetOnly)
}

fn require_drawing_worksheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
    chartsheet_err: ErrorKind,
) -> Result<String, Error> {
    let meta = index.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(name));
    let Some((_, part, kind)) = meta else {
        return Err(Error::with_path(
            ErrorKind::SheetNotFound {
                name: name.to_string(),
            },
            format!("ops[{op_index}].id"),
        ));
    };
    match *kind {
        "worksheet" => Ok(part.clone()),
        "dialogsheet" | "macrosheet" => Err(Error::with_path(
            ErrorKind::DialogOrMacroSheet,
            format!("ops[{op_index}].id"),
        )),
        _ => Err(Error::with_path(
            chartsheet_err,
            format!("ops[{op_index}].id"),
        )),
    }
}

fn require_layout_worksheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
) -> Result<String, Error> {
    let meta = index
        .iter()
        .find(|(n, _, k)| n.eq_ignore_ascii_case(name) && *k == "worksheet");
    if let Some((_, part, _)) = meta {
        return Ok(part.clone());
    }
    if index.iter().any(|(n, _, _)| n.eq_ignore_ascii_case(name)) {
        return Err(Error::with_path(
            ErrorKind::LayoutWorksheetOnly,
            format!("ops[{op_index}].id"),
        ));
    }
    Err(Error::with_path(
        ErrorKind::SheetNotFound {
            name: name.to_string(),
        },
        format!("ops[{op_index}].id"),
    ))
}

fn reject_hf_pic_sheet(
    kind: &str,
    hf: Option<super::request::HfSlot>,
    op_index: usize,
) -> Result<(), Error> {
    if hf.is_some() && kind != "worksheet" {
        return Err(Error::with_path(
            ErrorKind::InsertPictureWorksheetOnly,
            format!("ops[{op_index}].id"),
        ));
    }
    Ok(())
}

fn load_content_sheets(
    pkg: &Package,
    sheet_index: &[(String, String, &'static str)],
    dirty: &mut Vec<(String, Document)>,
) -> Result<(), Error> {
    for (_, part, kind) in sheet_index {
        if *kind == "worksheet" || *kind == "chartsheet" {
            load_dirty(pkg, dirty, part, false)?;
        }
    }
    Ok(())
}

pub(crate) fn load_dirty(
    pkg: &Package,
    dirty: &mut Vec<(String, Document)>,
    part: &str,
    make_explicit: bool,
) -> Result<(), Error> {
    if let Some((_, doc)) = dirty.iter_mut().find(|(p, _)| p == part) {
        if make_explicit {
            make_refs_explicit(&mut doc.root);
        }
        return Ok(());
    }
    let mut parsed = xml::parse(pkg.get(part)?)?;
    if make_explicit {
        // 省略 r 的行和单元格按隐式编号补成显式，否则后面插新行会让它们的编号漂移。
        make_refs_explicit(&mut parsed.root);
    }
    dirty.push((part.to_string(), parsed));
    Ok(())
}

pub(crate) fn dirty_of<'a>(dirty: &'a mut [(String, Document)], part: &str) -> &'a mut Document {
    &mut dirty.iter_mut().find(|(p, _)| p == part).unwrap().1
}

fn sheet_index(
    workbook: &Element,
    rels: &Relationships,
    wb_part: &str,
) -> Result<Vec<(String, String, &'static str)>, Error> {
    let sheets = workbook.child_by_local("sheets").ok_or_else(|| {
        Error::new(ErrorKind::XmlMissing {
            parent: "workbook.xml".into(),
            child: "sheets".into(),
        })
    })?;
    let mut out = Vec::new();
    for sh in sheets.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("").to_string();
        // `r:id`：前缀只是习惯，任何映射到 relationships 命名空间的前缀都合法。
        let rid = sh.attr_local("id").ok_or_else(|| {
            Error::new(ErrorKind::XmlMissing {
                parent: "sheet".into(),
                child: "r:id".into(),
            })
        })?;
        let rel = rels.find_by_id(rid).ok_or_else(|| {
            Error::new(ErrorKind::RelNotFound {
                source: RelSource::Unspecified,
                rid: rid.to_string(),
            })
        })?;
        let kind = inspect::sheet_kind(&rel.rel_type);
        let part = opc::resolve_rel_target(wb_part, &rel.target)?;
        out.push((name, part, kind));
    }
    Ok(out)
}

fn parse_cell_id(id: &str, op_index: usize) -> Result<(String, String), Error> {
    let rest = id.strip_prefix("cell:").ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let bang = rest.rfind('!').ok_or_else(|| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    let name = rest[..bang].to_string();
    if name.is_empty() {
        return Err(Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        ));
    }
    let a1 = a1::normalize_a1(&rest[bang + 1..]).map_err(|_| {
        Error::with_path(
            ErrorKind::InvalidId {
                reason: IdReason::NotCell { id: id.to_string() },
            },
            format!("ops[{op_index}].id"),
        )
    })?;
    Ok((name, a1))
}

/// SpreadsheetML 允许 `<row>` 和 `<c>` 省略 `r`，此时编号由出现顺序推出（§18.3.1.73、§18.3.1.4）。
/// 这里把隐式编号写成显式，语义不变，但后续插入新行就不会改变别的行的编号。
fn make_refs_explicit(worksheet: &mut Element) {
    let Some(sheet_data) = worksheet.child_by_local_mut("sheetData") else {
        return;
    };
    let mut prev_row = 0u32;
    for row in sheet_data.element_children_mut() {
        if row.local_name() != "row" {
            continue;
        }
        let declared = row.attr("r").and_then(|s| s.parse::<u32>().ok());
        let row_num = declared.unwrap_or(prev_row + 1);
        if declared.is_none() {
            row.set_attr("r", row_num.to_string());
        }
        prev_row = row_num;

        let mut prev_col = 0u32;
        for c in row.element_children_mut() {
            if c.local_name() != "c" {
                continue;
            }
            let declared_col = c
                .attr("r")
                .and_then(|s| a1::parse_cell(s).ok())
                .map(|p| p.col);
            let col = declared_col.unwrap_or(prev_col + 1);
            if declared_col.is_none() {
                c.set_attr("r", a1::format_cell(col, row_num));
            }
            prev_col = col;
        }
    }
}

pub(crate) fn apply_set_cell(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    op_index: usize,
) -> Result<(), Error> {
    let index = worksheet
        .child_by_local("sheetData")
        .map(formula::index_sheet)
        .unwrap_or_default();

    if let Some(formula::Occupied::DataTable { .. }) = formula::occupy_at(&index, cell) {
        return Err(Error::with_path(
            ErrorKind::DataTableFormula {
                cell: a1::format_cell(cell.col, cell.row),
            },
            format!("ops[{op_index}].id"),
        ));
    }

    let array_master = match formula::occupy_at(&index, cell) {
        Some(formula::Occupied::Array { master }) => Some(master),
        _ => None,
    };
    let shared_here = match formula::occupy_at(&index, cell) {
        Some(formula::Occupied::Shared { si, master }) => Some((si, master)),
        _ => None,
    };

    let wants_array = op.array == Some(true) || op.array_ref.is_some();
    let wants_drop_array = op.array == Some(false);
    let wants_explode = op.shared == Some(false);
    let wants_shared_keep = op.shared == Some(true) || op.shared_ref.is_some();
    let is_array_master = array_master == Some(cell);
    let is_shared_master = shared_here
        .as_ref()
        .map(|(_, master)| *master == cell)
        .unwrap_or(false);

    if let Some(master) = array_master {
        if master != cell {
            return Err(Error::with_path(
                ErrorKind::ArrayFormulaPart {
                    cell: a1::format_cell(cell.col, cell.row),
                },
                format!("ops[{op_index}].id"),
            ));
        }
    }

    if wants_array {
        let (start, end) = op
            .array_ref
            .or_else(|| {
                index
                    .arrays
                    .iter()
                    .find(|g| g.master == cell)
                    .map(|g| (g.start, g.end))
            })
            .unwrap_or((cell, cell));
        return write_array_formula(worksheet, cell, start, end, op, &index, op_index);
    }

    if is_array_master && op.formula.is_some() && !wants_drop_array {
        let g = index.arrays.iter().find(|g| g.master == cell).unwrap();
        return write_array_formula(worksheet, cell, g.start, g.end, op, &index, op_index);
    }

    if is_array_master {
        apply_plain_or_clear(worksheet, cell, op);
        return Ok(());
    }

    if let Some((_, master)) = &shared_here {
        if *master != cell && !wants_shared_keep {
            apply_plain_or_clear(worksheet, cell, op);
            return Ok(());
        }
    }

    let explode_then_write =
        wants_explode || (is_shared_master && op.value.is_some() && !wants_shared_keep);

    if explode_then_write {
        if let Some((si, master)) = shared_here {
            let body = index
                .shared
                .get(&si)
                .map(|g| g.formula.clone())
                .unwrap_or_default();
            explode_shared_group(worksheet, &si, master, &body);
        }
        apply_plain_or_clear(worksheet, cell, op);
        return Ok(());
    }

    if wants_shared_keep || (is_shared_master && op.formula.is_some()) {
        return write_shared_formula(worksheet, cell, op, &index, is_shared_master, op_index);
    }

    apply_plain_or_clear(worksheet, cell, op);
    Ok(())
}

pub(crate) fn write_json_cell(
    worksheet: &mut Element,
    cell: a1::CellRef,
    value: &serde_json::Value,
    op_index: usize,
) -> Result<(), Error> {
    let parsed = super::request::parse_cell_value(value, &format!("ops[{op_index}]"))?;
    apply_set_cell(
        worksheet,
        cell,
        &SetCellOp {
            id: String::new(),
            value: Some(parsed),
            formula: None,
            shared: None,
            shared_ref: None,
            array: None,
            array_ref: None,
        },
        op_index,
    )
}

fn apply_plain_or_clear(worksheet: &mut Element, cell: a1::CellRef, op: &SetCellOp) {
    let sheet_data = ensure_sheet_data(worksheet);
    if matches!(&op.value, Some(SetCellValue::Null)) {
        clear_cell(sheet_data, cell);
        expand_dimension(worksheet, cell);
        return;
    }
    let c = ensure_cell(sheet_data, cell);
    fill_cell(c, op);
    expand_dimension(worksheet, cell);
}

fn write_array_formula(
    worksheet: &mut Element,
    cell: a1::CellRef,
    start: a1::CellRef,
    end: a1::CellRef,
    op: &SetCellOp,
    index: &formula::FormulaIndex,
    op_index: usize,
) -> Result<(), Error> {
    if cell != start {
        return Err(Error::with_path(
            ErrorKind::FormulaRangeNotTopLeft,
            format!("ops[{op_index}].id"),
        ));
    }
    let body = formula::formula_body(op.formula.as_deref().unwrap_or("")).to_string();
    reject_overlap(index, start, end, SkipGroup::Array(cell), op_index)?;
    let refer = a1::format_range(start, end);
    {
        let sheet_data = ensure_sheet_data(worksheet);
        for row in start.row..=end.row {
            for col in start.col..=end.col {
                let here = a1::CellRef { col, row };
                let c = ensure_cell(sheet_data, here);
                if here == cell {
                    write_formula_elem(c, &body, FormulaWrite::Array { refer: &refer });
                } else {
                    strip_formula_and_value(c);
                }
            }
        }
    }
    expand_dimension(worksheet, start);
    expand_dimension(worksheet, end);
    Ok(())
}

fn write_shared_formula(
    worksheet: &mut Element,
    cell: a1::CellRef,
    op: &SetCellOp,
    index: &formula::FormulaIndex,
    is_shared_master: bool,
    op_index: usize,
) -> Result<(), Error> {
    if !is_shared_master && op.shared_ref.is_none() {
        return Err(Error::with_path(
            ErrorKind::SharedNeedsRef,
            format!("ops[{op_index}]"),
        ));
    }
    let existing_by_master = index.shared.values().find(|g| g.master == cell);
    let (start, end) = op
        .shared_ref
        .or_else(|| existing_by_master.map(|g| (g.start, g.end)))
        .unwrap_or((cell, cell));
    if cell != start {
        return Err(Error::with_path(
            ErrorKind::FormulaRangeNotTopLeft,
            format!("ops[{op_index}].id"),
        ));
    }
    let existing = existing_by_master.or_else(|| {
        let hits: Vec<_> = index
            .shared
            .values()
            .filter(|g| a1::ranges_overlap(g.start, g.end, start, end))
            .collect();
        if hits.len() == 1 {
            Some(hits[0])
        } else {
            None
        }
    });
    let body = op
        .formula
        .as_deref()
        .map(formula::formula_body)
        .map(str::to_string)
        .or_else(|| existing.map(|g| g.formula.clone()))
        .unwrap_or_default();
    let si = existing.map(|g| g.si.clone()).unwrap_or_else(|| {
        worksheet
            .child_by_local("sheetData")
            .map(formula::next_si)
            .unwrap_or(0)
            .to_string()
    });
    let same_range = existing
        .map(|g| g.start == start && g.end == end && g.master == cell)
        .unwrap_or(false);
    if same_range {
        let refer = a1::format_range(start, end);
        let sheet_data = ensure_sheet_data(worksheet);
        let c = ensure_cell(sheet_data, cell);
        if let Some(f) = c.child_by_local_mut("f") {
            f.set_text(&body);
            if f.attr("t") != Some("shared") {
                f.set_attr("t", "shared");
            }
            f.set_attr("si", &si);
            f.set_attr("ref", refer);
        } else {
            write_formula_elem(
                c,
                &body,
                FormulaWrite::Shared {
                    si: &si,
                    refer: Some(&refer),
                },
            );
        }
        strip_value_only(c);
        expand_dimension(worksheet, cell);
        return Ok(());
    }
    reject_overlap(index, start, end, SkipGroup::Shared(&si), op_index)?;
    if let Some(old) = existing {
        let members = worksheet
            .child_by_local("sheetData")
            .map(|sd| formula::shared_members(sd, &old.si))
            .unwrap_or_default();
        for m in members {
            if m != cell && !a1::in_range(m, start, end) {
                let shifted = formula::shift_a1_formula(&old.formula, old.master, m);
                let sheet_data = ensure_sheet_data(worksheet);
                let c = ensure_cell(sheet_data, m);
                write_formula_elem(c, &shifted, FormulaWrite::Normal);
            }
        }
    }
    if start == end {
        apply_plain_or_clear(worksheet, cell, op);
        return Ok(());
    }
    let refer = a1::format_range(start, end);
    {
        let sheet_data = ensure_sheet_data(worksheet);
        for row in start.row..=end.row {
            for col in start.col..=end.col {
                let here = a1::CellRef { col, row };
                let c = ensure_cell(sheet_data, here);
                if here == cell {
                    write_formula_elem(
                        c,
                        &body,
                        FormulaWrite::Shared {
                            si: &si,
                            refer: Some(&refer),
                        },
                    );
                } else {
                    write_formula_elem(
                        c,
                        "",
                        FormulaWrite::Shared {
                            si: &si,
                            refer: None,
                        },
                    );
                }
            }
        }
    }
    expand_dimension(worksheet, start);
    expand_dimension(worksheet, end);
    Ok(())
}

fn explode_shared_group(worksheet: &mut Element, si: &str, master: a1::CellRef, body: &str) {
    let members = worksheet
        .child_by_local("sheetData")
        .map(|sd| formula::shared_members(sd, si))
        .unwrap_or_default();
    let sheet_data = ensure_sheet_data(worksheet);
    for m in members {
        if m == master {
            continue;
        }
        let shifted = formula::shift_a1_formula(body, master, m);
        let c = ensure_cell(sheet_data, m);
        write_formula_elem(c, &shifted, FormulaWrite::Normal);
    }
}

enum SkipGroup<'a> {
    Shared(&'a str),
    Array(a1::CellRef),
}

fn reject_overlap(
    index: &formula::FormulaIndex,
    start: a1::CellRef,
    end: a1::CellRef,
    skip: SkipGroup<'_>,
    op_index: usize,
) -> Result<(), Error> {
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            let here = a1::CellRef { col, row };
            match formula::occupy_at(index, here) {
                Some(formula::Occupied::DataTable { .. }) => {
                    return Err(Error::with_path(
                        ErrorKind::FormulaGroupOverlap {
                            cell: a1::format_cell(col, row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Array { master }) => {
                    let ok = matches!(skip, SkipGroup::Array(m) if m == master);
                    if !ok {
                        return Err(Error::with_path(
                            ErrorKind::FormulaGroupOverlap {
                                cell: a1::format_cell(col, row),
                            },
                            format!("ops[{op_index}].id"),
                        ));
                    }
                }
                Some(formula::Occupied::Shared { si, .. }) => {
                    let ok = matches!(skip, SkipGroup::Shared(s) if s == si);
                    if !ok {
                        return Err(Error::with_path(
                            ErrorKind::FormulaGroupOverlap {
                                cell: a1::format_cell(col, row),
                            },
                            format!("ops[{op_index}].id"),
                        ));
                    }
                }
                None => {}
            }
        }
    }
    Ok(())
}

enum FormulaWrite<'a> {
    Normal,
    Shared { si: &'a str, refer: Option<&'a str> },
    Array { refer: &'a str },
}

fn strip_formula_and_value(c: &mut Element) {
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) => true,
    });
    c.remove_attr("t");
}

fn strip_value_only(c: &mut Element) {
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "v" | "is"),
        Node::Text(_) => true,
    });
}

fn write_formula_elem(c: &mut Element, body: &str, kind: FormulaWrite<'_>) {
    strip_formula_and_value(c);
    let mut f = Element::new("f");
    match kind {
        FormulaWrite::Normal => {}
        FormulaWrite::Shared { si, refer } => {
            f.set_attr("t", "shared");
            f.set_attr("si", si);
            if let Some(r) = refer {
                f.set_attr("ref", r);
            }
        }
        FormulaWrite::Array { refer } => {
            f.set_attr("t", "array");
            f.set_attr("ref", refer);
        }
    }
    if !body.is_empty() {
        f.set_text(body);
    }
    c.children.insert(0, Node::Element(f));
}

/// 合并区域碰到共享主格、数组或数据表时拒绝，避免 Excel 修复。
pub(crate) fn reject_merge_formula_conflicts(
    worksheet: &Element,
    start: a1::CellRef,
    end: a1::CellRef,
    op_index: usize,
) -> Result<(), Error> {
    let Some(sheet_data) = worksheet.child_by_local("sheetData") else {
        return Ok(());
    };
    let index = formula::index_sheet(sheet_data);
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            let here = a1::CellRef { col, row };
            match formula::occupy_at(&index, here) {
                Some(formula::Occupied::DataTable { .. }) => {
                    return Err(Error::with_path(
                        ErrorKind::DataTableFormula {
                            cell: a1::format_cell(col, row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Array { master }) => {
                    return Err(Error::with_path(
                        ErrorKind::ArrayFormulaPart {
                            cell: a1::format_cell(master.col, master.row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Shared { si, master })
                    if master == here
                        && formula::has_shared_dependents(sheet_data, &si, master) =>
                {
                    return Err(Error::with_path(
                        ErrorKind::SharedFormulaMaster {
                            cell: a1::format_cell(master.col, master.row),
                        },
                        format!("ops[{op_index}].id"),
                    ));
                }
                Some(formula::Occupied::Shared { .. }) | None => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn find_cell(sheet_data: &Element, cell: a1::CellRef) -> Option<&Element> {
    for row in sheet_data.children_by_local("row") {
        if row.attr("r").and_then(|s| s.parse::<u32>().ok()) != Some(cell.row) {
            continue;
        }
        for c in row.children_by_local("c") {
            if c.attr("r").and_then(|r| a1::parse_cell(r).ok()) == Some(cell) {
                return Some(c);
            }
        }
    }
    None
}

pub(crate) fn ensure_sheet_data(worksheet: &mut Element) -> &mut Element {
    if worksheet.child_by_local("sheetData").is_none() {
        worksheet.push_element(Element::new("sheetData"));
    }
    worksheet.child_by_local_mut("sheetData").unwrap()
}

pub(crate) fn ensure_cell(sheet_data: &mut Element, cell: a1::CellRef) -> &mut Element {
    let _ = ensure_row(sheet_data, cell.row);
    let row_idx = find_row_insert_pos(sheet_data, cell.row).0;
    let Node::Element(row) = &mut sheet_data.children[row_idx] else {
        unreachable!();
    };
    let col_pos = find_cell_insert_pos(row, cell.col);
    if !col_pos.1 {
        let mut c = Element::new("c");
        c.set_attr("r", a1::format_cell(cell.col, cell.row));
        row.children.insert(col_pos.0, Node::Element(c));
    }
    let col_idx = find_cell_insert_pos(row, cell.col).0;
    let Node::Element(c) = &mut row.children[col_idx] else {
        unreachable!();
    };
    c
}

pub(crate) fn ensure_row(sheet_data: &mut Element, row: u32) -> &mut Element {
    let row_pos = find_row_insert_pos(sheet_data, row);
    if !row_pos.1 {
        let mut el = Element::new("row");
        el.set_attr("r", row.to_string());
        sheet_data.children.insert(row_pos.0, Node::Element(el));
    }
    let row_idx = find_row_insert_pos(sheet_data, row).0;
    let Node::Element(row_el) = &mut sheet_data.children[row_idx] else {
        unreachable!();
    };
    row_el
}

pub(crate) fn find_row_index(sheet_data: &Element, row: u32) -> Option<usize> {
    find_row_insert_pos(sheet_data, row).found_then()
}

fn find_row_insert_pos(sheet_data: &Element, row: u32) -> (usize, bool) {
    for (i, n) in sheet_data.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "row" {
            continue;
        }
        let r = e.attr("r").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        if r == row {
            return (i, true);
        }
        if r > row {
            return (i, false);
        }
    }
    (sheet_data.children.len(), false)
}

fn find_cell_insert_pos(row: &Element, col: u32) -> (usize, bool) {
    for (i, n) in row.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "c" {
            continue;
        }
        let Some(r) = e.attr("r") else { continue };
        let Ok(pref) = a1::parse_cell(r) else {
            continue;
        };
        if pref.col == col {
            return (i, true);
        }
        if pref.col > col {
            return (i, false);
        }
    }
    (row.children.len(), false)
}

pub(crate) fn clear_cell(sheet_data: &mut Element, cell: a1::CellRef) {
    let Some(row_idx) = find_row_insert_pos(sheet_data, cell.row).found_then() else {
        return;
    };
    let Node::Element(row) = &mut sheet_data.children[row_idx] else {
        return;
    };
    let Some(cell_idx) = find_cell_insert_pos(row, cell.col).found_then() else {
        return;
    };
    let Node::Element(c) = &mut row.children[cell_idx] else {
        return;
    };
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) => false,
    });
    c.remove_attr("t");
    let only_r = c
        .attrs
        .iter()
        .all(|(k, _)| k.rsplit(':').next().unwrap_or(k) == "r");
    if only_r {
        row.children.remove(cell_idx);
    }
    let row_empty = row.element_children().all(|e| e.local_name() != "c");
    if row_empty && row.attrs.iter().all(|(k, _)| k == "r") {
        sheet_data.children.remove(row_idx);
    }
}

trait FoundThen {
    fn found_then(self) -> Option<usize>;
}

impl FoundThen for (usize, bool) {
    fn found_then(self) -> Option<usize> {
        if self.1 {
            Some(self.0)
        } else {
            None
        }
    }
}

fn fill_cell(c: &mut Element, op: &SetCellOp) {
    c.children.retain(|n| match n {
        Node::Element(e) => !matches!(e.local_name(), "f" | "v" | "is"),
        Node::Text(_) => true,
    });
    if let Some(formula) = &op.formula {
        c.remove_attr("t");
        let body = formula.strip_prefix('=').unwrap_or(formula);
        let mut f = Element::new("f");
        f.set_text(body);
        c.children.insert(0, Node::Element(f));
        return;
    }
    match op.value.as_ref() {
        Some(SetCellValue::Number(n)) => {
            c.remove_attr("t");
            let mut v = Element::new("v");
            v.set_text(number_xml(n));
            c.push_element(v);
        }
        Some(SetCellValue::Bool(b)) => {
            c.set_attr("t", "b");
            let mut v = Element::new("v");
            v.set_text(if *b { "1" } else { "0" });
            c.push_element(v);
        }
        Some(SetCellValue::Text(s)) => {
            c.set_attr("t", "inlineStr");
            let mut t = Element::new("t");
            if s.starts_with(' ') || s.ends_with(' ') || s.contains('\n') {
                t.set_attr("xml:space", "preserve");
            }
            t.set_text(s.clone());
            let mut is = Element::new("is");
            is.push_element(t);
            c.push_element(is);
        }
        Some(SetCellValue::Null) | None => {}
    }
}

fn number_xml(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    n.to_string()
}

pub(crate) fn expand_dimension(worksheet: &mut Element, cell: a1::CellRef) {
    let Some(dim) = worksheet.child_by_local_mut("dimension") else {
        return;
    };
    let Some(pref) = dim.attr("ref") else {
        dim.set_attr("ref", a1::format_cell(cell.col, cell.row));
        return;
    };
    let Ok((mut start, mut end)) = a1::parse_range(pref) else {
        return;
    };
    if cell.col < start.col {
        start.col = cell.col;
    }
    if cell.row < start.row {
        start.row = cell.row;
    }
    if cell.col > end.col {
        end.col = cell.col;
    }
    if cell.row > end.row {
        end.row = cell.row;
    }
    dim.set_attr(
        "ref",
        format!(
            "{}:{}",
            a1::format_cell(start.col, start.row),
            a1::format_cell(end.col, end.row)
        ),
    );
}

pub(crate) fn mark_full_calc(workbook: &mut Element) {
    if let Some(calc) = workbook.child_by_local_mut("calcPr") {
        calc.set_attr("fullCalcOnLoad", "1");
        return;
    }
    workbook.push_element(Element::new("calcPr").with_attr("fullCalcOnLoad", "1"));
}

pub(crate) fn remove_calc_chain(pkg: &mut Package, wb_part: &str) -> Result<(), Error> {
    let mut rels = Relationships::from_package(pkg, wb_part)?;
    let targets: Vec<String> = rels
        .rels
        .iter()
        .filter(|r| r.rel_type.ends_with("/calcChain") || r.rel_type == REL_CALC_CHAIN)
        .filter(|r| !r.is_external())
        .filter_map(|r| opc::resolve_rel_target(wb_part, &r.target).ok())
        .collect();

    let mut extra = Vec::new();
    if pkg.contains("/xl/calcChain.xml") {
        extra.push("/xl/calcChain.xml".to_string());
    }

    let mut types = ContentTypes::from_package(pkg)?;
    let mut changed_rels = false;
    for t in targets.iter().chain(extra.iter()) {
        if pkg.contains(t) {
            pkg.remove(t);
            types.remove_override(t);
            rels.remove_by_target_part(wb_part, t);
            changed_rels = true;
        }
    }
    if changed_rels {
        types.write_to(pkg)?;
        rels.write_to(pkg, wb_part)?;
    }
    Ok(())
}
