//! 按 id 改单元格或工作表图，改单元格时标记打开时重算。

use super::hyperlink;
use super::request::{Patch, SetProtectionOp, XlsxOp};
use super::sparkline;
use crate::error::{Error, ErrorKind, RelSource};
use crate::opc::{self, Package, Relationships};
use crate::xml::{self, Document, Element};

use super::a1;
use super::background;
use super::chart;
use super::chartsheet;
use super::comments;
use super::connection;
use super::custom_view;
use super::extlink;
use super::form_control;
use super::hf;
use super::ink;
use super::inspect;
use super::layout;
use super::legacy_hf;
use super::model3d;
use super::ole;
use super::omml;
use super::picture;
use super::shape;
use super::sheets;
use super::slicer;
use super::smartart;
use super::style;

pub fn apply(bytes: &[u8], ops: &[XlsxOp]) -> Result<Vec<u8>, Error> {
    apply_in(bytes, ops, None)
}

pub(crate) fn apply_in(
    bytes: &[u8],
    ops: &[XlsxOp],
    base_dir: Option<&std::path::Path>,
) -> Result<Vec<u8>, Error> {
    apply_pkg(bytes, ops, base_dir)?.save_to_vec()
}

pub fn dry_run(bytes: &[u8], ops: &[XlsxOp]) -> Result<Vec<String>, Error> {
    Ok(apply_pkg(bytes, ops, None)?.dirty_part_names())
}

fn apply_pkg(
    bytes: &[u8],
    ops: &[XlsxOp],
    base_dir: Option<&std::path::Path>,
) -> Result<crate::opc::Package, Error> {
    let _pins = super::list_pin::PinGuard::enter();
    let mut pkg = Package::open_bytes(bytes.to_vec())?;
    opc::reject_strict(&pkg)?;
    let wb_part = opc::office_document_part(&pkg)?;
    let mut wb_doc = xml::parse(pkg.get(&wb_part)?)?;
    super::list_pin::pin_workbook(&mut wb_doc.root);
    let wb_rels = Relationships::from_package(&pkg, &wb_part)?;

    let mut sheet_index = sheet_index(&wb_doc.root, &wb_rels, &wb_part)?;
    super::draw_pin::preload(&pkg, &sheet_index)?;
    super::list_pin::capture_pivots(&pkg, &sheet_index)?;

    let mut dirty_sheets: Vec<(String, Document)> = Vec::new();
    let mut styles_doc: Option<(String, Document)> = None;
    let mut touched_cells = false;
    let mut eval_formulas = false;
    let mut workbook_dirty = false;
    let mut hot: Option<super::cell::SheetHot> = None;

    for (i, op) in ops.iter().enumerate() {
        if !matches!(op, XlsxOp::SetCell(_)) {
            hot = None;
        }
        match op {
            XlsxOp::SetCell(op) => {
                touched_cells = true;
                eval_formulas = true;
                super::cell::apply_hot(&pkg, &sheet_index, &mut dirty_sheets, &mut hot, op, i)?;
            }
            XlsxOp::SetCellStyle(op) => {
                let (sheet_name, a1) = super::cell::parse_cell_id(&op.id, i)?;
                let part = worksheet_part(&sheet_index, &sheet_name, i)?;
                let cell = a1::parse_cell(&a1)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let styles_root = style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                style::apply_set_cell_style(&mut doc.root, styles_root, cell, op, i)?;
            }
            XlsxOp::InsertCellStyle(op) => {
                let styles_root = style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                style::insert_cell_style(styles_root, op, i)?;
            }
            XlsxOp::DeleteCellStyle(op) => {
                let styles_root = style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                style::delete_cell_style(styles_root, op, i)?;
            }
            XlsxOp::SetCellNamedStyle(op) => {
                let (sheet_name, a1) = super::cell::parse_cell_id(&op.id, i)?;
                let part = worksheet_part(&sheet_index, &sheet_name, i)?;
                let cell = a1::parse_cell(&a1)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let styles_root = style::ensure_styles(&mut pkg, &wb_part, &mut styles_doc)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                style::apply_set_cell_named_style(&mut doc.root, styles_root, cell, op, i)?;
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
            XlsxOp::ReplaceLegacyPicture(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::LegacyPictureWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                legacy_hf::replace(
                    &mut pkg,
                    &doc.root,
                    &part,
                    &op.id,
                    &op.shape_id,
                    &op.file,
                    i,
                )?;
            }
            XlsxOp::DeleteLegacyPicture(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::LegacyPictureWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                legacy_hf::delete(&mut pkg, &mut doc.root, &part, &op.id, &op.shape_id, i)?;
            }
            XlsxOp::InsertPicture(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                reject_hf_pic_sheet(kind, op.hf, i)?;
                if kind != "worksheet" && kind != "chartsheet" {
                    return Err(Error::with_path(
                        ErrorKind::InsertPictureWorksheetOnly,
                        format!("ops[{i}].id"),
                    ));
                }
                if op.hf.is_none() {
                    reject_rel_mismatch(kind, op.rel.is_some(), i)?;
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
                eval_formulas = true;
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
                eval_formulas = true;
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
                eval_formulas = true;
            }
            XlsxOp::RenameSheet(op) => {
                sheets::rename(
                    &mut pkg,
                    &mut wb_doc,
                    &wb_part,
                    &mut sheet_index,
                    &mut dirty_sheets,
                    op,
                    i,
                )?;
                workbook_dirty = true;
                eval_formulas = true;
            }
            XlsxOp::MoveSheet(op) => {
                sheets::move_sheet(&mut wb_doc, &mut sheet_index, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::SetSheetHidden(op) => {
                sheets::set_hidden(&mut wb_doc, &sheet_index, op, i)?;
                workbook_dirty = true;
            }
            XlsxOp::SetSheetBackground(op) => {
                let part = background::require_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                background::set(
                    &mut pkg,
                    &part,
                    &mut doc.root,
                    &op.file,
                    &format!("ops[{i}].file"),
                )?;
            }
            XlsxOp::DeleteSheetBackground(op) => {
                let part = background::require_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                background::delete(&mut pkg, &part, &mut doc.root, i)?;
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
                eval_formulas = true;
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
                eval_formulas = true;
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
                eval_formulas = true;
            }
            XlsxOp::SetDefinedName(op) => {
                super::names::apply_set(&pkg, &mut wb_doc, &sheet_index, &dirty_sheets, op, i)?;
                workbook_dirty = true;
                eval_formulas = true;
            }
            XlsxOp::DeleteDefinedName(op) => {
                super::names::apply_delete(&mut wb_doc, op, i)?;
                workbook_dirty = true;
                eval_formulas = true;
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
                let mut op = op.clone();
                super::view::intern_sort_dxfs(&mut pkg, &wb_part, &mut styles_doc, &mut op)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::view::apply_sort(&mut doc.root, kind, &op, i)?;
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
            XlsxOp::SetRowHidden(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_set_row_hidden(&mut doc.root, op);
            }
            XlsxOp::SetColHidden(op) => {
                let part = require_outline_worksheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, true)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::outline::apply_set_col_hidden(&mut doc.root, op);
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
                if let Some(host) = &op.drawing {
                    let (part, _) = require_ws_or_chartsheet(
                        &sheet_index,
                        &op.sheet,
                        i,
                        ErrorKind::InsertHyperlinkWorksheetOnly,
                    )?;
                    hyperlink::apply_insert_drawing(
                        &mut pkg,
                        &part,
                        host,
                        op.url.as_deref(),
                        op.location.as_deref(),
                        op.tooltip.as_deref(),
                        i,
                    )?;
                } else {
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
            }
            XlsxOp::SetHyperlink(op) => {
                if let Some(host) = &op.drawing {
                    let (part, _) = require_ws_or_chartsheet(
                        &sheet_index,
                        &op.sheet,
                        i,
                        ErrorKind::InsertHyperlinkWorksheetOnly,
                    )?;
                    hyperlink::apply_set_drawing(&mut pkg, &part, host, op, i)?;
                } else {
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
            }
            XlsxOp::DeleteHyperlink(op) => {
                if let Some(host) = &op.drawing {
                    let (part, _) = require_ws_or_chartsheet(
                        &sheet_index,
                        &op.sheet,
                        i,
                        ErrorKind::InsertHyperlinkWorksheetOnly,
                    )?;
                    hyperlink::apply_delete_drawing(&mut pkg, &part, host, &op.id, i)?;
                } else {
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
            XlsxOp::InsertProtectedRange(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::protect::apply_insert_range(&mut doc.root, kind, op, i)?;
            }
            XlsxOp::SetProtectedRange(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::protect::apply_set_range(&mut doc.root, kind, op, i)?;
            }
            XlsxOp::DeleteProtectedRange(op) => {
                let (part, kind) = require_named_sheet(&sheet_index, &op.sheet, i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                super::protect::apply_delete_range(&mut doc.root, kind, op, i)?;
            }
            XlsxOp::InsertTextBox(op) => {
                let (part, kind) = require_ws_or_chartsheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                reject_rel_mismatch(kind, op.rel.is_some(), i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::insert_textbox(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::InsertShape(op) => {
                let (part, kind) = require_ws_or_chartsheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                reject_rel_mismatch(kind, op.rel.is_some(), i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::insert_shape(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::InsertConnector(op) => {
                let (part, kind) = require_ws_or_chartsheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                reject_rel_mismatch(kind, op.rel.is_some(), i)?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::insert_connector(&mut pkg, doc, &part, op)?;
            }
            XlsxOp::DeleteTextBox(op) => {
                let (part, _) = require_ws_or_chartsheet(
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
                let (part, _) = require_ws_or_chartsheet(
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
                let (part, _) = require_ws_or_chartsheet(
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
            XlsxOp::SetOleSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ole::set_size(&mut pkg, doc, &part, op, i)?;
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
            XlsxOp::SetModel3dSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertModel3dWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                model3d::set_size(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetModel3dAnchor(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertModel3dWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                model3d::set_anchor(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::InsertInk(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::InsertInkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ink::insert(&mut pkg, doc, &part, op, &format!("ops[{i}].file"))?;
            }
            XlsxOp::ReplaceInk(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertInkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                ink::replace(&mut pkg, &part, op, i)?;
            }
            XlsxOp::DeleteInk(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertInkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ink::delete(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::SetInkSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertInkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ink::set_size(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetInkAnchor(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertInkWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                ink::set_anchor(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetModel3dCamera(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertModel3dWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                model3d::set_camera(&mut pkg, &doc.root, &part, op, i)?;
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
            XlsxOp::SetMathSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertMathWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                omml::set_size(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetMathAnchor(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertMathWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                omml::set_anchor(&mut pkg, &doc.root, &part, op, i)?;
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
            XlsxOp::SetSlicerSize(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                slicer::set_size(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSlicerAnchor(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                slicer::set_anchor(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetSlicerView(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                slicer::set_view(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetSlicerFilter(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                slicer::set_filter(&mut pkg, &part, op, i)?;
            }
            XlsxOp::SetThemeColors(op) => {
                super::theme::set_colors(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::SetThemeFonts(op) => {
                super::theme::set_fonts(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::SetThemeFmt(op) => {
                super::theme::set_fmt(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::SetThemeExtraSchemes(op) => {
                super::theme::set_extra_schemes(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::SetThemeObjectDefaults(op) => {
                super::theme::set_object_defaults(&mut pkg, &wb_part, op, i)?;
            }
            XlsxOp::SetThemeOverride(op) => {
                super::theme::set_override(&mut pkg, &wb_part, op, i)?;
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
                    &mut dirty_sheets,
                    op,
                    i,
                )?;
                workbook_dirty = true;
            }
            XlsxOp::SetExternalLink(op) => {
                extlink::set_path(&mut pkg, &wb_doc, &wb_part, op, i)?;
            }
            XlsxOp::SetTextBoxSize(op) => {
                let (part, _) = require_ws_or_chartsheet(
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
                let (part, _) = require_ws_or_chartsheet(
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
                let (part, _) = require_ws_or_chartsheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::set_shape_text(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetShapeRuns(op) => {
                let (part, _) = require_ws_or_chartsheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::set_shape_runs(&mut pkg, &doc.root, &part, op, i)?;
            }
            XlsxOp::SetShapeFormat(op) => {
                let (part, _) = require_ws_or_chartsheet(
                    &sheet_index,
                    &op.target.sheet,
                    i,
                    ErrorKind::InsertTextBoxWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                shape::set_shape_format(&mut pkg, &doc.root, &part, op, i)?;
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
            XlsxOp::SetFormControl(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::FormControlWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                form_control::set(&mut pkg, &mut doc.root, &part, op, i)?;
            }
            XlsxOp::InsertFormControl(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::FormControlWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                form_control::insert(&mut pkg, doc, &part, op, i)?;
            }
            XlsxOp::DeleteFormControl(op) => {
                let part = require_drawing_worksheet(
                    &sheet_index,
                    &op.sheet,
                    i,
                    ErrorKind::FormControlWorksheetOnly,
                )?;
                load_dirty(&pkg, &mut dirty_sheets, &part, false)?;
                let doc = dirty_of(&mut dirty_sheets, &part);
                form_control::delete(&mut pkg, &mut doc.root, &part, op, i)?;
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
                    eval_formulas = true;
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
                    eval_formulas = true;
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
        super::cell::mark_full_calc(&mut wb_doc.root);
        super::cell::remove_calc_chain(&mut pkg, &wb_part)?;
        workbook_dirty = true;
    }
    if workbook_dirty {
        pkg.set(&wb_part, wb_doc.to_bytes()?)?;
    }
    if eval_formulas {
        super::eval::fill_formula_caches(&mut pkg, base_dir)?;
    }

    Ok(pkg)
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

fn require_ws_or_chartsheet(
    index: &[(String, String, &'static str)],
    name: &str,
    op_index: usize,
    other_err: ErrorKind,
) -> Result<(String, &'static str), Error> {
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
        "worksheet" | "chartsheet" => Ok((part.clone(), *kind)),
        "dialogsheet" | "macrosheet" => Err(Error::with_path(
            ErrorKind::DialogOrMacroSheet,
            format!("ops[{op_index}].id"),
        )),
        _ => Err(Error::with_path(other_err, format!("ops[{op_index}].id"))),
    }
}

fn reject_rel_mismatch(kind: &str, has_rel: bool, op_index: usize) -> Result<(), Error> {
    if kind == "chartsheet" && !has_rel {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "from".into(),
            },
            format!("ops[{op_index}].from"),
        ));
    }
    if kind == "worksheet" && has_rel {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: "fromX".into(),
            },
            format!("ops[{op_index}].fromX"),
        ));
    }
    Ok(())
}

fn reject_hf_pic_sheet(
    kind: &str,
    hf: Option<super::request::HfSlot>,
    op_index: usize,
) -> Result<(), Error> {
    if hf.is_some() && kind != "worksheet" && kind != "chartsheet" {
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
            super::cell::make_refs_explicit(&mut doc.root);
        }
        return Ok(());
    }
    let mut parsed = xml::parse(pkg.get(part)?)?;
    super::list_pin::pin_sheet(&mut parsed.root);
    let _ = super::ole::pin_objects(pkg, part, &mut parsed.root);
    if make_explicit {
        // 省略 r 的行和单元格按隐式编号补成显式，否则后面插新行会让它们的编号漂移。
        super::cell::make_refs_explicit(&mut parsed.root);
    }
    dirty.push((part.to_string(), parsed));
    Ok(())
}

pub(crate) fn dirty_of<'a>(dirty: &'a mut [(String, Document)], part: &str) -> &'a mut Document {
    &mut dirty.iter_mut().find(|(p, _)| p == part).unwrap().1
}

pub(crate) fn worksheet_part(
    sheet_index: &[(String, String, &str)],
    sheet_name: &str,
    i: usize,
) -> Result<String, Error> {
    sheet_index
        .iter()
        .find(|(n, _, k)| n.eq_ignore_ascii_case(sheet_name) && *k == "worksheet")
        .map(|(_, part, _)| part.clone())
        .ok_or_else(|| {
            if sheet_index
                .iter()
                .any(|(n, _, _)| n.eq_ignore_ascii_case(sheet_name))
            {
                Error::with_path(ErrorKind::SetCellWorksheetOnly, format!("ops[{i}].id"))
            } else {
                Error::with_path(
                    ErrorKind::SheetNotFound {
                        name: sheet_name.to_string(),
                    },
                    format!("ops[{i}].id"),
                )
            }
        })
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
