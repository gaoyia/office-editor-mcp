    use super::super::geometry::GeometryContext;
    use super::super::picture::AnchorKind;
    use super::*;

    #[test]
    fn decimal_axis_ticks_do_not_invent_a_scientific_label_at_zero() {
        let ticks = tick_values(-0.3, 0.5, 0.1);
        assert_eq!(ticks.len(), 9);
        assert_eq!(ticks[3], 0.0);
        assert_eq!(fmt_tick(ticks[3]), "0");
        assert!(ticks.iter().all(|v| !fmt_tick(*v).contains('e')));
        assert!(
            fmt_tick(1e-8).contains('e'),
            "genuinely small data is not zero"
        );
    }

    #[test]
    fn signed_crossing_uses_major_grid_rasterization_not_independent_rounding() {
        let p = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, 0.0), 133);
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, -1.0), 157);
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, -2.0), 180);
        assert_eq!(crossing_axis_y(&p, -3.0, 5.0, 1.0, 4.0), 39);
        assert_eq!(crossing_axis_y(&p, -2.0, 8.0, 2.0, 0.0), 166);
        assert_eq!(crossing_axis_y(&p, 0.0, 5.0, 1.0, 0.0), 204);
        assert_eq!(crossing_axis_y(&p, -5.0, 0.0, 1.0, 0.0), 16);
        // Invert frames use inclusive edges; positive fills retain the
        // supplied shape edge rather than unconditionally clearing a grid.
        assert_eq!(
            column_bar_bounds(57.1, 86.56666666666666, 133, 180, -2.0, 0.0, true),
            [57, 133, 86, 180]
        );
        assert_eq!(
            column_bar_bounds(57.1, 86.56666666666666, 16, 53, -1.0, 0.0, true),
            [57, 16, 86, 53]
        );
        assert_eq!(
            column_bar_bounds(70.0, 107.0, 334, 51, 1.0, 0.0, true),
            [70, 51, 107, 334]
        );
    }

    #[test]
    fn one_cell_size_uses_anchor_ext_when_frame_xfrm_is_zero() {
        // Excel 16.0 Save writes graphicFrame/xfrm/ext as 0×0 while keeping the
        // real oneCell size on xdr:ext. Prefer the anchor so export still works.
        let geometry = GeometryContext::calibri11();
        let sheet = crate::xml::parse(br#"<worksheet/>"#).unwrap();
        let cases = [
            (5486400, 3200400, 0, 0, true, Ok((576, 336))),
            (5486400, 3200400, 5486400, 3200400, true, Ok((576, 336))),
            (0, 0, 5486400, 3200400, true, Ok((576, 336))),
            (5486400, 3200400, 5486400, 3200400, false, Ok((576, 336))),
            (0, 0, 0, 0, true, Err(())),
        ];
        for (anchor_cx, anchor_cy, frame_cx, frame_cy, include_anchor, expected) in cases {
            let anchor_ext = if include_anchor {
                format!(r#"<xdr:ext cx="{anchor_cx}" cy="{anchor_cy}"/>"#)
            } else {
                String::new()
            };
            let xml = format!(
                r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><xdr:oneCellAnchor><xdr:from><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>{anchor_ext}<xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr><xdr:xfrm><a:off x="0" y="0"/><a:ext cx="{frame_cx}" cy="{frame_cy}"/></xdr:xfrm></xdr:graphicFrame></xdr:oneCellAnchor></xdr:wsDr>"#
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let anchor = doc.root.child_by_local("oneCellAnchor").unwrap();
            let got = bitmap_px(&sheet.root, anchor, AnchorKind::OneCell, &geometry);
            match expected {
                Ok(size) => assert_eq!(got.unwrap(), size, "anchor={anchor_cx}x{anchor_cy} frame={frame_cx}x{frame_cy} include={include_anchor}"),
                Err(()) => assert!(got.is_err(), "expected NI for zero extents"),
            }
        }
    }

    /// Existing colour/layout assertions inspect the white-composited export.
    /// Raw RGBA and coverage have separate assertions below.
    fn render_rgb_chart(
        root: &Element,
        width: u32,
        height: u32,
        accents: [[u8; 3]; 6],
        font_bytes: Option<&[u8]>,
        host: ChartHost,
        cells: Option<&super::super::sparkline::CellCache>,
        sheet: &str,
    ) -> Result<Vec<u8>, Error> {
        let data =
            super::render_chart(root, width, height, accents, font_bytes, host, cells, sheet)?;
        let mut reader = png::Decoder::new(std::io::Cursor::new(data))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = &buf[..info.buffer_size()];
        if info.color_type == png::ColorType::Rgb {
            return encode_png(width, height, data, png::ColorType::Rgb, RenderDpi::DEFAULT);
        }
        let rgb: Vec<_> = data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                p[..3].iter().map(move |c| {
                    ((u16::from(*c) * u16::from(p[3]) + 255 * (255 - u16::from(p[3])) + 127) / 255)
                        as u8
                })
            })
            .collect();
        encode_png(width, height, &rgb, png::ColorType::Rgb, RenderDpi::DEFAULT)
    }

    #[test]
    fn bar_negative_default_inversion_and_explicit_point_overrides() {
        for (extra, expected) in [
            ("", true),
            ("<c:invertIfNegative/>", true),
            ("<c:invertIfNegative val=\"1\"/>", true),
            ("<c:invertIfNegative val=\"0\"/>", false),
            ("<c:invertIfNegative val=\"false\"/>", false),
            ("<c:invertIfNegative val=\"0\"/><c:dPt><c:idx val=\"0\"/><c:invertIfNegative/></c:dPt>", true),
            ("<c:dPt><c:idx val=\"0\"/><c:invertIfNegative val=\"0\"/></c:dPt>", false),
        ] {
            let xml = bar_xml(&line_ser(0, "S1", &[Some(-8.0), Some(8.0)], extra), "", "maxMin", Some("bar"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
            let series = &chart.series[0];
            assert_eq!(series.point_inversion.first().copied().flatten().unwrap_or(series.invert_if_negative), expected, "{extra}");
            for dpi_value in [96, 106, 144] {
                let dpi = RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap()).unwrap();
                let mut canvas = Canvas::new(80, 60); canvas.dpi = dpi;
                canvas.fill_rect(0, 0, 80, 60, [200, 100, 50]);
                draw_bar_value(&mut canvas, series, 0, -8.0, [10, 10, 60, 40], FALLBACK_ACCENTS[0]);
                let pixel = |x: usize, y: usize| {let i=(y*80+x)*3; &canvas.px[i..i+3]};
                assert_eq!(pixel(30, 25), if expected { &[255,255,255] } else { &FALLBACK_ACCENTS[0] });
                if expected {assert!(pixel(10, 25).iter().all(|v| *v < 160), "negative outline must be black");}
                // Positive values must never be inverted.
                draw_bar_value(&mut canvas, series, 0, 8.0, [10, 10, 60, 40], FALLBACK_ACCENTS[0]);
                assert_eq!(&canvas.px[(25*80+30)*3..(25*80+30)*3+3], &FALLBACK_ACCENTS[0]);
            }
        }
    }

    #[test]
    fn bar_negative_outline_is_one_pixel_solid_and_pixel_aligned() {
        // Official signed bar-2 (chart:3, A=-8) uses a 1 px solid #000000 box.
        // Mid-edge neighbors are pure fill or background, not a 2 px gray AA
        // band. Bounds are half-open fill [x0, x1) x [y0, y1); the outline sits
        // on those integer edges, so the right/bottom lines occupy the exclusive
        // fill sides and coincide with the zero axis on the value end.
        let xml = bar_xml(
            &line_ser(0, "S1", &[Some(-8.0), Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let series = &chart.series[0];
        let bg = [200, 100, 50];
        let [x0, y0, x1, y1] = [10, 10, 60, 40];
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            let mut canvas = Canvas::new(80, 60);
            canvas.dpi = dpi;
            canvas.fill_rect(0, 0, 80, 60, bg);
            draw_bar_value(
                &mut canvas,
                series,
                0,
                -8.0,
                [x0, y0, x1, y1],
                FALLBACK_ACCENTS[0],
            );
            let pixel = |x: i32, y: i32| {
                let i = (y as usize * 80 + x as usize) * 3;
                [canvas.px[i], canvas.px[i + 1], canvas.px[i + 2]]
            };
            assert_eq!(pixel(x0, 25), INK, "left mid @ {dpi_value}");
            assert_eq!(pixel(x1, 25), INK, "right mid @ {dpi_value}");
            assert_eq!(pixel(35, y0), INK, "top mid @ {dpi_value}");
            assert_eq!(pixel(35, y1), INK, "bottom mid @ {dpi_value}");
            assert_eq!(pixel(x0 - 1, 25), bg, "outside left @ {dpi_value}");
            assert_eq!(
                pixel(x0 + 1, 25),
                [255, 255, 255],
                "inside left @ {dpi_value}"
            );
            assert_eq!(
                pixel(x1 - 1, 25),
                [255, 255, 255],
                "inside right @ {dpi_value}"
            );
            assert_eq!(pixel(x1 + 1, 25), bg, "outside right @ {dpi_value}");
            assert_eq!(pixel(35, y0 - 1), bg, "outside top @ {dpi_value}");
            assert_eq!(
                pixel(35, y0 + 1),
                [255, 255, 255],
                "inside top @ {dpi_value}"
            );
            assert_eq!(
                pixel(35, y1 - 1),
                [255, 255, 255],
                "inside bottom @ {dpi_value}"
            );
            assert_eq!(pixel(35, y1 + 1), bg, "outside bottom @ {dpi_value}");
            for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                assert_eq!(pixel(x, y), INK, "corner {x},{y} @ {dpi_value}");
            }
            assert_eq!(pixel(x0 - 1, y0 - 1), bg, "outside corner @ {dpi_value}");
        }
    }

    fn mixed_column_xml(values: &[f64]) -> String {
        let pts = values
            .iter()
            .enumerate()
            .map(|(i, _v)| {
                let name = (b'A' + i as u8) as char;
                format!("<c:pt idx=\"{i}\"><c:v>{name}</c:v></c:pt>",)
            })
            .collect::<String>();
        let nums = values
            .iter()
            .enumerate()
            .map(|(i, v)| format!("<c:pt idx=\"{i}\"><c:v>{v}</c:v></c:pt>"))
            .collect::<String>();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>S</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="{n}"/>{pts}</c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="{n}"/>{nums}</c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx>
        <c:scaling>
          <c:min val="-3"/>
          <c:max val="5"/>
        </c:scaling>
        <c:majorUnit val="1"/>
        <c:majorGridlines/>
        <c:crosses val="autoZero"/>
      </c:valAx>
      <c:catAx>
        <c:scaling><c:orientation val="minMax"/></c:scaling>
        <c:crosses val="autoZero"/>
      </c:catAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            n = values.len(),
            pts = pts,
            nums = nums
        )
    }

    fn decode_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = &buf[..info.buffer_size()];
        if info.color_type == png::ColorType::Rgb {
            return (info.width, info.height, data.to_vec());
        }
        let rgb: Vec<u8> = data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| {
                p[..3].iter().map(move |c| {
                    ((u16::from(*c) * u16::from(p[3]) + 255 * (255 - u16::from(p[3])) + 127) / 255)
                        as u8
                })
            })
            .collect();
        (info.width, info.height, rgb)
    }

    fn row_ink_runs(rgb: &[u8], w: u32, y: i32) -> Vec<(i32, i32)> {
        let mut runs = Vec::new();
        let mut start = None;
        for x in 0..=w as i32 {
            let ink = if x < w as i32 {
                let i = (y as usize * w as usize + x as usize) * 3;
                rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40
            } else {
                false
            };
            match (ink, start) {
                (true, None) => start = Some(x),
                (false, Some(s)) => {
                    runs.push((s, x - 1));
                    start = None;
                }
                _ => {}
            }
        }
        runs
    }

    fn long_ink_rows(rgb: &[u8], w: u32, h: u32, min_dark: i32) -> Vec<i32> {
        (0..h as i32)
            .filter(|&y| {
                row_ink_runs(rgb, w, y)
                    .iter()
                    .map(|(a, b)| b - a + 1)
                    .sum::<i32>()
                    >= min_dark
            })
            .collect()
    }

    #[test]
    fn inverted_columns_rasterize_both_path_edges_without_a_fill_inset() {
        let xml = mixed_column_xml(&[-1.0, -4.0, -2.0]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let mut chart =
            read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        chart.series[0].invert_if_negative = true;
        let mut canvas = Canvas::new(300, 220);
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -5.0, 0.0, 1.0, None, 0.0, false, None);
        // Three equal path widths are 221 / 7.5 pixels. The middle path
        // crosses columns 130..160: rounding two fill edges and subtracting
        // one instead loses both outline sides (131..159).
        assert_eq!(
            row_ink_runs(&canvas.px, canvas.w, 30),
            vec![
                (57, 57),
                (86, 86),
                (130, 130),
                (160, 160),
                (204, 204),
                (233, 233)
            ]
        );
    }

    #[test]
    fn coincident_inverted_path_edges_remain_visible_vertical_hairlines() {
        let xml = mixed_column_xml(&[-1.0, -4.0, -2.0]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mut canvas = Canvas::new(80, 220);
        // Each .4px path has both edges in one pixel. All three must remain
        // visible, even though inclusive x0 == x1 for every outline.
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 2.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -5.0, 0.0, 1.0, None, 0.0, false, None);
        assert_eq!(row_ink_runs(&canvas.px, canvas.w, 30), vec![(35, 37)]);
    }

    #[test]
    fn inverted_path_edges_keep_fractional_phase_and_integer_translation() {
        for x in [-20.75, -0.25, 0.0, 0.5, 3.9, 130.76666666666665] {
            for width in [0.21, 1.0, 2.5, 29.466666666666665] {
                let bounds = column_bar_bounds(x, x + width, 10, 40, -1.0, 0.0, true);
                assert_eq!(
                    bounds,
                    [x.floor() as i32, 10, (x + width).floor() as i32, 40]
                );
                for offset in [-7, -1, 1, 11] {
                    let shifted = column_bar_bounds(
                        x + f64::from(offset),
                        x + width + f64::from(offset),
                        10,
                        40,
                        -1.0,
                        0.0,
                        true,
                    );
                    assert_eq!(shifted, [bounds[0] + offset, 10, bounds[2] + offset, 40]);
                }
            }
        }
    }

    #[test]
    fn positive_column_bounds_preserve_the_supplied_shape_edge() {
        // A rasterized shape edge is not the same thing as a grid row.
        // Moving every positive endpoint by 1 also shifts non-major values.
        assert_eq!(
            column_bar_bounds(10.0, 20.0, 100, 40, 2.3, 0.0, false),
            [10, 40, 20, 100]
        );
    }

    #[test]
    fn thin_inverted_columns_keep_their_single_pixel_outline() {
        // Thin path edges can fall in the same pixel or in separate pixels.
        // Neither case is empty; a small outline need not be a solid ink run.
        let xml = mixed_column_xml(&[-2.0; 60]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let dpi =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        // Supply the plot explicitly: this is an outline-coverage regression,
        // not a dependency on installed fonts or automatic axis label layout.
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
        let mut canvas = Canvas::new(314, 221);
        canvas.dpi = dpi;
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -3.0, 5.0, 1.0, None, 0.0, false, None);
        // Verify both path edges of all 60 columns, not a run count: an
        // unfilled 1.47px-wide path can have two separated 1px edges.
        let slot = 221.0 / 60.0;
        let width = slot / 2.5;
        let mut expected = Vec::new();
        for i in 0..60 {
            let x = 35.0 + slot * f64::from(i) + (slot - width) / 2.0;
            expected.push(x.floor() as i32);
            expected.push((x + width).floor() as i32);
        }
        expected.sort_unstable();
        expected.dedup();
        let actual: Vec<i32> = row_ink_runs(&canvas.px, canvas.w, 167)
            .into_iter()
            .flat_map(|(a, b)| a..=b)
            .collect();
        assert_eq!(actual, expected, "all 60 thin path outlines");
    }

    #[test]
    fn inverted_column_outline_sits_on_zero_axis_and_value_grid() {
        // 106dpi mixed-sign columns (official A=-2 on [-3,5]) must press the
        // zero axis and land the bottom hairline on the -2 grid, not hang a
        // 1px framed box below / beside those rows.
        let xml = mixed_column_xml(&[-2.0, 4.0, 1.0]);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let dpi =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        let png = render_chart_at(
            &doc.root,
            314,
            221,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            dpi,
        )
        .unwrap();
        let (w, h, rgb) = decode_rgb(&png);
        assert_eq!((w, h), (314, 221));
        let grids = long_ink_rows(&rgb, w, h, 160);
        assert!(
            grids.len() >= 5,
            "expected signed major grids, got {grids:?}"
        );
        // Explicit -3..5 / major 1 yields 9 grids; 0 is the sixth, -2 the eighth, 4 the second.
        assert_eq!(grids.len(), 9, "signed major grids {grids:?}");
        let zero = grids[5];
        let neg2 = grids[7];
        let pos4 = grids[1];
        let ink = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40
        };
        // Side strokes are 1px verticals under the 0 axis; skip the value axis.
        let probe = zero + ((neg2 - zero) / 3).max(8);
        let sides: Vec<i32> = (50..w as i32 - 20)
            .filter(|&x| ink(x, probe) && ink(x, probe + 8))
            .collect();
        assert!(
            sides.len() >= 2,
            "expected inverted A sides, got {sides:?} grids={grids:?} zero={zero} probe={probe}"
        );
        let x0 = sides[0];
        let x1 = *sides
            .iter()
            .find(|&&x| x >= x0 + 16)
            .unwrap_or_else(|| panic!("A right edge after {x0} in {sides:?} grids={grids:?}"));
        assert!(
            x1 - x0 >= 16,
            "A box too narrow {x0}..{x1} sides={sides:?} grids={grids:?}"
        );
        let span_ink = |y: i32| (x0..=x1).filter(|&x| ink(x, y)).count();
        assert!(
            span_ink(zero) >= (x1 - x0) as usize,
            "zero axis must press the A box top; zero={zero} box={x0}..{x1} ink={}",
            span_ink(zero)
        );
        assert!(
            span_ink(zero + 1) <= 4,
            "no separate top stroke 1px below the axis; ink={} box={x0}..{x1}",
            span_ink(zero + 1)
        );
        assert!(
            span_ink(neg2) >= (x1 - x0) as usize,
            "A bottom must sit on the -2 grid; y={neg2} ink={}",
            span_ink(neg2)
        );
        assert!(
            neg2 + 1 >= h as i32 || span_ink(neg2 + 1) <= 4,
            "A box must not hang 1px past the value grid; ink={}",
            span_ink(neg2 + 1)
        );
        // Positive 4 must start 1px below its grid, not cover the line.
        let is_teal = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            let (r, g, b) = (rgb[i], rgb[i + 1], rgb[i + 2]);
            b > 80 && u16::from(b) > u16::from(r) + 20 && g > r
        };
        let teal_on = |y: i32| (0..w as i32).filter(|&x| is_teal(x, y)).count();
        assert_eq!(
            teal_on(pos4),
            0,
            "value grid must not be covered by the bar"
        );
        assert!(
            teal_on(pos4 + 1) >= 10,
            "positive bar must start immediately under the value grid; n={}",
            teal_on(pos4 + 1)
        );
    }

    #[test]
    fn pie_auto_layout_matches_public_office_aspect_and_host_probes() {
        // Excel public PlotArea relative to its 4 pt origin, then PowerPoint.
        // Includes height-limited, width-limited, no legend and title hosts.
        let cases = [
            (
                432.0,
                252.0,
                0.0,
                29.314488188976377,
                98.09275590551181,
                22.75,
                198.5,
            ),
            (
                432.0,
                180.0,
                0.0,
                29.314488188976377,
                129.5927559055118,
                18.25,
                135.5,
            ),
            (
                300.0,
                360.0,
                0.0,
                29.314488188976377,
                23.91779527559055,
                68.57503937007874,
                214.84992125984252,
            ),
            (
                600.0,
                252.0,
                0.0,
                29.314488188976377,
                182.0927559055118,
                22.75,
                198.5,
            ),
            (
                300.0,
                300.0,
                0.0,
                29.314488188976377,
                23.91779527559055,
                38.57503937007874,
                214.84992125984252,
            ),
            (432.0, 252.0, 0.0, 0.0, 112.75, 22.75, 198.5),
            (240.0, 180.0, 0.0, 0.0, 48.25, 18.25, 135.5),
            (
                432.0,
                252.0,
                23.65,
                29.314488188976377,
                108.4396062992126,
                44.92181102362205,
                177.80637795,
            ),
            (
                432.0,
                252.0,
                26.15503937007874,
                29.134488188976377,
                109.62559055,
                47.27031496062992,
                175.61440945,
            ),
        ];
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            let px = |pt| dpi.pt_to_px_font(pt);
            for (w, h, title, legend, left, top, diameter) in cases {
                let plot = pie_plot_bounds(px(w), px(h), px(title), px(legend), dpi).unwrap();
                // COM reports geometry to 1/12700 pt. No image-specific offset.
                assert!((plot.x - px(left + 4.0)).abs() < px(0.001));
                assert!((plot.y - px(top + 4.0)).abs() < px(0.001));
                assert!((plot.w - px(diameter)).abs() < px(0.001));
                assert_eq!(plot.w, plot.h);
            }
            assert!(pie_plot_bounds(px(20.0), px(20.0), 0.0, 0.0, dpi).is_none());
        }
    }

    #[test]
    fn valid_unsupported_backgrounds_are_not_silently_defaulted() {
        for config in [
            "<c:spPr><a:ln w='25400'/></c:spPr>",
            "<c:roundedCorners val='0'/>",
            "<c:spPr><a:solidFill><a:srgbClr val='123456'/></a:solidFill></c:spPr>",
            "<c:roundedCorners val='1'/>",
            "<c:spPr><a:solidFill><a:srgbClr val='FFFFFF'/></a:solidFill></c:spPr>",
            "<c:spPr><a:noFill/></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='25000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='100000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='0' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            "<c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='5400000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='accent1'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:tint val='60000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:shade val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='60000'/><a:lumOff val='40000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:tint val='60000'/></a:schemeClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:alpha val='50000'/></a:schemeClr></a:solidFill></c:spPr>",
        ] {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(super::render_chart(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                ""
            )
            .is_ok());
        }
    }

    #[test]
    fn rectangular_frame_and_chart_plot_fills_have_real_pixels() {
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            let xml = LONG_LEGEND.replace("<c:chart>", "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>")
                .replace("<c:plotArea>", "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='FFF0DD'/></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame(&doc.root).unwrap();
            assert!(!frame.rounded);
            assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
            assert_eq!(frame.plot_fill, FrameFill::Solid([255, 240, 221]));
            let png = render_chart_at(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                dpi,
            )
            .unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                .read_info()
                .unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            assert_eq!(
                info.color_type,
                png::ColorType::Rgb,
                "rectangle is opaque; no rounded-corner mask"
            );
            let px = |x: usize, y: usize| &buf[(y * 576 + x) * 3..(y * 576 + x) * 3 + 3];
            assert_eq!(
                px(5, 5),
                &[234, 247, 247],
                "chart-area fill must not stay white"
            );
            assert_eq!(
                px(0, 5),
                &BORDER,
                "straight edge must reach the corner band"
            );
            assert!(
                buf[..info.buffer_size()]
                    .as_chunks::<3>()
                    .0
                    .contains(&[255, 240, 221]),
                "plot-area fill must be independently painted"
            );
            assert!(
                buf[..info.buffer_size()]
                    .as_chunks::<3>()
                    .0
                    .contains(&FALLBACK_ACCENTS[0]),
                "data must still be drawn"
            );
        }
    }

    #[test]
    fn chart_and_plot_nofill_are_not_silently_white() {
        let hidden = "<c:spPr><a:noFill/><a:extLst><a:ext uri=\"{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}\"><a14:hiddenFill xmlns:a14=\"http://schemas.microsoft.com/office/drawing/2010/main\"><a:solidFill><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:solidFill></a14:hiddenFill></a:ext></a:extLst></c:spPr>";
        for chart_sppr in ["<c:spPr><a:noFill/></c:spPr>", hidden] {
            let xml = LONG_LEGEND.replace(
                "<c:chart>",
                &format!("<c:roundedCorners val='0'/>{chart_sppr}<c:chart>"),
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame(&doc.root).unwrap();
            assert_eq!(frame.fill, FrameFill::NoFill, "{chart_sppr}");
            assert_eq!(frame.plot_fill, FrameFill::Default);
            let png = super::render_chart(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                .read_info()
                .unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            let i = (5 * 576 + 5) * 4;
            assert_eq!(
                buf[i + 3],
                0,
                "chart-area noFill must be transparent, not white"
            );
            assert_ne!(&buf[i..i + 3], &[255, 255, 255]);
        }

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:noFill/></c:spPr>",
            );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
        assert_eq!(frame.plot_fill, FrameFill::NoFill);
        let png = super::render_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(
            info.color_type,
            png::ColorType::Rgb,
            "opaque chart fill with plot noFill stays RGB"
        );
        let px = |x: usize, y: usize| &buf[(y * 576 + x) * 3..(y * 576 + x) * 3 + 3];
        assert_eq!(
            px(5, 5),
            &[234, 247, 247],
            "plot noFill must not paint a white plot over the chart fill"
        );
        assert!(
            buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[234, 247, 247]),
            "chart-area fill must remain visible in the plot"
        );
        assert!(
            !buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[255, 255, 255]),
            "plot noFill over a non-white chart fill must not introduce a white fallback"
        );

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:noFill/></c:spPr><c:chart>",
            )
            .replace("<c:plotArea>", "<c:plotArea><c:spPr><a:noFill/></c:spPr>");
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::NoFill);
        assert_eq!(frame.plot_fill, FrameFill::NoFill);
        let png = super::render_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let margin = (5 * 576 + 5) * 4;
        assert_eq!(
            buf[margin + 3],
            0,
            "both noFill must keep the chart margin transparent"
        );
        let mut transparent = 0usize;
        for px in buf[..info.buffer_size()].chunks_exact(4) {
            if px[3] == 0 {
                transparent += 1;
                assert_ne!(
                    &px[..3],
                    &[255, 255, 255],
                    "transparent noFill must not use a white key"
                );
            }
        }
        assert!(
            transparent > 1000,
            "both noFill must leave a transparent field, got {transparent}"
        );
    }

    #[test]
    fn chart_and_plot_translucent_fills_keep_straight_alpha() {
        assert_eq!(ooxml_alpha_to_u8(0), 0);
        assert_eq!(ooxml_alpha_to_u8(25000), 64);
        assert_eq!(ooxml_alpha_to_u8(50000), 128);
        assert_eq!(ooxml_alpha_to_u8(100000), 255);

        let xml = LONG_LEGEND.replace(
            "<c:chart>",
            "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='50000'/></a:srgbClr></a:solidFill></c:spPr><c:chart>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::Translucent([234, 247, 247], 50000));
        assert_eq!(frame.plot_fill, FrameFill::Default);
        let png = super::render_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let i = (5 * 576 + 5) * 4;
        assert_eq!(
            &buf[i..i + 3],
            &[234, 247, 247],
            "chart fill must stay unassociated, not flattened onto white"
        );
        assert_eq!(buf[i + 3], 128, "alpha 50000 maps to coverage 128");

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='0F6082'><a:alpha val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
        assert_eq!(
            frame.plot_fill,
            FrameFill::Translucent([15, 96, 130], 50000)
        );
        let png = super::render_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(
            info.color_type,
            png::ColorType::Rgb,
            "translucent plot over an opaque chart stays opaque"
        );
        let px = |x: usize, y: usize| &buf[(y * 576 + x) * 3..(y * 576 + x) * 3 + 3];
        assert_eq!(px(5, 5), &[234, 247, 247]);
        assert!(
            buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[124, 171, 188]),
            "plot alpha 50000 must composite 0F6082 over the chart fill, not fall back to white or an opaque plot color"
        );
        assert!(
            !buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[255, 255, 255]),
            "translucent plot over a non-white chart fill must not introduce a white fallback"
        );
        assert!(
            !buf[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .contains(&[15, 96, 130]),
            "translucent plot must not paint the source color as opaque"
        );

        let xml = LONG_LEGEND.replace(
            "<c:chart>",
            "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='25000'/></a:srgbClr></a:solidFill></c:spPr><c:chart>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert_eq!(
            read_frame(&doc.root).unwrap().fill,
            FrameFill::Translucent([234, 247, 247], 25000)
        );
        let png = super::render_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let i = (5 * 576 + 5) * 4;
        assert_eq!(&buf[i..i + 3], &[234, 247, 247]);
        assert_eq!(buf[i + 3], 64, "alpha 25000 maps to coverage 64");
    }


    #[test]
    fn explicit_width_and_inherited_chart_pen_are_distinct_and_scatter_is_preserved() {
        let dpi=RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap();
        for (attribute,ordinary,scatter) in [("",4.0,4.0),("w='28575'",3.0,4.0),("w='12700'",1.0,2.0)] {
            let doc=crate::xml::parse(format!("<spPr><ln {attribute}/></spPr>").as_bytes()).unwrap();
            assert_eq!(read_line_sp_pr(&doc.root,FALLBACK_ACCENTS,dpi,true).unwrap().width,ordinary);
            assert_eq!(read_line_sp_pr(&doc.root,FALLBACK_ACCENTS,dpi,false).unwrap().width,scatter);
        }
    }

    #[test]
    fn preset_dash_parser_requires_explicit_native_validated_cap_and_pen() {
        for name in ["dash","dashDot","dot","lgDash","lgDashDot","lgDashDotDot","sysDash","sysDot","sysDashDot","sysDashDotDot"] {
            for cap in ["flat","rnd"] {
                let doc=crate::xml::parse(format!("<ln w='50800' cap='{cap}'><prstDash val='{name}'/></ln>").as_bytes()).unwrap();
                let a=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,2.,RenderDpi::default(),true);
                if cap=="flat" || matches!(name,"dash"|"dot") {assert!(a.unwrap().dash.is_some());}else{assert!(a.is_err());}
                assert!(read_solid_ln(&doc.root,FALLBACK_ACCENTS,2.,RenderDpi::default()).is_err());
            }
        }
    }
    #[test]
    fn preset_dash_rejects_unverified_bounds_ambiguity_and_complex_pens() {
        for attrs in ["", "w='50800'", "cap='flat'", "w='0' cap='flat'", "w='NaN' cap='flat'", "w='bad' cap='flat'", "w='50800' cap='sq'"] {
            let doc=crate::xml::parse(format!("<ln {attrs}><prstDash val='dash'/></ln>").as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,2.,RenderDpi::default(),true).is_err());
        }
        for body in ["<prstDash val='unknown'/>","<prstDash val='dash' phase='1'/>","<prstDash val='dash'><x/></prstDash>","<prstDash val='dash'/><prstDash val='solid'/>","<prstDash val='dash'/><custDash><ds d='100000' sp='100000'/></custDash>","<custDash><ds d='100000' sp='100000'/></custDash><prstDash val='dash'/>","<prstDash val='dash'/><noFill/>","<prstDash val='dash'/><solidFill><srgbClr val='175DB5'><alpha val='50000'/></srgbClr></solidFill>","<prstDash val='dash'/><headEnd type='triangle'/>"] {
            let doc=crate::xml::parse(format!("<ln w='50800' cap='flat'>{body}</ln>").as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,2.,RenderDpi::default(),true).is_err(),"{body}");
        }
    }
    #[test]
    fn preset_dash_end_to_end_reuses_custom_plot_legend_and_gap_phase() {
        for (name,stops) in [("dash",vec![(400000,300000)]),("dot",vec![(100000,300000)]),("sysDashDotDot",vec![(300000,100000),(100000,100000),(100000,100000)])] {
            for cap in ["flat","rnd"] {
                if cap=="rnd" && name=="sysDashDotDot" {continue;}
                for ys in [vec![Some(40.);8],vec![Some(40.),Some(40.),Some(60.),Some(40.),Some(40.)],vec![Some(40.),Some(40.),None,Some(40.),Some(40.)]] {
                    let make=|dash:&str|standard_line_xml(&line_ser(0,"H",&ys,&format!("<c:spPr><a:ln w='50800' cap='{cap}'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill>{dash}<a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>")),"");
                    let custom=format!("<a:custDash>{}</a:custDash>",stops.iter().map(|(d,s)|format!("<a:ds d='{d}' sp='{s}'/>")).collect::<String>());let preset=format!("<a:prstDash val='{name}'/>");
                    let a=make(&preset);let b=make(&custom);assert_eq!(render_line(&a).unwrap(),render_line(&b).unwrap());
                    assert_eq!(render_line(&a.replace("<c:legendPos val=\"r\"/>","<c:legendPos val=\"b\"/>")).unwrap(),render_line(&b.replace("<c:legendPos val=\"r\"/>","<c:legendPos val=\"b\"/>")).unwrap());
                }
            }
        }
    }
    #[test]
    fn preset_dash_does_not_change_omitted_or_solid_legacy_styles() {
        for attrs in ["", "w='50800'", "cap='rnd'", "w='50800' cap='flat'"] {
            let make=|dash:&str|crate::xml::parse(format!("<ln {attrs}>{dash}<round/></ln>").as_bytes()).unwrap();let a=make("");let b=make("<prstDash val='solid'/>");let c=make("<prstDash/>");
            for node in [&a,&b,&c] {let style=read_ln_with_caps(&node.root,FALLBACK_ACCENTS,2.,RenderDpi::default(),true).unwrap();assert!(style.dash.is_none());}
        }
    }

    #[test]
    fn positive_flat_custom_dash_parses_percentages_of_the_explicit_device_pen() {
        for (width,device) in [(12700,1.0),(25400,3.0),(50800,6.0)] {
            let doc=crate::xml::parse(format!("<ln w='{width}' cap='flat'><custDash><ds d='200000' sp='100000'/></custDash><round/></ln>").as_bytes()).unwrap();
            let style=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap(),true).unwrap();
            assert_eq!(style.width,device);
            assert_eq!(style.dash.unwrap().painted_intervals(device*5.0,16).unwrap(),[(0.0,device*2.0),(device*3.0,device*5.0)]);
            assert!(read_solid_ln(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap()).is_err());
        }
    }
    #[test]
    fn custom_dash_rejects_unimplemented_caps_missing_width_and_ambiguous_stops() {
        for attrs in ["w='50800'","cap='flat'","w='0' cap='flat'"] {
            let doc=crate::xml::parse(format!("<ln {attrs}><custDash><ds d='200000' sp='100000'/></custDash></ln>").as_bytes()).unwrap();
            assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::default(),true).is_err());
        }
        for body in ["<custDash/>","<custDash offset='1'><ds d='1' sp='1'/></custDash>","<custDash><ds d='0' sp='1'/></custDash>","<custDash><ds d='1.5' sp='1'/></custDash>","<custDash><ds d='1'/></custDash>","<custDash><ds d='2147483648' sp='1'/></custDash>","<custDash><ds d='1' sp='1' other='1'/></custDash>","<prstDash val='solid'/><custDash><ds d='1' sp='1'/></custDash>","<custDash><ds d='1' sp='1'/></custDash><noFill/>","<custDash><ds d='1' sp='1'/></custDash><solidFill><srgbClr val='175DB5'><alpha val='50000'/></srgbClr></solidFill>"] {
            let doc=crate::xml::parse(format!("<ln w='50800' cap='flat'>{body}</ln>").as_bytes()).unwrap();
            assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::default(),true).is_err(),"{body}");
        }
    }
    #[test]
    fn noncustom_join_parser_requires_explicit_positive_simple_pen() {
        for cap in ["flat","rnd","sq"] {
            for dash in ["","<prstDash val='solid'/>"] {
                for (tag,join) in [("<bevel/>",stroke::Join::Bevel),("<miter/>",stroke::Join::Miter {limit:800000}),("<miter lim='100000'/>",stroke::Join::Miter {limit:100000})] {
                    let xml=format!("<ln w='50800' cap='{cap}'><solidFill><srgbClr val='175DB5'/></solidFill>{dash}{tag}</ln>");
                    let doc=crate::xml::parse(xml.as_bytes()).unwrap();let style=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).unwrap();assert_eq!(style.join,join);assert!(style.dash.is_none());
                }
            }
            for bad in ["<miter lim='-1'/>","<miter lim='1.5'/>","<miter lim='2147483648'/>","<bevel lim='1'/>","<round/><bevel/>","<miter><x/></miter>"] {
                let doc=crate::xml::parse(format!("<ln w='50800' cap='{cap}'>{bad}</ln>").as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).is_err(),"{bad}");
            }
        }
    }
    #[test]
    fn noncustom_join_does_not_change_omitted_unresolved_or_other_readers() {
        for attrs in ["w='50800'","cap='rnd'","w='0' cap='rnd'",""] {
            let doc=crate::xml::parse(format!("<ln {attrs}><solidFill><srgbClr val='175DB5'/></solidFill><bevel/></ln>").as_bytes()).unwrap();
            assert_eq!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).unwrap().join,stroke::Join::Round);
        }
        for cap in ["flat","sq"] {
            let doc=crate::xml::parse(format!("<ln cap='{cap}'><bevel/></ln>").as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).is_err());
        }
        for tag in ["<bevel/>","<miter/>"] {
            let doc=crate::xml::parse(format!("<ln w='50800' cap='rnd'>{tag}</ln>").as_bytes()).unwrap();assert_eq!(read_solid_ln(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default()).unwrap().join,stroke::Join::Round);
        }
    }
    #[test]
    fn noncustom_join_reuses_full_run_geometry_without_dash_budget() {
        for cap in [stroke::Cap::Flat,stroke::Cap::Round,stroke::Cap::Square] {
            for join in [stroke::Join::Bevel,stroke::Join::Miter {limit:800000}] {
                let pts=[(10.0,40.0),(20.0,40.0),(40.0,10.0),(60.0,40.0),(70.0,40.0)];let style=stroke::Style::new(6.0,cap).unwrap();let p=stroke::DashPattern::from_width_percent(&[(100000000.0,100000.0)],6.0).unwrap();let mut solid=Canvas::new(90,60);solid.custom_dash_budget=0;let mut custom=Canvas::new(90,60);
                stroke_run_pattern_join(&mut solid,&pts,style,None,join,[23,93,181]).unwrap();stroke_run_pattern_join(&mut custom,&pts,style,Some(&p),join,[23,93,181]).unwrap();assert_eq!(solid.px,custom.px);assert_eq!(solid.alpha,custom.alpha);assert_eq!(solid.custom_dash_budget,0);
            }
        }
    }
    #[test]
    fn noncustom_join_keeps_legacy_round_and_single_edge_pixels() {
        for cap in [stroke::Cap::Flat,stroke::Cap::Round,stroke::Cap::Square] {
            let style=stroke::Style::new(6.0,cap).unwrap();let mut a=Canvas::new(90,60);let mut b=Canvas::new(90,60);let pts=[(10.0,40.0),(40.0,10.0),(70.0,40.0)];
            stroke_run_style(&mut a,&pts,style,[23,93,181]);stroke_run_pattern_join(&mut b,&pts,style,None,stroke::Join::Round,[23,93,181]).unwrap();assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);
            for join in [stroke::Join::Bevel,stroke::Join::Miter {limit:800000}] {
                let mut a=Canvas::new(90,60);let mut b=Canvas::new(90,60);let pts=[(10.25,20.25),(70.25,30.25)];stroke_run_style(&mut a,&pts,style,[23,93,181]);stroke_run_pattern_join(&mut b,&pts,style,None,join,[23,93,181]).unwrap();assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);
            }
        }
    }
    #[test]
    fn noncustom_join_round_trip_honors_previously_ignored_round_cap_join() {
        let make=|tag:&str,dash:&str| {
            let extra=format!("<c:spPr><a:ln w='101600' cap='rnd'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill>{dash}{tag}</a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");standard_line_xml(&line_ser(0,"H",&[Some(10.0),Some(10.0),Some(90.0),Some(10.0),Some(10.0)],&extra),"")
        };
        let round=render_line(&make("<a:round/>","<a:prstDash val='solid'/>")).unwrap();let bevel=render_line(&make("<a:bevel/>","<a:prstDash val='solid'/>")).unwrap();let miter=render_line(&make("<a:miter/>","<a:prstDash val='solid'/>")).unwrap();assert_ne!(round,bevel);assert_ne!(bevel,miter);assert_ne!(round,miter);
        assert_eq!(round,render_line(&make("","<a:prstDash val='solid'/>")).unwrap());assert_eq!(bevel,render_line(&make("<a:bevel/>","")).unwrap());assert_eq!(bevel,render_line(&make("<a:miter lim='100000'/>","")).unwrap());
    }
    #[test]
    fn noncustom_join_true_gaps_are_separate_runs_and_complex_pens_stay_gated() {
        for cap in ["flat","rnd","sq"] {
            let extra=format!("<c:spPr><a:ln w='101600' cap='{cap}'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:prstDash val='solid'/><a:miter/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
            let gap=standard_line_xml(&line_ser(0,"H",&[Some(10.0),Some(10.0),Some(90.0),None,Some(90.0),Some(10.0),Some(10.0),Some(10.0)],&extra),"");let connected=standard_line_xml(&line_ser(0,"H",&[Some(10.0),Some(10.0),Some(90.0),Some(90.0),Some(90.0),Some(10.0),Some(10.0),Some(10.0)],&extra),"");assert_ne!(render_line(&gap).unwrap(),render_line(&connected).unwrap());
            for fill in ["<noFill/>","<solidFill><srgbClr val='175DB5'><alpha val='50000'/></srgbClr></solidFill>"] {
                let xml=format!("<ln w='50800' cap='{cap}'>{fill}<miter/></ln>");let doc=crate::xml::parse(xml.as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).is_err());
            }
        }
        let mut c=Canvas::new(100,60);let style=stroke::Style::new(6.0,stroke::Cap::Square).unwrap();for pts in [&[(10.0,40.0),(35.0,10.0)][..],&[(65.0,10.0),(90.0,40.0)][..]] {stroke_run_pattern_join(&mut c,pts,style,None,stroke::Join::Miter {limit:800000},[23,93,181]).unwrap();}assert_eq!(&c.px[(10*100+50)*3..(10*100+51)*3],&[255,255,255]);
    }
    #[test]
    fn miter_parser_validates_limits_and_preserves_legacy_readers() {
        for cap in ["flat","rnd","sq"] {
            let make=|extra:&str|format!("<ln w='50800' cap='{cap}'><solidFill><srgbClr val='175DB5'/></solidFill><custDash><ds d='10000000' sp='100000'/></custDash>{extra}</ln>");
            for (tag,expected) in [("<miter/>",800000),("<miter lim='0'/>",0),("<miter lim='200000'/>",200000),("<miter lim='2147483647'/>",2147483647)] {
                let doc=crate::xml::parse(make(tag).as_bytes()).unwrap();let style=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).unwrap();assert_eq!(style.join,stroke::Join::Miter {limit:expected});
            }
            if cap!="rnd" {
                let solid=make("<miter/>").replace("<custDash><ds d='10000000' sp='100000'/></custDash>","");
                let doc=crate::xml::parse(solid.as_bytes()).unwrap();assert_eq!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).unwrap().join,stroke::Join::Miter {limit:800000});
            }
            for bad in ["<miter lim='-1'/>","<miter lim='1.5'/>","<miter lim='NaN'/>","<miter lim='2147483648'/>","<miter lim='800%'/>","<miter other='1'/>","<miter><x/></miter>","<miter/><round/>"] {
                let doc=crate::xml::parse(make(bad).as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).is_err(),"{bad}");
            }
        }
        let legacy=crate::xml::parse(b"<ln w='50800' cap='rnd'><solidFill><srgbClr val='175DB5'/></solidFill><miter/></ln>").unwrap();
        assert_eq!(read_solid_ln(&legacy.root,FALLBACK_ACCENTS,6.0,RenderDpi::default()).unwrap().join,stroke::Join::Round);
    }
    #[test]
    fn miter_true_tip_is_not_clipped_to_half_width_mask() {
        let points=[(30.0,70.0),(40.0,30.0),(50.0,70.0)];let mut m=Canvas::new(80,80);let mut b=Canvas::new(80,80);
        m.stroke_custom_dash_union_join(&points,12.0,stroke::Cap::Flat,stroke::Cap::Flat,stroke::Join::Miter {limit:800000},[23,93,181]);
        b.stroke_custom_dash_union_join(&points,12.0,stroke::Cap::Flat,stroke::Cap::Flat,stroke::Join::Bevel,[23,93,181]);
        let pixel=|c:&Canvas,x:usize,y:usize|c.px[(y*80+x)*3..(y*80+x+1)*3].to_vec();
        assert_eq!(pixel(&m,40,12),vec![23,93,181]);assert_eq!(pixel(&b,40,12),vec![255,255,255]);
        let mut limited=Canvas::new(80,80);limited.stroke_custom_dash_union_join(&points,12.0,stroke::Cap::Flat,stroke::Cap::Flat,stroke::Join::Miter {limit:200000},[23,93,181]);assert_eq!(limited.px,b.px);
    }
    #[test]
    fn miter_limit_round_trip_and_default_are_vertex_local() {
        let make=|tag:&str| {
            let extra=format!("<c:spPr><a:ln w='101600' cap='flat'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='10000000' sp='100000'/></a:custDash>{tag}</a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
            standard_line_xml(&line_ser(0,"H",&[Some(10.0),Some(10.0),Some(90.0),Some(10.0),Some(10.0)],&extra),"")
        };
        let bevel=render_line(&make("<a:bevel/>")).unwrap();let one=render_line(&make("<a:miter lim='100000'/>")).unwrap();let two=render_line(&make("<a:miter lim='200000'/>")).unwrap();let eight=render_line(&make("<a:miter lim='800000'/>")).unwrap();
        assert_eq!(bevel,one);assert_ne!(bevel,two);assert_ne!(two,eight);assert_eq!(eight,render_line(&make("<a:miter/>")).unwrap());
    }
    #[test]
    fn miter_single_edge_short_dots_budget_and_offcanvas_are_preserved() {
        for cap in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square] {
            for d in [25000.0,100000.0,10000000.0] {
                let p=stroke::DashPattern::from_width_percent(&[(d,100000.0)],6.0).unwrap();let style=stroke::Style::new(6.0,cap).unwrap();let mut a=Canvas::new(100,50);let mut b=Canvas::new(100,50);
                for (canvas,join) in [(&mut a,stroke::Join::Round),(&mut b,stroke::Join::Miter {limit:800000})] {stroke_run_pattern_join(canvas,&[(20.25,20.25),(70.25,20.25)],style,Some(&p),join,[23,93,181]).unwrap();}
                assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);assert_eq!(a.custom_dash_budget,b.custom_dash_budget);
            }
        }
        let p=stroke::DashPattern::from_width_percent(&[(10000000.0,100000.0)],6.0).unwrap();let mut c=Canvas::new(80,60);c.custom_dash_budget=1;
        stroke_run_pattern_join(&mut c,&[(-50.0,40.0),(40.0,-50.0),(130.0,40.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),stroke::Join::Miter {limit:800000},[23,93,181]).unwrap();assert_eq!(c.custom_dash_budget,0);
        let before=c.px.clone();assert!(stroke_run_pattern_join(&mut c,&[(10.0,40.0),(40.0,10.0),(70.0,40.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),stroke::Join::Miter {limit:800000},[23,93,181]).is_err());assert_eq!(before,c.px);
    }
    #[test]
    fn bevel_is_irrelevant_for_a_single_edge_or_round_dot() {
        for cap in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square] {
            for d in [25000.0,100000.0,10000000.0] {
                let pattern=stroke::DashPattern::from_width_percent(&[(d,100000.0)],6.0).unwrap();let style=stroke::Style::new(6.0,cap).unwrap();
                let mut a=Canvas::new(100,50);let mut b=Canvas::new(100,50);
                for (canvas,join) in [(&mut a,stroke::Join::Round),(&mut b,stroke::Join::Bevel)] {
                    stroke_run_pattern_join(canvas,&[(20.25,20.25),(70.25,20.25)],style,Some(&pattern),join,[23,93,181]).unwrap();
                }
                assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);assert_eq!(a.custom_dash_budget,b.custom_dash_budget);
            }
        }
    }
    #[test]
    fn bevel_custom_parser_is_explicit_and_does_not_expand_legacy_solid() {
        for cap in ["flat","sq","rnd"] {
            let xml=format!("<ln w='50800' cap='{cap}'><solidFill><srgbClr val='175DB5'/></solidFill><custDash><ds d='10000000' sp='100000'/></custDash><bevel/></ln>");
            let doc=crate::xml::parse(xml.as_bytes()).unwrap();let style=read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).unwrap();assert_eq!(style.join,stroke::Join::Bevel);
            for bad in ["<miter lim='-1'/>","<bevel lim='1'/>","<bevel><x/></bevel>","<round/><bevel/>","<bevel/><bevel/>"] {
                let doc=crate::xml::parse(xml.replace("<bevel/>",bad).as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default(),true).is_err());
            }
        }
        let doc=crate::xml::parse(b"<ln w='50800' cap='rnd'><solidFill><srgbClr val='175DB5'/></solidFill><bevel/></ln>").unwrap();assert_eq!(read_solid_ln(&doc.root,FALLBACK_ACCENTS,6.0,RenderDpi::default()).unwrap().join,stroke::Join::Round);
    }
    #[test]
    fn bevel_custom_joins_fill_hull_without_round_bulge_or_butt_gap() {
        let pts=[(15.0,40.0),(40.0,15.0),(65.0,40.0)];let mut round=Canvas::new(80,60);let mut bevel=Canvas::new(80,60);
        for (canvas,join) in [(&mut round,stroke::Join::Round),(&mut bevel,stroke::Join::Bevel)] {canvas.stroke_custom_dash_union_join(&pts,14.0,stroke::Cap::Flat,stroke::Cap::Flat,join,[23,93,181]);}
        assert_ne!(round.px,bevel.px);let at=|c:&Canvas,x:usize,y:usize|c.px[(y*80+x)*3..(y*80+x+1)*3].to_vec();
        assert_eq!(at(&bevel,40,15),vec![23,93,181]);assert_eq!(at(&bevel,40,9),vec![255,255,255]);assert_ne!(at(&round,40,9),vec![255,255,255]);
    }
    #[test]
    fn bevel_custom_round_trip_reaches_line_renderer_and_respects_budget() {
        let make=|join:&str| {
            let extra=format!("<c:spPr><a:ln w='101600' cap='flat'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='10000000' sp='100000'/></a:custDash><a:{join}/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
            standard_line_xml(&line_ser(0,"H",&[Some(10.0),Some(10.0),Some(90.0),Some(10.0)],&extra),"")
        };
        assert_ne!(render_line(&make("round")).unwrap(),render_line(&make("bevel")).unwrap());assert_ne!(render_line(&make("miter")).unwrap(),render_line(&make("bevel")).unwrap());
        let mut c=Canvas::new(80,60);c.custom_dash_budget=1;let p=stroke::DashPattern::from_width_percent(&[(10000000.0,100000.0)],6.0).unwrap();
        stroke_run_pattern_join(&mut c,&[(10.0,40.0),(40.0,10.0),(70.0,40.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),stroke::Join::Bevel,[23,93,181]).unwrap();assert_eq!(c.custom_dash_budget,0);
    }
    #[test]
    fn custom_dash_union_retrace_does_not_reblend_boundary() {
        let mut once=Canvas::new(80,60);let mut retrace=Canvas::new(80,60);
        let a=(18.25,20.25);let b=(58.25,20.25);let color=[23,93,181];
        stroke_run_dash_caps(&mut once,&[a,b],6.0,stroke::Cap::Round,stroke::Cap::Round,color);
        stroke_run_dash_caps(&mut retrace,&[a,b,a],6.0,stroke::Cap::Round,stroke::Cap::Round,color);
        assert_eq!(once.px,retrace.px);assert_eq!(once.alpha,retrace.alpha);
    }
    #[test]
    fn custom_dash_union_matches_one_blend_of_max_coverage_at_turn() {
        for cap in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square] {
            let points=[(18.25,40.25),(38.25,12.25),(58.25,40.25)];let width=6.0;let color=[23,93,181];
            let mut actual=Canvas::new(80,60);let mut expected=Canvas::new(80,60);
            stroke_run_dash_caps(&mut actual,&points,width,cap,cap,color);
            let a=stroke::CapSegment::new(points[0],points[1],width,cap,stroke::Cap::Round).unwrap();
            let b=stroke::CapSegment::new(points[1],points[2],width,stroke::Cap::Round,cap).unwrap();
            for y in 0..60 {for x in 0..80 {
                let p=(f64::from(x)+0.5,f64::from(y)+0.5);
                let alpha=coverage(a.signed_distance(p),0.0).max(coverage(b.signed_distance(p),0.0));
                if alpha>0 {expected.blend(x,y,color,alpha);}
            }}
            assert_eq!(actual.px,expected.px);assert_eq!(actual.alpha,expected.alpha);
        }
    }
    #[test]
    fn custom_dash_union_zero_edges_reverse_and_offcanvas_are_stable() {
        let p=[(-30.25,20.25),(30.25,10.25),(90.25,20.25)];let mut reversed=p;reversed.reverse();
        let mut a=Canvas::new(60,40);let mut b=Canvas::new(60,40);let mut c=Canvas::new(60,40);
        for (canvas,points) in [(&mut a,p.as_slice()),(&mut b,reversed.as_slice()),(&mut c,&[p[0],p[0],p[1],p[1],p[2]])] {
            stroke_run_dash_caps(canvas,points,6.0,stroke::Cap::Flat,stroke::Cap::Flat,[23,93,181]);
        }
        assert_eq!(a.px,b.px);assert_eq!(a.px,c.px);
        let before=a.px.clone();a.stroke_custom_dash_union(&[(-100.0,-100.0),(-80.0,-60.0),(-50.0,-100.0)],6.0,stroke::Cap::Round,stroke::Cap::Round,[23,93,181]);assert_eq!(a.px,before);
    }
    #[test]
    fn custom_dash_union_keeps_phase_and_piece_budget_across_true_turns() {
        let mut c=Canvas::new(80,60);c.custom_dash_budget=1;
        let p=stroke::DashPattern::from_width_percent(&[(10000000.0,100000.0)],6.0).unwrap();
        stroke_run_pattern(&mut c,&[(10.0,40.0),(40.0,10.0),(70.0,40.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),[23,93,181]).unwrap();
        assert_eq!(c.custom_dash_budget,0);let before=c.px.clone();
        assert!(stroke_run_pattern(&mut c,&[(10.0,50.0),(30.0,50.0)],stroke::Style::new(6.0,stroke::Cap::Flat).unwrap(),Some(&p),[23,93,181]).is_err());assert_eq!(c.px,before);
    }

    #[test]
    fn round_dot_raster_reuses_one_existing_circle_and_is_invariant_below_one_pen() {
        let mut expected=None;
        for d in [25000.0,50000.0,100000.0] {
            let p=stroke::DashPattern::from_width_percent(&[(d,100000.0)],6.0).unwrap();let mut c=Canvas::new(100,40);
            stroke_run_pattern(&mut c,&[(20.0,20.0),(70.0,20.0)],stroke::Style::new(6.0,stroke::Cap::Round).unwrap(),Some(&p),[23,93,181]).unwrap();
            if let Some(ref pixels)=expected {assert_eq!(&c.px,pixels);}else{expected=Some(c.px.clone());}
            let mut primitive=Canvas::new(100,40);
            for x in [20.0,32.0,44.0,56.0,68.0] {primitive.stroke_segment(x,20.0,x,20.0,6.0,[23,93,181]);}
            assert_eq!(c.px,primitive.px);assert_eq!(c.alpha,primitive.alpha);
        }
    }
    #[test]
    fn round_dot_is_not_a_closed_polyline_and_uses_the_shared_budget() {
        let p=stroke::DashPattern::from_width_percent(&[(50000.0,100000.0)],2.0).unwrap();let mut c=Canvas::new(100,50);c.custom_dash_budget=2;
        let style=stroke::Style::new(2.0,stroke::Cap::Round).unwrap();
        stroke_run_pattern(&mut c,&[(20.0,20.0),(25.0,20.0)],style,Some(&p),[23,93,181]).unwrap();assert_eq!(c.custom_dash_budget,0);
        let before=c.px.clone();assert!(stroke_run_pattern(&mut c,&[(30.0,20.0),(35.0,20.0)],style,Some(&p),[23,93,181]).is_err());assert_eq!(c.px,before);
        let mut looped=Canvas::new(100,50);stroke_run_dash_caps(&mut looped,&[(20.0,20.0),(40.0,20.0),(20.0,20.0)],6.0,stroke::Cap::Round,stroke::Cap::Round,[23,93,181]);
        assert_eq!(&looped.px[(20*100+35)*3..(20*100+36)*3],&[23,93,181]);
    }
    #[test]
    fn positive_round_dots_reach_legacy_and_advanced_legend_keys_without_enabling_zero_stops() {
        for pos in ["r","b"] {
            let make=|d| {
                let extra=format!("<c:spPr><a:ln w='50800' cap='rnd'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='{d}' sp='100000'/></a:custDash><a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
                standard_line_xml(&line_ser(0,"H",&[Some(40.0);8],&extra),"").replace("<c:legendPos val=\"r\"/>",&format!("<c:legendPos val=\"{pos}\"/>"))
            };
            assert_eq!(render_line(&make(25000)).unwrap(),render_line(&make(100000)).unwrap());assert!(render_line(&make(0)).is_err());
        }
    }

    #[test]
    fn explicit_round_square_custom_dash_parser_supports_only_evidenced_baseline() {
        for cap in ["rnd","sq","flat"] {
            let make=|extra:&str,d:i32| format!("<ln w='50800' cap='{cap}'><solidFill><srgbClr val='175DB5'/></solidFill><custDash><ds d='{d}' sp='100000'/></custDash>{extra}</ln>");
            let parse=|xml:String| {let doc=crate::xml::parse(xml.as_bytes()).unwrap();read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,4.0,RenderDpi::default(),true)};
            assert!(parse(make("<round/>",200000)).is_ok());
            assert!(parse(make("<bevel/>",200000)).is_ok());
            assert!(parse(make("<miter/>",200000)).is_ok());
            let alpha=make("",200000).replace("val='175DB5'/>","val='175DB5'><alpha val='50000'/></srgbClr>");
            assert!(parse(alpha).is_err());
            for d in [1,25000,50000,100000] {assert!(parse(make("",d)).is_ok());}
        }
    }
    #[test]
    fn round_square_custom_raster_does_not_depend_on_collinear_tessellation() {
        for cap in [stroke::Cap::Round,stroke::Cap::Square] {
            let p=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
            let style=stroke::Style::new(6.0,cap).unwrap();let mut a=Canvas::new(120,40);let mut b=Canvas::new(120,40);
            stroke_run_pattern(&mut a,&[(20.0,20.0),(100.0,20.0)],style,Some(&p),[23,93,181]).unwrap();
            stroke_run_pattern(&mut b,&[(20.0,20.0),(23.0,20.0),(54.0,20.0),(100.0,20.0)],style,Some(&p),[23,93,181]).unwrap();
            assert_eq!(a.px,b.px);assert_eq!(a.alpha,b.alpha);
        }
    }
    #[test]
    fn square_internal_dash_is_flat_while_true_run_ends_extend_half_a_pen() {
        let p=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],6.0).unwrap();
        let mut flat=Canvas::new(120,40);let mut square=Canvas::new(120,40);
        for (c,cap) in [(&mut flat,stroke::Cap::Flat),(&mut square,stroke::Cap::Square)] {
            stroke_run_pattern(c,&[(20.0,20.0),(100.0,20.0)],stroke::Style::new(6.0,cap).unwrap(),Some(&p),[23,93,181]).unwrap();
        }
        assert_ne!(flat.px,square.px);
        // Exclude BOTH actual run endpoints: each legitimately gets a cap.
        for y in 0..40 {assert_eq!(&flat.px[(y*120+25)*3..(y*120+95)*3],&square.px[(y*120+25)*3..(y*120+95)*3]);}
        assert_ne!(&flat.px[(20*120+101)*3..(20*120+102)*3],&square.px[(20*120+101)*3..(20*120+102)*3]);
    }
    #[test]
    fn round_square_custom_keys_render_in_both_legacy_and_advanced_legends() {
        for pos in ["r","b"] {
            let mut images=Vec::new();
            for cap in ["flat","sq","rnd"] {
                let extra=format!("<c:spPr><a:ln w='50800' cap='{cap}'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:custDash><a:ds d='200000' sp='100000'/></a:custDash><a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
                let xml=standard_line_xml(&line_ser(0,"H",&[Some(40.0);8],&extra),"").replace("<c:legendPos val=\"r\"/>",&format!("<c:legendPos val=\"{pos}\"/>"));
                images.push(render_line(&xml).unwrap());
            }
            assert_ne!(images[0],images[1]);assert_ne!(images[0],images[2]);assert_ne!(images[1],images[2]);
        }
    }

    #[test]
    fn patterned_strokes_share_an_image_budget_and_never_silently_truncate() {
        let pattern=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],2.0).unwrap();
        let style=stroke::Style::new(2.0,stroke::Cap::Flat).unwrap();
        let mut c=Canvas::new(80,40);c.custom_dash_budget=2;
        stroke_run_pattern(&mut c,&[(20.0,20.0),(30.0,20.0)],style,Some(&pattern),[23,93,181]).unwrap();
        assert_eq!(c.custom_dash_budget,0);
        let pixel=|c:&Canvas,x:usize| &c.px[(20*80+x)*3..(20*80+x)*3+3]==[23,93,181];
        assert!(pixel(&c,21));assert!(!pixel(&c,25));assert!(pixel(&c,27));
        let before=c.px.clone();
        assert!(stroke_run_pattern(&mut c,&[(40.0,20.0),(50.0,20.0)],style,Some(&pattern),[23,93,181]).is_err());
        assert_eq!(before,c.px);
    }

    #[test]
    fn custom_flat_raster_is_invariant_to_collinear_subdivision_without_touching_legacy_solids() {
        let pattern=stroke::DashPattern::from_width_percent(&[(200000.0,100000.0)],1.0).unwrap();
        let style=stroke::Style::new(1.0,stroke::Cap::Flat).unwrap();
        let mut a=Canvas::new(80,40);let mut b=Canvas::new(80,40);
        stroke_run_pattern(&mut a,&[(20.0,20.0),(60.0,20.0)],style,Some(&pattern),[23,93,181]).unwrap();
        stroke_run_pattern(&mut b,&[(20.0,20.0),(23.5,20.0),(41.5,20.0),(60.0,20.0)],style,Some(&pattern),[23,93,181]).unwrap();
        assert_eq!(a.px,b.px);
    }
    #[test]
    fn flat_custom_dash_and_actual_pen_are_painted_in_advanced_legend_too() {
        let make=|dash:&str| standard_line_xml(&line_ser(0,"H",&[Some(40.0);8],&format!("<c:spPr><a:ln w='50800' cap='flat'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill>{dash}<a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>")),"")
            .replace("<c:legendPos val=\"r\"/>","<c:legendPos val=\"b\"/>");
        let (_,_,solid)=png_rgb(&render_line(&make("<a:prstDash val='solid'/>")).unwrap());
        let (_,_,dashed)=png_rgb(&render_line(&make("<a:custDash><a:ds d='200000' sp='100000'/></a:custDash>")).unwrap());
        let a=count_in(&solid,576,0,576,300,336,[23,93,181],0);
        let b=count_in(&dashed,576,0,576,300,336,[23,93,181],0);
        assert!(b>0 && b<a,"the advanced legend must show a real dashed key: solid={a},dash={b}");
    }

    #[test]
    fn explicit_line_caps_and_key_width_use_shared_style_but_other_strokes_stay_gated() {
        let mut images=Vec::new();
        for cap in ["rnd","flat","sq"] {
            let extra=format!("<c:spPr><a:ln w='50800' cap='{cap}' cmpd='sng' algn='ctr'><a:solidFill><a:srgbClr val='175DB5'/></a:solidFill><a:prstDash val='solid'/><a:round/></a:ln></c:spPr><c:marker><c:symbol val='none'/></c:marker>");
            let xml=standard_line_xml(&line_ser(0,"H",&[Some(40.0),Some(40.0),Some(40.0)],&extra),"");images.push(render_line(&xml).unwrap());
            let node=crate::xml::parse(format!("<a:ln xmlns:a='a' cap='{cap}' w='50800'><a:prstDash val='solid'/><a:round/></a:ln>").as_bytes()).unwrap();
            let style=read_ln_with_caps(&node.root,FALLBACK_ACCENTS,2.0,RenderDpi::default(),true).unwrap();assert_eq!(style.cap,stroke::Cap::from_ooxml(cap).unwrap());
            if cap!="rnd" {assert!(read_solid_ln(&node.root,FALLBACK_ACCENTS,2.0,RenderDpi::default()).is_err());}
        }
        assert_ne!(images[0],images[1]);assert_ne!(images[0],images[2]);assert_ne!(images[1],images[2]);
        for body in ["<a:prstDash val='dash'/>","<a:custDash><a:ds d='200000' sp='100000'/></a:custDash>","<a:headEnd type='triangle'/>","<a:miter/>","<a:solidFill><a:srgbClr val='175DB5'><a:alpha val='50000'/></a:srgbClr></a:solidFill>"] {
            let doc=crate::xml::parse(format!("<a:ln xmlns:a='a' cap='flat'>{body}</a:ln>").as_bytes()).unwrap();assert!(read_ln_with_caps(&doc.root,FALLBACK_ACCENTS,2.0,RenderDpi::default(),true).is_err());
        }
        let mut thin=Canvas::new(90,40);let mut thick=Canvas::new(90,40);
        for (canvas,width) in [(&mut thin,1.0),(&mut thick,6.0)] {draw_plot_key_styled(canvas,PlotKind::StandardLine,20,10,49,19,[0,0,0],MarkerKind::None,stroke::Style::new(width,stroke::Cap::Flat),None).unwrap();}
        assert_ne!(thin.px,thick.px);
    }

    #[test]
    fn duplicate_endpoint_edges_do_not_steal_nonround_run_caps() {
        for cap in [stroke::Cap::Flat,stroke::Cap::Square] {
            let mut simple=Canvas::new(80,40);let mut repeated=Canvas::new(80,40);let style=stroke::Style::new(8.0,cap).unwrap();
            stroke_run_style(&mut simple,&[(20.0,20.0),(60.0,20.0)],style,[0,0,0]);
            stroke_run_style(&mut repeated,&[(20.0,20.0),(20.0,20.0),(60.0,20.0),(60.0,20.0)],style,[0,0,0]);assert_eq!(simple.px,repeated.px);
        }
    }

    #[test]
    fn shared_stroke_round_runs_preserve_legacy_pixels_exactly() {
        for width in [1.0, 2.25, 4.0, 7.5] {
            for points in [vec![(10.0,20.0),(40.0,20.0),(40.0,50.0),(90.0,50.0)], vec![(10.0,20.0),(10.0,20.0),(90.0,50.0)], vec![(-5.0,10.0),(50.5,40.5),(130.0,10.0)]] {
                let mut before=Canvas::new(120,70);let mut after=Canvas::new(120,70);
                for pair in points.windows(2) { before.stroke_segment(pair[0].0,pair[0].1,pair[1].0,pair[1].1,width,[23,93,181]); }
                stroke_run(&mut after,&points,width,[23,93,181]);assert_eq!(before.px,after.px);assert_eq!(before.alpha,after.alpha);
            }
        }
    }

    #[test]
    fn explicit_cap_prerequisites_render_document_contract_not_native_gold() {
        let mut image=Canvas::new(280,180);
        for (i,cap) in [stroke::Cap::Round,stroke::Cap::Flat,stroke::Cap::Square].into_iter().enumerate() {
            let y=25.0+i as f64*55.0;
            stroke_run_style(&mut image,&[(30.0,y),(130.0,y),(180.0,y+20.0),(240.0,y+20.0)],stroke::Style::new(12.0,cap).unwrap(),[23,93,181]);
        }
        let mut round=Canvas::new(80,30);let mut flat=Canvas::new(80,30);let mut square=Canvas::new(80,30);
        for (canvas,cap) in [(&mut round,stroke::Cap::Round),(&mut flat,stroke::Cap::Flat),(&mut square,stroke::Cap::Square)] {stroke_run_style(canvas,&[(20.0,15.0),(60.0,15.0)],stroke::Style::new(8.0,cap).unwrap(),[0,0,0]);}
        assert_ne!(round.px,flat.px);assert_ne!(round.px,square.px);assert_ne!(flat.px,square.px);
        if let Some(dir)=std::env::var_os("OOXP_STROKE_FOUNDATION_REVIEW_DIR") {
            let dir=std::path::PathBuf::from(dir);assert!(dir.is_dir());let path=dir.join("document-contract-cap-primitives.png");assert!(!path.exists());std::fs::write(path,image.encode_png().unwrap()).unwrap();
        }
    }

    #[test]
    fn two_stop_linear_frame_fills_use_office_gamma_weight() {
        let xml = LONG_LEGEND.replace(
            "<c:chart>",
            "<c:roundedCorners val='0'/><c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='0' scaled='1'/><a:tileRect/></a:gradFill></c:spPr><c:chart>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert_eq!(
            frame.fill,
            FrameFill::Linear(LinearFrameGrad {
                stops: gradient::StopTable::new(vec![(0, [234, 247, 247]), (100_000, [15, 96, 130])]).unwrap(),
                ang: LINEAR_ANG_HORIZONTAL,
                path: None,
            })
        );
        let png = super::render_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let stride = if info.color_type == png::ColorType::Rgba {
            4
        } else {
            3
        };
        let mid = (8 * 424 + 212) * stride;
        assert_eq!(
            &buf[mid..mid + 3],
            &[171, 190, 199],
            "geometric mid must follow gamma 2.2 plus the Office weight curve, not an sRGB lerp"
        );
        let left = (8 * 424 + 5) * stride;
        assert_eq!(&buf[left..left + 3], &[234, 247, 247]);

        let xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='0F6082'/></a:gs><a:gs pos='100000'><a:srgbClr val='EAF7F7'/></a:gs></a:gsLst><a:lin ang='5400000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>",
            );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert_eq!(
            read_frame(&doc.root).unwrap().plot_fill,
            FrameFill::Linear(LinearFrameGrad {
                stops: gradient::StopTable::new(vec![(0, [15, 96, 130]), (100_000, [234, 247, 247])]).unwrap(),
                ang: LINEAR_ANG_VERTICAL,
                path: None,
            })
        );
        assert!(super::render_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .is_ok());
    }

    const OFFICE16_THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="0E2841"/></a:dk2>
      <a:lt2><a:srgbClr val="E8E8E8"/></a:lt2>
      <a:accent1><a:srgbClr val="156082"/></a:accent1>
      <a:accent2><a:srgbClr val="E97132"/></a:accent2>
      <a:accent3><a:srgbClr val="196B24"/></a:accent3>
      <a:accent4><a:srgbClr val="0F9ED5"/></a:accent4>
      <a:accent5><a:srgbClr val="A02B93"/></a:accent5>
      <a:accent6><a:srgbClr val="4EA72E"/></a:accent6>
      <a:hlink><a:srgbClr val="467886"/></a:hlink>
      <a:folHlink><a:srgbClr val="96607D"/></a:folHlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;

    #[test]
    fn stop_table_axis_cache_preserves_all_legacy_pixels_exactly() {
        let start = [234, 247, 247];
        let end = [15, 96, 130];
        for (width, height) in [(2, 3), (17, 9), (424, 265), (4096, 3)] {
            for ang in [LINEAR_ANG_HORIZONTAL, LINEAR_ANG_VERTICAL] {
                for clip in [false, true] {
                    let (x0, y0, x1, y1) = if clip { (-5, -2, width + 7, height + 3) } else { (0, 0, width, height) };
                    let grad = LinearFrameGrad { stops: gradient::StopTable::new(vec![(0, start), (100_000, end)]).unwrap(), ang, path: None };
                    let mut canvas = Canvas::new(width as u32, height as u32);
                    canvas.fill_linear_rect(x0, y0, x1, y1, &grad);
                    for y in 0..height {
                        for x in 0..width {
                            let along = if ang == LINEAR_ANG_VERTICAL { (f64::from(y - y0) + 0.5) / f64::from(y1 - y0) } else { (f64::from(x - x0) + 0.5) / f64::from(x1 - x0) };
                            let share = office_grad_weight(along);
                            let expected = std::array::from_fn::<_, 3, _>(|i| office_gamma_mix(start[i], end[i], share));
                            let i = ((y * width + x) * 3) as usize;
                            assert_eq!(&canvas.px[i..i + 3], &expected, "{width}x{height} angle={ang} clip={clip} x={x} y={y}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn multistop_frame_and_plot_use_segmented_srgb_not_two_stop_gamma() {
        let stops = "<a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='50000'><a:srgbClr val='0F6082'/></a:gs><a:gs pos='100000'><a:srgbClr val='F04020'/></a:gs>";
        for plot in [false, true] {
            for ang in [0, 5_400_000] {
                let fill = format!("<c:spPr><a:gradFill><a:gsLst>{stops}</a:gsLst><a:lin ang='{ang}' scaled='1'/></a:gradFill></c:spPr>");
                let xml = if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                let frame = read_frame(&doc.root).unwrap();
                let FrameFill::Linear(grad) = (if plot { frame.plot_fill } else { frame.fill }) else { panic!("expected gradient table"); };
                assert_eq!(grad.stops.len(), 3);
                assert_eq!(grad.sample(0.25), [125, 172, 189]);
                assert_eq!(grad.sample(0.5), [15, 96, 130]);
                assert_eq!(grad.sample(0.75), [128, 80, 81]);
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    assert!(render_chart_at(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).is_ok());
                }
            }
        }
        for stops in [
            "<a:gs pos='0'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='0'><a:srgbClr val='000000'/></a:gs><a:gs pos='100000'><a:srgbClr val='FF0000'/></a:gs>",
            "<a:gs pos='10000'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='50000'><a:srgbClr val='000000'/></a:gs><a:gs pos='100000'><a:srgbClr val='FF0000'/></a:gs>",
            "<a:gs pos='0'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='50000'><a:srgbClr val='000000'><a:alpha val='50000'/></a:srgbClr></a:gs><a:gs pos='100000'><a:srgbClr val='FF0000'/></a:gs>",
        ] {
            let fill = format!("<c:spPr><a:gradFill><a:gsLst>{stops}</a:gsLst><a:lin ang='0'/></a:gradFill></c:spPr>");
            let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")).as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_ok(), "close-enough stop tables must parse: {stops}");
            assert!(render_chart_at(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", RenderDpi::DEFAULT).is_ok());
        }
    }

    #[test]
    fn frame_scheme_tint_shade_and_lum_match_official_excel_pixels() {
        let accent1 = [0x15, 0x60, 0x82];
        assert_eq!(frame_tint(accent1, 60_000), [170, 182, 193]);
        assert_eq!(frame_shade(accent1, 75_000), [17, 84, 114]);
        assert_eq!(frame_lum(accent1, Some(75_000), None), [16, 72, 98]);
        assert_eq!(
            frame_lum(accent1, Some(60_000), Some(40_000)),
            [70, 177, 225]
        );
        // Pure 2.2 or gamma-encoded lerp must not be used: they miss official by 1+.
        assert_ne!(
            accent1.map(|c| {
                let lin = (f64::from(c) / 255.0).powf(2.2);
                let mixed = lin * 0.6 + 0.4;
                (mixed.powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
            }),
            [170, 182, 193]
        );
        assert_ne!(tint_rgb(accent1, 0.6), [170, 182, 193]);
        assert_ne!(scale_rgb(accent1, 0.75), [17, 84, 114]);

        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        assert_eq!(scheme_slot("accent1", &scheme), Some(accent1));
        assert_eq!(scheme_slot("dk2", &scheme), Some([0x0E, 0x28, 0x41]));
        assert_eq!(scheme_slot("lt2", &scheme), Some([0xE8, 0xE8, 0xE8]));
        assert_eq!(scheme_slot("dk1", &scheme), Some([0, 0, 0]));
        assert_eq!(scheme_slot("lt1", &scheme), Some([255, 255, 255]));
        // Without the theme, dk2 is not the stale hardcoded 1F497D.
        let fallback = theme_scheme(None, FALLBACK_ACCENTS);
        assert_eq!(scheme_slot("dk2", &fallback), None);
        assert_eq!(scheme_slot("accent1", &fallback), Some(accent1));

        let cases = [
            (
                "<c:spPr><a:solidFill><a:schemeClr val='accent1'/></a:solidFill></c:spPr>",
                FrameFill::Solid(accent1),
            ),
            (
                "<c:spPr><a:solidFill><a:schemeClr val='dk2'/></a:solidFill></c:spPr>",
                FrameFill::Solid([0x0E, 0x28, 0x41]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:tint val='60000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([170, 182, 193]),
            ),
            (
                "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:tint val='60000'/></a:schemeClr></a:solidFill></c:spPr>",
                FrameFill::Solid([170, 182, 193]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:shade val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([17, 84, 114]),
            ),
            (
                "<c:spPr><a:solidFill><a:schemeClr val='accent1'><a:shade val='75000'/></a:schemeClr></a:solidFill></c:spPr>",
                FrameFill::Solid([17, 84, 114]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([16, 72, 98]),
            ),
            (
                "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumMod val='60000'/><a:lumOff val='40000'/></a:srgbClr></a:solidFill></c:spPr>",
                FrameFill::Solid([70, 177, 225]),
            ),
        ];
        for (config, expected) in cases {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame_with(&doc.root, &scheme).unwrap();
            assert_eq!(frame.fill, expected, "parse {config}");
            assert!(super::render_chart_with_theme(
                &doc.root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                RenderDpi::DEFAULT,
                Some(OFFICE16_THEME.as_bytes()),
            )
            .is_ok());
        }

        let plot_xml = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:schemeClr val='accent1'/></a:solidFill></c:spPr>",
            );
        let doc = crate::xml::parse(plot_xml.as_bytes()).unwrap();
        let frame = read_frame_with(&doc.root, &scheme).unwrap();
        assert_eq!(frame.fill, FrameFill::Solid([234, 247, 247]));
        assert_eq!(frame.plot_fill, FrameFill::Solid(accent1));
    }

    #[test]
    fn signed_luminance_offset_is_distinct_from_alpha_percentage() {
        for (raw, expected) in [("0", 0), ("-0", 0), ("40000", 40_000), ("-20000", -20_000), ("100000", 100_000), ("-100000", -100_000)] {
            let xml = crate::xml::parse(format!(r#"<lumOff val="{raw}"/>"#).as_bytes()).unwrap();
            assert_eq!(parse_luminance_offset(&xml.root), Some(expected));
        }
        assert_eq!(parse_ooxml_alpha(Some("-20000")), None);
        assert_eq!(frame_lum([21, 96, 130], None, Some(-100_000)), [0, 0, 0]);
        assert_eq!(frame_lum([21, 96, 130], None, Some(100_000)), [255, 255, 255]);
    }

    #[test]
    fn standalone_luminance_offset_matches_controlled_native_colors() {
        // Six same-title/data/288x180pt fixtures were re-exported readonly by
        // Excel16/DPI106. These are native color probes, not full-image passes.
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base, offset, alpha, expected) in [
            ("srgbClr", "156082", "40000", "", FrameFill::Solid([122, 199, 233])),
            ("srgbClr", "156082", "-20000", "", FrameFill::Solid([7, 31, 42])),
            ("srgbClr", "156082", "100000", "", FrameFill::Solid([255, 255, 255])),
            ("schemeClr", "accent1", "40000", "", FrameFill::Solid([122, 199, 233])),
            ("srgbClr", "156082", "40000", "<a:alpha val='50000'/>", FrameFill::Translucent([122, 199, 233], 50_000)),
        ] {
            let xml = format!("<c:chartSpace xmlns:c='http://schemas.openxmlformats.org/drawingml/2006/chart' xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main'><c:spPr><a:solidFill><a:{kind} val='{base}'><a:lumOff val='{offset}'/>{alpha}</a:{kind}></a:solidFill></c:spPr></c:chartSpace>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(read_frame_with(&doc.root, &scheme).unwrap().fill, expected);
        }
        // Derived algebra regression; preserves existing positive mod+off and
        // extends its offset operand without inventing a transform-order rule.
        assert_eq!(frame_lum([21, 96, 130], Some(100_000), Some(-20_000)), [7, 31, 42]);
    }

    #[test]
    fn luminance_offset_paints_chart_and_plot_pixels_at_three_dpis() {
        for plot in [false, true] {
            for (offset, rgb, opacity, byte_alpha) in [(40_000, [122, 199, 233], 100_000, 255u8), (-20_000, [7, 31, 42], 100_000, 255), (40_000, [122, 199, 233], 50_000, 128)] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumOff val='{offset}'/><a:alpha val='{opacity}'/></a:srgbClr></a:solidFill></c:spPr>");
                let xml = if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let image = render_chart_at(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    let mut decoder = png::Decoder::new(std::io::Cursor::new(image)).read_info().unwrap();
                    let mut buffer = vec![0; decoder.output_buffer_size()];
                    let info = decoder.next_frame(&mut buffer).unwrap();
                    let stride = if info.color_type == png::ColorType::Rgba { 4 } else { 3 };
                    // Plot paint composites over the existing opaque white
                    // chart frame; chart alpha itself stays straight RGBA.
                    let (expected_rgb, expected_alpha) = if plot {
                        (rgb.map(|channel| ((u32::from(channel) * u32::from(byte_alpha) + 255 * (255 - u32::from(byte_alpha)) + 127) / 255) as u8), 255)
                    } else {
                        (rgb, byte_alpha)
                    };
                    let count = buffer[..info.buffer_size()].chunks_exact(stride).filter(|px| px[..3] == expected_rgb && (stride == 3 || px[3] == expected_alpha)).count();
                    assert!(count > 100, "plot={plot} offset={offset} alpha={opacity} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn unsupported_luminance_offset_inputs_do_not_lose_semantics() {
        for transform in ["<a:lumOff/>", "<a:lumOff val='-100001'/>", "<a:lumOff val='100001'/>", "<a:lumOff val='1.5'/>", "<a:lumOff val='+20000'/>", "<a:lumOff val='40000'><a:alpha val='50000'/></a:lumOff>", "<a:lumOff val='40000'/><a:lumOff val='40000'/>", "<a:lumOff val='40000'/><a:satMod val='50000'/>"] {
            let config = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'>{transform}</a:srgbClr></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>")).as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err(), "{transform}");
        }
    }

    #[test]
    fn alpha_off_adds_percentage_points_not_a_multiplier() {
        let c=[21,96,130];
        assert_eq!(frame_alpha_off_fill(c,40_000,10_000),FrameFill::Translucent(c,50_000));
        assert_eq!(frame_alpha_off_fill(c,40_000,-10_000),FrameFill::Translucent(c,30_000));
        assert_eq!(frame_alpha_off_fill(c,100_000,-50_000),FrameFill::Translucent(c,50_000));
    }
    #[test]
    fn alpha_off_clamps_at_transparent_and_opaque_with_wide_safe_arithmetic() {
        let c=[21,96,130];
        assert_eq!(frame_alpha_off_fill(c,40_000,-50_000),FrameFill::Translucent(c,0));
        assert_eq!(frame_alpha_off_fill(c,40_000,100_000),FrameFill::Solid(c));
        assert_eq!(frame_alpha_off_fill(c,100_000,100_000),FrameFill::Solid(c));
        assert_eq!(frame_alpha_off_fill(c,0,i32::MIN),FrameFill::Translucent(c,0));
        assert_eq!(frame_alpha_off_fill(c,100_000,i32::MAX),FrameFill::Solid(c));
    }
    #[test]
    fn alpha_off_signed_fixed_percentage_range_and_structure_are_strict() {
        for v in [-100_000,-50_000,0,50_000,100_000] {
            let xml=format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:alphaOff val='{v}'/></a:srgbClr>");let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_ok());
        }
        for attrs in ["","val='-100001'","val='100001'","val='-2147483648'","val='10%'","val='0.1'","val='10000' other='1'"] {
            let xml=format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:alphaOff {attrs}/></a:srgbClr>");let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{attrs}");
        }
    }
    #[test]
    fn alpha_off_later_absolute_alpha_overrides_but_validates_offset() {
        for off in [-100_000,10_000] {
            let xml=format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:alphaOff val='{off}'/><a:alpha val='40000'/></a:srgbClr>");let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).unwrap(),FrameFill::Translucent([21,96,130],40_000));
        }
        let xml="<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:alphaOff val='100001'/><a:alpha val='40000'/></a:srgbClr>";let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err());
    }
    #[test]
    fn alpha_off_explicit_alpha_operand_stays_structurally_fixed() {
        for a in ["<a:alpha val='100001'/>","<a:alpha val='-1'/>","<a:alpha val='40000' other='1'/>"] {
            let xml=format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{a}<a:alphaOff val='-100000'/></a:srgbClr>");let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err());
        }
    }
    #[test]
    fn alpha_off_theme_preserves_base_rgb_and_uses_existing_coverage() {
        let xml="<a:schemeClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='accent1'><a:alphaOff val='-50000'/></a:schemeClr>";let d=crate::xml::parse(xml.as_bytes()).unwrap();let scheme=theme_scheme(Some(OFFICE16_THEME.as_bytes()),FALLBACK_ACCENTS);assert_eq!(resolve_frame_solid(&d.root,&scheme).unwrap(),FrameFill::Translucent([21,96,130],50_000));assert_eq!(ooxml_alpha_to_u8(50_000),128);
    }
    #[test]
    fn alpha_off_chart_plot_match_existing_alpha_references_at_three_dpis() {
        for plot in [false,true] {
            for (off,reference) in [(-50_000,0),(-10_000,30_000),(10_000,50_000),(100_000,100_000)] {
                let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:alpha val='40000'/><a:alphaOff val='{off}'/></a:srgbClr></a:solidFill></c:spPr>");let reference=format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:alpha val='{reference}'/></a:srgbClr></a:solidFill></c:spPr>");let make=|fill:&str|if plot{LONG_LEGEND.replace("<c:plotArea>",&format!("<c:plotArea>{fill}"))}else{LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))};let d=crate::xml::parse(make(&fill).as_bytes()).unwrap();let reference=crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for v in [96,106,144]{let dpi=RenderDpi::from_request(serde_json::json!({"dpi":v}).as_object().unwrap()).unwrap();let render=|r:&Element|render_chart_at(r,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();assert_eq!(render(&d.root),render(&reference.root),"plot={plot} off={off} dpi={v}");}
            }
        }
    }
    #[test]
    fn alpha_off_unverified_mixed_repeated_and_nested_sequences_remain_ni() {
        for t in ["<a:alphaOff val='-10000'/><a:alphaOff val='10000'/>","<a:alphaMod val='50000'/><a:alphaOff val='100001'/>","<a:hueOff val='600000'/><a:alphaOff val='10000'/>","<a:alphaOff val='10000'><a:alpha val='40000'/></a:alphaOff>","<a:alpha val='40000'/><a:alphaOff val='10000'/><a:alpha val='40000'/>"]{let xml=format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{t}</a:srgbClr>");let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{t}");}
    }

    #[test]
    fn hue_off_signed_angle_range_and_strict_structure() {
        for value in [i32::MIN, -27_000_000, 0, 27_000_000, i32::MAX] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:hueOff val='{value}'/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_ok());
        }
        for attrs in ["", "val='2147483648'", "val='-2147483649'", "val='90deg'", "val='90%'", "val='0.5'", "val='5400000' other='1'"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:hueOff {attrs}/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{attrs}");
        }
    }

    #[test]
    fn hue_off_units_signed_rotation_and_complete_turns() {
        assert_eq!(frame_hue_offset([255, 0, 0], 60 * 60_000), [255, 255, 0]);
        assert_eq!(frame_hue_offset([255, 0, 0], -120 * 60_000), [0, 0, 255]);
        assert_eq!(frame_hue_offset([21, 96, 130], -360 * 60_000), [21, 96, 130]);
    }

    #[test]
    fn hue_off_excel_native_profile_does_not_clamp_overflow_to_red() {
        assert_eq!(frame_hue_offset([210, 126, 74], -90 * 60_000), [194, 74, 210]);
        assert_eq!(frame_hue_offset([210, 126, 74], 450 * 60_000), [90, 210, 74]);
        assert_ne!(frame_hue_offset([21, 96, 130], -360 * 60_000), [130, 21, 21]);
    }

    #[test]
    fn hue_off_achromatic_identity_and_large_signed_angles_stay_stable() {
        for rgb in [[0,0,0], [128,128,128], [255,255,255]] {
            for off in [i32::MIN, -27_000_000, 0, 27_000_000, i32::MAX] {
                assert_eq!(frame_hue_offset(rgb, off), rgb);
            }
        }
        for rgb in [[21,96,130], [210,126,74]] { assert_eq!(frame_hue_offset(rgb, 0), rgb); }
    }

    #[test]
    fn hue_off_theme_and_srgb_share_the_same_angle_semantics() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<a:{kind} xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='{base}'><a:hueOff val='27000000'/></a:{kind}>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &scheme).unwrap(), FrameFill::Solid(frame_hue_offset([21,96,130],27_000_000)));
        }
    }

    #[test]
    fn hue_off_trailing_alpha_reuses_legacy_opacity() {
        let xml = "<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:hueOff val='5400000'/><a:alpha val='50000'/></a:srgbClr>";
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert_eq!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).unwrap(), FrameFill::Translucent(frame_hue_offset([21,96,130],5_400_000),50000));
    }

    #[test]
    fn hue_off_chart_plot_at_three_dpis_match_explicit_color_references() {
        for plot in [false,true] {
            for offset in [-27_000_000,5_400_000,27_000_000] {
                // Integration locks existing HSL quantization, not a fitted native RGB.
                let c=frame_hue_offset([21,96,130],offset);let rgb=format!("{:02X}{:02X}{:02X}",c[0],c[1],c[2]);
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:hueOff val='{offset}'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill:&str| if plot { LONG_LEGEND.replace("<c:plotArea>",&format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")) };
                let doc=crate::xml::parse(make(&fill).as_bytes()).unwrap();let reference=crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96,106,144] {
                    let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();
                    assert_eq!(render(&doc.root),render(&reference.root),"plot={plot} offset={offset} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn hue_off_unverified_mixed_repeated_or_nested_transforms_remain_ni() {
        for transforms in ["<a:hueOff val='5400000'/><a:hueOff val='5400000'/>","<a:alpha val='50000'/><a:hueOff val='5400000'/>","<a:hueOff val='5400000'/><a:alphaMod val='50000'/>","<a:hueMod val='50000'/><a:hueOff val='5400000'/>","<a:hueOff val='5400000'><a:alpha val='50000'/></a:hueOff>"] {
            let xml=format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");let doc=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&doc.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{transforms}");
        }
    }

    #[test]
    fn alpha_mod_standalone_uses_opaque_input_and_caps_at_one() {
        let rgb = [21, 96, 130];
        for (factor, expected) in [(0, FrameFill::Translucent(rgb, 0)), (25_000, FrameFill::Translucent(rgb, 25_000)), (50_000, FrameFill::Translucent(rgb, 50_000)), (100_000, FrameFill::Solid(rgb)), (200_000, FrameFill::Solid(rgb)), (i32::MAX as u32, FrameFill::Solid(rgb))] {
            assert_eq!(frame_alpha_mod_fill(rgb, 100_000, factor), expected);
        }
    }

    #[test]
    fn alpha_mod_multiplies_explicit_input_before_clamping() {
        let rgb = [21, 96, 130];
        assert_eq!(frame_alpha_mod_fill(rgb, 40_000, 50_000), FrameFill::Translucent(rgb, 20_000));
        assert_eq!(frame_alpha_mod_fill(rgb, 40_000, 200_000), FrameFill::Translucent(rgb, 80_000));
        assert_eq!(frame_alpha_mod_fill(rgb, 50_000, 200_000), FrameFill::Solid(rgb));
        assert_eq!(frame_alpha_mod_fill(rgb, 0, i32::MAX as u32), FrameFill::Translucent(rgb, 0));
    }

    #[test]
    fn alpha_mod_fractional_products_avoid_intermediate_percentage_rounding() {
        let rgb = [21, 96, 130];
        assert_eq!(frame_alpha_mod_fill(rgb, 50_000, 33_333), FrameFill::AlphaProduct(rgb, 1_666_650_000));
        assert_eq!(frame_alpha_mod_fill(rgb, 50_001, 33_333), FrameFill::AlphaProduct(rgb, 1_666_683_333));
        assert_eq!(alpha_product_to_u8(1_666_650_000), 42);
        assert_eq!(alpha_product_to_u8(1_666_683_333), 43);
        assert_eq!(alpha_product_to_u8(0), 0);
        assert_eq!(alpha_product_to_u8(ALPHA_PRODUCT_DENOM), 255);
    }

    #[test]
    fn alpha_mod_ordered_absolute_alpha_overrides_modulation() {
        for factor in [50_000, 200_000] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:alphaMod val='{factor}'/><a:alpha val='40000'/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).unwrap(), FrameFill::Translucent([21, 96, 130], 40_000));
        }
    }

    #[test]
    fn alpha_mod_resolves_srgb_and_theme_without_changing_rgb() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<a:{kind} xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='{base}'><a:alphaMod val='50000'/></a:{kind}>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &scheme).unwrap(), FrameFill::Translucent([21, 96, 130], 50_000));
        }
    }

    #[test]
    fn alpha_mod_strict_inputs_reject_negative_and_extra_structure() {
        for transforms in ["<a:alphaMod/>", "<a:alphaMod val='-1'/>", "<a:alphaMod val='2147483648'/>", "<a:alphaMod val='+50000'/>", "<a:alphaMod val='50%'/>", "<a:alphaMod val='0.5'/>", "<a:alphaMod val='50000' other='1'/>", "<a:alphaMod val='50000'><a:alpha val='50000'/></a:alphaMod>", "<a:alpha val='100001'/><a:alphaMod val='50000'/>", "<a:alpha val='50000' other='1'/><a:alphaMod val='50000'/>", "<a:alphaMod val='-1'/><a:alpha val='50000'/>"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{transforms}");
        }
    }

    #[test]
    fn alpha_mod_chart_and_plot_fractional_products_match_coverage_at_three_dpis() {
        for plot in [false, true] {
            for (initial, reference) in [(50_000, 16_666), (50_001, 16_667)] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:alpha val='{initial}'/><a:alphaMod val='33333'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:alpha val='{reference}'/></a:srgbClr></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} initial={initial} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn alpha_mod_straight_rgba_keeps_color_and_final_coverage() {
        let fill = "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:alpha val='50001'/><a:alphaMod val='33333'/></a:srgbClr></a:solidFill></c:spPr>";
        let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")).as_bytes()).unwrap();
        let image = render_chart_at(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap()).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(image)).read_info().unwrap();
        let mut buffer = vec![0; decoder.output_buffer_size()];
        let info = decoder.next_frame(&mut buffer).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert!(buffer[..info.buffer_size()].chunks_exact(4).filter(|px| *px == [21, 96, 130, 43]).count() > 100);
    }

    #[test]
    fn alpha_mod_unverified_other_color_sequences_remain_unsupported() {
        for transforms in ["<a:alphaMod val='50000'/><a:alphaMod val='-1'/>", "<a:alpha val='50000'/><a:alphaMod val='50000'/><a:alphaMod val='50000'/>", "<a:hueMod val='50000'/><a:alphaMod val='50000'/>", "<a:alphaMod val='50000'/><a:satMod val='50000'/>", "<a:lumOff val='10000'/><a:alphaMod val='50000'/>"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{transforms}");
        }
    }

    #[test]
    fn hue_percentage_accepts_zero_and_modulation_above_one_turn() {
        for value in [0, 50_000, 100_000, 200_000, 300_000, i32::MAX] {
            let doc = crate::xml::parse(format!("<hueMod val='{value}'/>").as_bytes()).unwrap();
            assert_eq!(parse_positive_percentage(&doc.root), Some(value as u32));
        }
        let negative = crate::xml::parse(b"<satMod val='-50000'/>").unwrap();
        assert_eq!(parse_signed_percentage(&negative.root), Some(-50_000), "signed saturation is unchanged");
        assert_eq!(parse_positive_percentage(&negative.root), None);
    }

    #[test]
    fn hue_percentage_rejects_negative_and_unimplemented_lexical_structure() {
        for xml in ["<hueMod/>", "<hueMod val='-1'/>", "<hueMod val='2147483648'/>", "<hueMod val='+50000'/>", "<hueMod val='50%'/>", "<hueMod val='0.5'/>", "<hueMod val=' 50000'/>", "<hueMod val=''/>", "<hueMod val='50000' other='1'/>", "<hueMod val='50000'><alpha val='50000'/></hueMod>"] {
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(parse_positive_percentage(&doc.root), None, "{xml}");
        }
    }

    #[test]
    fn hue_modulation_multiplies_angles_and_wraps_complete_turns() {
        for (factor, expected) in [(0, [255, 0, 0]), (50_000, [0, 255, 0]), (100_000, [0, 0, 255]), (150_000, [255, 0, 0]), (200_000, [0, 255, 0]), (300_000, [255, 0, 0])] {
            assert_eq!(frame_hue([0, 0, 255], factor), expected, "{factor}");
        }
        assert_eq!(frame_hue([21, 96, 130], 0), [130, 21, 21]);
        assert_eq!(frame_hue([21, 96, 130], 200_000), [130, 89, 21]);
        assert_eq!(frame_hue([21, 96, 130], 300_000), [21, 28, 130]);
    }

    #[test]
    fn hue_modulation_preserves_identity_and_achromatic_colors() {
        for rgb in [[0; 3], [128; 3], [255; 3], [21, 96, 130], [210, 126, 74]] {
            assert_eq!(frame_hue(rgb, 100_000), rgb);
            if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
                for factor in [0, 50_000, 200_000, i32::MAX as u32] {
                    assert_eq!(frame_hue(rgb, factor), rgb);
                }
            }
        }
    }

    #[test]
    fn hue_modulation_resolves_srgb_and_theme_frames() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<c:chartSpace xmlns:c='http://schemas.openxmlformats.org/drawingml/2006/chart' xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main'><c:spPr><a:solidFill><a:{kind} val='{base}'><a:hueMod val='200000'/></a:{kind}></a:solidFill></c:spPr></c:chartSpace>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(read_frame_with(&doc.root, &scheme).unwrap().fill, FrameFill::Solid([130, 89, 21]));
        }
    }

    #[test]
    fn hue_modulation_trailing_alpha_preserves_straight_color() {
        for (alpha, expected) in [(0, FrameFill::Translucent([130, 89, 21], 0)), (50_000, FrameFill::Translucent([130, 89, 21], 50_000)), (100_000, FrameFill::Solid([130, 89, 21]))] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:hueMod val='200000'/><a:alpha val='{alpha}'/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).unwrap(), expected);
        }
    }

    #[test]
    fn hue_modulation_chart_and_plot_match_explicit_color_at_three_dpis() {
        for plot in [false, true] {
            for alpha in ["", "<a:alpha val='50000'/>"] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='0000FF'><a:hueMod val='50000'/>{alpha}</a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='00FF00'>{alpha}</a:srgbClr></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} alpha={alpha} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn hue_modulation_unverified_transform_sequences_stay_unsupported() {
        for transforms in ["<a:alpha val='50000'/><a:hueMod val='50000'/>", "<a:hueMod val='50000'/><a:hueMod val='50000'/>", "<a:hueMod val='50000'/><a:lumOff val='10000'/>", "<a:satMod val='50000'/><a:hueMod val='50000'/>", "<a:hueMod val='50000'/><a:satMod val='50000'/>", "<a:hueOff val='600000'/><a:hueMod val='50000'/>"] {
            let config = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'>{transforms}</a:srgbClr></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>")).as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err(), "{transforms}");
        }
    }

    #[test]
    fn alpha_mod_pair_retains_exact_products_and_rgb() {
        for (a,b,product,byte) in [(50000,33333,1666650000,42),(50001,33333,1666683333,43),(33333,50000,1666650000,42),(33333,50001,1666683333,43)] {let x=crate::xml::parse(format!("<alphaMod val='{a}'/>").as_bytes()).unwrap();let y=crate::xml::parse(format!("<alphaMod val='{b}'/>").as_bytes()).unwrap();assert_eq!(frame_alpha_mod_pair([61,101,71],&x.root,&y.root),Some(FrameFill::AlphaProduct([61,101,71],product)));assert_eq!(alpha_product_to_u8(product),byte);}
    }

    #[test]
    fn alpha_mod_pair_early_saturation_preserves_factor_order() {
        let x=crate::xml::parse(b"<alphaMod val='200000'/>").unwrap();let y=crate::xml::parse(b"<alphaMod val='50000'/>").unwrap();assert_eq!(frame_alpha_mod_pair([61,101,71],&x.root,&y.root),Some(FrameFill::Translucent([61,101,71],50000)));assert_eq!(frame_alpha_mod_pair([61,101,71],&y.root,&x.root),Some(FrameFill::Solid([61,101,71])));
    }

    #[test]
    fn alpha_mod_pair_zero_identity_and_full_opacity() {
        for (a,b,alpha) in [(0,2147483647,0),(2147483647,0,0),(100000,50000,50000),(50000,100000,50000),(100000,100000,100000),(25000,25000,6250)] {let x=crate::xml::parse(format!("<alphaMod val='{a}'/>").as_bytes()).unwrap();let y=crate::xml::parse(format!("<alphaMod val='{b}'/>").as_bytes()).unwrap();let expected=if alpha==100000{FrameFill::Solid([61,101,71])}else{FrameFill::Translucent([61,101,71],alpha)};assert_eq!(frame_alpha_mod_pair([61,101,71],&x.root,&y.root),Some(expected));}
    }

    #[test]
    fn alpha_mod_pair_maximum_factor_and_d_minus_one_remain_exact() {
        for (a,b,expected) in [(1,2147483647,FrameFill::AlphaProduct([61,101,71],2147483647)),(2147483647,1,FrameFill::Translucent([61,101,71],1)),(2147483647,2147483647,FrameFill::Solid([61,101,71])),(99999,100001,FrameFill::AlphaProduct([61,101,71],9999999999))] {let x=crate::xml::parse(format!("<alphaMod val='{a}'/>").as_bytes()).unwrap();let y=crate::xml::parse(format!("<alphaMod val='{b}'/>").as_bytes()).unwrap();assert_eq!(frame_alpha_mod_pair([61,101,71],&x.root,&y.root),Some(expected));}assert_eq!(alpha_product_to_u8(9999999999),255);
    }

    #[test]
    fn alpha_mod_pair_strict_operands_both_sides_even_under_zero() {
        let zero=crate::xml::parse(b"<alphaMod val='0'/>").unwrap();
        for raw in ["<alphaMod/>","<alphaMod val='-1'/>","<alphaMod val='2147483648'/>","<alphaMod val='50%'/>","<alphaMod val='+1'/>","<alphaMod val=' 1'/>","<alphaMod val='1.0'/>","<alphaMod val='1' extra='0'/>","<alphaMod val='1'><alphaMod val='0'/></alphaMod>","<alphaMod val='1'>x</alphaMod>","<alphaMod xmlns:x='urn:test' x:val='1'/>","<alphaOff val='0'/>"] {let x=crate::xml::parse(raw.as_bytes()).unwrap();assert_eq!(frame_alpha_mod_pair([61,101,71],&x.root,&zero.root),None);assert_eq!(frame_alpha_mod_pair([61,101,71],&zero.root,&x.root),None);}
        let mut x=crate::xml::parse(b"<alphaMod val='1'/>").unwrap();x.root.attrs.push(("val".to_owned(),"0".to_owned()));assert_eq!(frame_alpha_mod_pair([61,101,71],&zero.root,&x.root),None);let x=crate::xml::parse(b"<a:alphaMod xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='100000'>\n</a:alphaMod>").unwrap();assert_eq!(frame_alpha_mod_pair([61,101,71],&x.root,&zero.root),Some(FrameFill::Translucent([61,101,71],0)));
    }

    #[test]
    fn alpha_mod_pair_frame_plot_reference_at_three_dpis() {
        for (a,b) in [(200000_i32,50000),(50000,200000),(50001,33333),(99999,100001)] {for plot in [false,true] {let make=|t:&str|{let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='3D6547'>{t}</a:srgbClr></a:solidFill></c:spPr>");if plot{LONG_LEGEND.replace("<c:chart>","<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>",&format!("<c:plotArea>{fill}"))}else{LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))}};let x=crate::xml::parse(make(&format!("<a:alphaMod val='{a}'/><a:alphaMod val='{b}'/>")).as_bytes()).unwrap();let y=crate::xml::parse(make(&format!("<a:alpha val='{}'/><a:alphaMod val='{b}'/>",a.min(100000))).as_bytes()).unwrap();for value in [96,106,144] {let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();assert_eq!(render(&x.root),render(&y.root));}}}
    }

    #[test]
    fn alpha_mod_pair_other_bases_and_longer_chains_remain_gated() {
        for xml in ["<schemeClr val='accent1'><alphaMod val='50000'/><alphaMod val='50000'/></schemeClr>","<prstClr val='red'><alphaMod val='50000'/><alphaMod val='50000'/></prstClr>","<hslClr hue='0' sat='100000' lum='50000'><alphaMod val='50000'/><alphaMod val='50000'/></hslClr>","<scrgbClr r='0' g='0' b='0'><alphaMod val='50000'/><alphaMod val='50000'/></scrgbClr>","<sysClr val='window' lastClr='FFFFFF'><alphaMod val='50000'/><alphaMod val='50000'/></sysClr>","<srgbClr val='3D6547'><alpha val='50000'/><alphaMod val='50000'/><alphaMod val='50000'/></srgbClr>","<srgbClr val='3D6547'><alphaMod val='50000'/><alphaMod val='50000'/><alphaMod val='50000'/></srgbClr>"] {let x=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&x.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{xml}");}
    }

    #[test]
    fn alpha_mod_pair_does_not_enable_slide_host() {
        let fill="<c:spPr><a:solidFill><a:srgbClr val='3D6547'><a:alphaMod val='200000'/><a:alphaMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>";let a=crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();let b=crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();let render=|root:&Element,host|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,host,None,"",RenderDpi::DEFAULT).unwrap();assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn alpha_mod_off_is_noncommutative_and_keeps_rgb() {
        for (m,o,first,expected) in [(50000,25000,true,75000),(50000,25000,false,50000),(50000,-25000,true,25000),(50000,-25000,false,37500)] {
            let a=crate::xml::parse(format!("<alphaMod val='{m}'/>").as_bytes()).unwrap();let b=crate::xml::parse(format!("<alphaOff val='{o}'/>").as_bytes()).unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&b.root,first),Some(FrameFill::Translucent([61,101,71],expected)));
        }
    }

    #[test]
    fn alpha_mod_off_pins_each_stage_not_only_final_result() {
        for (m,o,first,expected) in [(200000,-50000,true,FrameFill::Translucent([61,101,71],50000)),(200000,-50000,false,FrameFill::Solid([61,101,71])),(0,25000,true,FrameFill::Translucent([61,101,71],25000)),(0,25000,false,FrameFill::Translucent([61,101,71],0))] {
            let a=crate::xml::parse(format!("<alphaMod val='{m}'/>").as_bytes()).unwrap();let b=crate::xml::parse(format!("<alphaOff val='{o}'/>").as_bytes()).unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&b.root,first),Some(expected));
        }
    }

    #[test]
    fn alpha_mod_off_preserves_exact_42_43_coverage_edge() {
        let a=crate::xml::parse(b"<alphaMod val='33333'/>").unwrap();
        for (off,product,byte) in [(-50000,1666650000,42),(-49999,1666683333,43)] {
            let b=crate::xml::parse(format!("<alphaOff val='{off}'/>").as_bytes()).unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&b.root,false),Some(FrameFill::AlphaProduct([61,101,71],product)));assert_eq!(alpha_product_to_u8(product),byte);
        }
    }

    #[test]
    fn alpha_mod_off_strict_operands_even_when_zero() {
        let valid_m=crate::xml::parse(b"<alphaMod val='0'/>").unwrap();let valid_o=crate::xml::parse(b"<alphaOff val='0'/>").unwrap();
        for raw in ["<alphaMod/>","<alphaMod val='-1'/>","<alphaMod val='2147483648'/>","<alphaMod val='50%'/>","<alphaMod val='+1'/>","<alphaMod val=' 1'/>","<alphaMod val='1.0'/>","<alphaMod val='1' extra='0'/>","<alphaMod val='1'><alphaOff val='0'/></alphaMod>","<alphaMod val='1'>x</alphaMod>","<alphaMod xmlns:x='urn:test' x:val='1'/>"] {let a=crate::xml::parse(raw.as_bytes()).unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&valid_o.root,true),None);}
        for raw in ["<alphaOff/>","<alphaOff val='-100001'/>","<alphaOff val='100001'/>","<alphaOff val='10%'/>","<alphaOff val='1' extra='0'/>","<alphaOff val='1'>x</alphaOff>","<alphaOff xmlns:x='urn:test' x:val='1'/>"] {let b=crate::xml::parse(raw.as_bytes()).unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&valid_m.root,&b.root,true),None);}
        let mut a=crate::xml::parse(b"<alphaMod val='1'/>").unwrap();a.root.attrs.push(("val".to_owned(),"0".to_owned()));assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&valid_o.root,true),None);
        let a=crate::xml::parse(b"<a:alphaMod xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='50000'/>").unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&valid_o.root,true),Some(FrameFill::Translucent([61,101,71],50000)));
    }

    #[test]
    fn alpha_mod_off_maximum_i32_factor_no_overflow_or_input_cap_guess() {
        let a=crate::xml::parse(b"<alphaMod val='2147483647'/>").unwrap();let b=crate::xml::parse(b"<alphaOff val='-99999'/>").unwrap();assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&b.root,false),Some(FrameFill::AlphaProduct([61,101,71],2147483647)));assert_eq!(frame_alpha_mod_off([61,101,71],&a.root,&b.root,true),Some(FrameFill::Translucent([61,101,71],1)));
    }

    #[test]
    fn alpha_mod_off_exact_reference_frame_plot_three_dpis() {
        for (transforms,reference) in [("<a:alphaMod val='50000'/><a:alphaOff val='25000'/>","<a:alpha val='75000'/>"),("<a:alphaOff val='25000'/><a:alphaMod val='50000'/>","<a:alpha val='50000'/>"),("<a:alphaOff val='-49999'/><a:alphaMod val='33333'/>","<a:alpha val='50001'/><a:alphaMod val='33333'/>")] {
            for plot in [false,true] {let make=|t:&str|{let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='3D6547'>{t}</a:srgbClr></a:solidFill></c:spPr>");if plot{LONG_LEGEND.replace("<c:chart>","<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>",&format!("<c:plotArea>{fill}"))}else{LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))}};let a=crate::xml::parse(make(transforms).as_bytes()).unwrap();let b=crate::xml::parse(make(reference).as_bytes()).unwrap();for value in [96,106,144] {let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();assert_eq!(render(&a.root),render(&b.root));}}
        }
    }

    #[test]
    fn alpha_mod_off_no_scheme_newbase_or_extra_chain_promotion() {
        for xml in ["<schemeClr val='accent1'><alphaMod val='50000'/><alphaOff val='25000'/></schemeClr>","<prstClr val='red'><alphaOff val='0'/><alphaMod val='50000'/></prstClr>","<hslClr hue='0' sat='100000' lum='50000'><alphaMod val='50000'/><alphaOff val='0'/></hslClr>","<scrgbClr r='0' g='0' b='0'><alphaMod val='50000'/><alphaOff val='0'/></scrgbClr>","<sysClr val='window' lastClr='FFFFFF'><alphaMod val='50000'/><alphaOff val='0'/></sysClr>","<srgbClr val='3D6547'><alpha val='50000'/><alphaMod val='50000'/><alphaOff val='25000'/></srgbClr>","<srgbClr val='3D6547'><alphaMod val='50000'/><alphaMod val='-1'/></srgbClr>","<srgbClr val='3D6547'><alphaOff val='-10000'/><alphaOff val='10000'/></srgbClr>"] {let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{xml}");}
    }

    #[test]
    fn alpha_mod_off_does_not_enable_slide_host() {
        let fill="<c:spPr><a:solidFill><a:srgbClr val='3D6547'><a:alphaMod val='50000'/><a:alphaOff val='25000'/></a:srgbClr></a:solidFill></c:spPr>";let a=crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();let b=crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();let render=|root:&Element,host|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,host,None,"",RenderDpi::DEFAULT).unwrap();assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn rgb_setter_alpha_all_channels_both_orders_independent_fields() {
        for (name,index) in [("red",0),("green",1),("blue",2)] {
            for (value,byte) in [(0,0),(25000,137),(50000,188),(100000,255)] {
                for opacity in [0,33333,50000,100000] {
                    let mut rgb=[61,101,71];rgb[index]=byte;let expected=if opacity==100000{FrameFill::Solid(rgb)}else{FrameFill::Translucent(rgb,opacity)};
                    for before in [false,true] {
                        let alpha=format!("<alpha val='{opacity}'/>");let component=format!("<{name} val='{value}'/>");let transforms=if before{format!("{alpha}{component}")}else{format!("{component}{alpha}")};let d=crate::xml::parse(format!("<srgbClr val='3D6547'>{transforms}</srgbClr>").as_bytes()).unwrap();assert_eq!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).unwrap(),expected);
                    }
                }
            }
        }
    }

    #[test]
    fn rgb_setter_alpha_opacity_is_not_setter_percentage_product() {
        let c=crate::xml::parse(b"<red val='50000'/>").unwrap();
        for (a,expected) in [(0,FrameFill::Translucent([188,101,71],0)),(50000,FrameFill::Translucent([188,101,71],50000)),(100000,FrameFill::Solid([188,101,71]))] {
            let alpha=crate::xml::parse(format!("<alpha val='{a}'/>").as_bytes()).unwrap();assert_eq!(frame_rgb_component_alpha([61,101,71],&c.root,&alpha.root),Some(expected));
        }
    }

    #[test]
    fn rgb_setter_alpha_strict_opacity_leaf_integer_and_namespace_contract() {
        let c=crate::xml::parse(b"<red val='50000'/>").unwrap();
        for xml in ["<alpha/>","<alpha val='-1'/>","<alpha val='100001'/>","<alpha val='50%'/>","<alpha val='+1'/>","<alpha val=' 1'/>","<alpha val='1.0'/>","<alpha val='2147483648'/>","<alpha val='1' extra='0'/>","<alpha val='1'><red val='0'/></alpha>","<alpha val='1'>opacity</alpha>","<alpha xmlns:x='urn:test' x:val='1'/>","<alphaMod val='50000'/>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(frame_rgb_component_alpha([61,101,71],&c.root,&d.root),None,"{xml}");
        }
        let mut a=crate::xml::parse(b"<alpha val='50000'/>").unwrap();a.root.attrs.push(("val".to_owned(),"100000".to_owned()));assert_eq!(frame_rgb_component_alpha([61,101,71],&c.root,&a.root),None);
        let a=crate::xml::parse(b"<a:alpha xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='50000'>
</a:alpha>").unwrap();assert_eq!(frame_rgb_component_alpha([61,101,71],&c.root,&a.root),Some(FrameFill::Translucent([188,101,71],50000)));
    }

    #[test]
    fn rgb_setter_alpha_validates_component_even_when_transparent() {
        let alpha=crate::xml::parse(b"<alpha val='0'/>").unwrap();
        for xml in ["<red val='200000'/>","<red val='-1'/>","<blue val='100001'/>","<red val='50000' extra='0'/>","<red val='50000'><alpha val='1'/></red>","<redMod val='50000'/>","<red xmlns:x='urn:test' x:val='50000'/>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(frame_rgb_component_alpha([61,101,71],&d.root,&alpha.root),None);
        }
    }

    #[test]
    fn rgb_setter_alpha_preserves_scheme_new_bases_and_other_mixed_gates() {
        for xml in ["<schemeClr val='accent1'><red val='50000'/><alpha val='50000'/></schemeClr>","<sysClr val='window' lastClr='FFFFFF'><alpha val='50000'/><red val='50000'/></sysClr>","<prstClr val='red'><alpha val='50000'/><red val='50000'/></prstClr>","<hslClr hue='0' sat='100000' lum='50000'><alpha val='50000'/><red val='50000'/></hslClr>","<scrgbClr r='0' g='0' b='0'><alpha val='50000'/><red val='50000'/></scrgbClr>","<srgbClr val='3D6547'><red val='50000'/><alphaMod val='50000'/></srgbClr>","<srgbClr val='3D6547'><alpha val='50000'/><red val='50000'/><alpha val='50000'/></srgbClr>","<srgbClr val='3D6547'><red val='50000'/><green val='10000'/><alpha val='50000'/></srgbClr>","<srgbClr val='3D6547'><red val='50000'/><alphaOff val='50000'/></srgbClr>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{xml}");
        }
    }

    #[test]
    fn rgb_setter_alpha_chart_plot_independent_rgb_reference_at_three_dpis() {
        for (name,rgb) in [("red","BC6547"),("green","3DBC47"),("blue","3D65BC")] {
            for plot in [false,true] {for before in [false,true] {for opacity in [33333,50000,100000] {
                let a=format!("<a:alpha val='{opacity}'/>");let c=format!("<a:{name} val='50000'/>");let transforms=if before{format!("{a}{c}")}else{format!("{c}{a}")};let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='3D6547'>{transforms}</a:srgbClr></a:solidFill></c:spPr>");let reference=format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'>{a}</a:srgbClr></a:solidFill></c:spPr>");let make=|fill:&str|if plot{LONG_LEGEND.replace("<c:chart>","<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>",&format!("<c:plotArea>{fill}"))}else{LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))};let x=crate::xml::parse(make(&fill).as_bytes()).unwrap();let y=crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96,106,144] {let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();assert_eq!(render(&x.root),render(&y.root));}
            }}}
        }
    }

    #[test]
    fn rgb_setter_alpha_full_opacity_matches_standalone_png() {
        for name in ["red","green","blue"] {for before in [false,true] {
            let c=format!("<a:{name} val='50000'/>");let transforms=if before{format!("<a:alpha val='100000'/>{c}")}else{format!("{c}<a:alpha val='100000'/>")};let make=|t:&str|LONG_LEGEND.replace("<c:chart>",&format!("<c:spPr><a:solidFill><a:srgbClr val='3D6547'>{t}</a:srgbClr></a:solidFill></c:spPr><c:chart>"));let a=crate::xml::parse(make(&c).as_bytes()).unwrap();let b=crate::xml::parse(make(&transforms).as_bytes()).unwrap();let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",RenderDpi::DEFAULT).unwrap();assert_eq!(render(&a.root),render(&b.root));
        }}
    }

    #[test]
    fn rgb_setter_alpha_does_not_enable_slide_frame_host() {
        let fill="<c:spPr><a:solidFill><a:srgbClr val='3D6547'><a:alpha val='50000'/><a:red val='50000'/></a:srgbClr></a:solidFill></c:spPr>";let a=crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();let b=crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();let render=|root:&Element,host|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,host,None,"",RenderDpi::DEFAULT).unwrap();assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn rgb_alpha_order_all_six_adjustments_both_isolated_orders() {
        for (name,rgb) in [("redMod",[92,128,128]),("greenMod",[128,92,128]),("blueMod",[128,128,92]),("redOff",[152,128,128]),("greenOff",[128,152,128]),("blueOff",[128,128,152])] {
            let value=if name.ends_with("Mod") {50000} else {10000};
            for opacity in [33333,50000,100000] {
                let expected=if opacity==100000 {FrameFill::Solid(rgb)} else {FrameFill::Translucent(rgb,opacity)};
                for before in [false,true] {
                    let a=format!("<alpha val='{opacity}'/>");let c=format!("<{name} val='{value}'/>");let transforms=if before {format!("{a}{c}")} else {format!("{c}{a}")};
                    let d=crate::xml::parse(format!("<srgbClr val='808080'>{transforms}</srgbClr>").as_bytes()).unwrap();
                    assert_eq!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).unwrap(),expected);
                }
            }
        }
    }

    #[test]
    fn rgb_alpha_order_sets_opacity_not_product_with_rgb_modulation() {
        let c=crate::xml::parse(b"<redMod val='50000'/>").unwrap();
        for (a,expected) in [(0,FrameFill::Translucent([92,128,128],0)),(50000,FrameFill::Translucent([92,128,128],50000)),(100000,FrameFill::Solid([92,128,128]))] {
            let alpha=crate::xml::parse(format!("<alpha val='{a}'/>").as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment_alpha([128;3],&c.root,&alpha.root),Some(expected));
        }
    }

    #[test]
    fn rgb_alpha_order_strict_alpha_leaf_integer_namespace_and_duplicate_contract() {
        let c=crate::xml::parse(b"<redMod val='50000'/>").unwrap();
        for xml in ["<alpha/>","<alpha val='-1'/>","<alpha val='100001'/>","<alpha val='50%'/>","<alpha val='+1'/>","<alpha val=' 1'/>","<alpha val='1.0'/>","<alpha val='2147483648'/>","<alpha val='1' extra='0'/>","<alpha val='1'><redMod val='0'/></alpha>","<alpha val='1'>opacity</alpha>","<alpha xmlns:x='urn:test' x:val='1'/>","<alphaMod val='50000'/>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment_alpha([128;3],&c.root,&d.root),None,"{xml}");
        }
        let mut a=crate::xml::parse(b"<alpha val='50000'/>").unwrap();a.root.attrs.push(("val".to_owned(),"100000".to_owned()));assert_eq!(frame_rgb_adjustment_alpha([128;3],&c.root,&a.root),None);
        let a=crate::xml::parse(b"<a:alpha xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='50000'>\n</a:alpha>").unwrap();assert_eq!(frame_rgb_adjustment_alpha([128;3],&c.root,&a.root),Some(FrameFill::Translucent([92,128,128],50000)));
    }

    #[test]
    fn rgb_alpha_order_validates_adjustment_even_when_fully_transparent() {
        let alpha=crate::xml::parse(b"<alpha val='0'/>").unwrap();
        for xml in ["<redMod val='200000'/>","<redMod val='-1'/>","<redOff val='100001'/>","<redMod val='50000' extra='0'/>","<redMod val='50000'><alpha val='1'/></redMod>","<red val='50000'/>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment_alpha([128;3],&d.root,&alpha.root),None);
        }
    }

    #[test]
    fn rgb_alpha_order_preserves_scheme_new_bases_and_all_other_mixed_gates() {
        for xml in ["<schemeClr val='accent1'><redMod val='50000'/><alpha val='50000'/></schemeClr>","<sysClr val='window' lastClr='FFFFFF'><alpha val='50000'/><redMod val='50000'/></sysClr>","<prstClr val='red'><alpha val='50000'/><redMod val='50000'/></prstClr>","<hslClr hue='0' sat='100000' lum='50000'><alpha val='50000'/><redMod val='50000'/></hslClr>","<scrgbClr r='0' g='0' b='0'><alpha val='50000'/><redMod val='50000'/></scrgbClr>","<srgbClr val='808080'><redMod val='50000'/><alphaMod val='50000'/></srgbClr>","<srgbClr val='808080'><alpha val='50000'/><redMod val='50000'/><alpha val='50000'/></srgbClr>","<srgbClr val='808080'><redMod val='50000'/><redOff val='10000'/><alpha val='50000'/></srgbClr>","<srgbClr val='808080'><red val='50000'/><alpha val='100001'/></srgbClr>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{xml}");
        }
    }

    #[test]
    fn rgb_alpha_order_frame_plot_and_rgb_reference_match_at_three_dpis() {
        for plot in [false,true] {
            for before in [false,true] {
                for opacity in [33333,50000,100000] {
                    let a=format!("<a:alpha val='{opacity}'/>");let c="<a:redMod val='50000'/>";let transforms=if before {format!("{a}{c}")} else {format!("{c}{a}")};
                    let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='808080'>{transforms}</a:srgbClr></a:solidFill></c:spPr>");let reference=format!("<c:spPr><a:solidFill><a:srgbClr val='5C8080'>{a}</a:srgbClr></a:solidFill></c:spPr>");
                    let make=|fill:&str|if plot {LONG_LEGEND.replace("<c:chart>","<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>",&format!("<c:plotArea>{fill}"))}else{LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))};let x=crate::xml::parse(make(&fill).as_bytes()).unwrap();let y=crate::xml::parse(make(&reference).as_bytes()).unwrap();
                    for value in [96,106,144] {let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();assert_eq!(render(&x.root),render(&y.root));}
                }
            }
        }
    }

    #[test]
    fn rgb_alpha_order_full_opacity_keeps_standalone_adjustment_pixels() {
        let a=crate::xml::parse(b"<srgbClr val='808080'><redOff val='10000'/></srgbClr>").unwrap();let b=crate::xml::parse(b"<srgbClr val='808080'><alpha val='100000'/><redOff val='10000'/></srgbClr>").unwrap();let scheme=theme_scheme(None,FALLBACK_ACCENTS);assert_eq!(resolve_frame_solid(&a.root,&scheme).unwrap(),resolve_frame_solid(&b.root,&scheme).unwrap());
    }

    #[test]
    fn rgb_alpha_order_does_not_enable_slide_frame_host() {
        let fill="<c:spPr><a:solidFill><a:srgbClr val='808080'><a:alpha val='50000'/><a:redMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>";let a=crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();let b=crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();let render=|root:&Element,host|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,host,None,"",RenderDpi::DEFAULT).unwrap();assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn rgb_adjustment_linear_arithmetic_and_untouched_channels() {
        for (channel,index) in [("red",0),("green",1),("blue",2)] {
            for (kind,value,expected) in [("Mod",0,0),("Mod",50000,92),("Mod",100000,128),("Off",-10000,96),("Off",10000,152),("Off",-100000,0),("Off",100000,255)] {
                let d=crate::xml::parse(format!("<{channel}{kind} val='{value}'/>").as_bytes()).unwrap();
                let mut input=[32,64,96];input[index]=128;let mut out=input;out[index]=expected;
                assert_eq!(frame_rgb_adjustment(input,&d.root),Some(out));
            }
        }
    }

    #[test]
    fn rgb_adjustment_identity_and_zero_hold_for_all_encoded_bytes() {
        for name in ["red","green","blue"] {
            for (kind,value,zero) in [("Mod",100000,false),("Off",0,false),("Mod",0,true)] {
                let d=crate::xml::parse(format!("<{name}{kind} val='{value}'/>").as_bytes()).unwrap();
                let index=if name=="red" {0} else if name=="green" {1} else {2};
                for byte in 0..=255 {
                    let input=[byte;3];let mut expected=input;if zero {expected[index]=0;}
                    assert_eq!(frame_rgb_adjustment(input,&d.root),Some(expected));
                }
            }
        }
    }

    #[test]
    fn rgb_adjustment_iec_not_encoded_example_or_guessed_gamma() {
        for (xml,input,expected) in [("<redMod val='50000'/>",[255,64,96],[188,64,96]),("<redMod val='50000'/>",[128,64,96],[92,64,96]),("<redOff val='1000'/>",[10,64,96],[30,64,96]),("<redOff val='1000'/>",[11,64,96],[31,64,96])] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment(input,&d.root),Some(expected));
        }
        // ECMA's FF*50%=80 encoded-byte example is NOT claimed to agree with Excel.
        let d=crate::xml::parse(b"<redMod val='50000'/>").unwrap();
        assert_ne!(frame_rgb_adjustment([255,0,0],&d.root),Some([128,0,0]));
    }

    #[test]
    fn rgb_adjustment_strict_integer_normal_subset_and_no_guessed_extension() {
        for name in ["redMod","greenMod","blueMod","redOff","greenOff","blueOff"] {
            for raw in ["100001","200000","-100001","2147483648","+1","50%","1.0"," 1","NaN",""] {
                let d=crate::xml::parse(format!("<{name} val='{raw}'/>").as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment([128;3],&d.root),None,"{name} {raw}");
            }
        }
        for name in ["redMod","greenMod","blueMod"] {
            let d=crate::xml::parse(format!("<{name} val='-1'/>").as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment([128;3],&d.root),None);
        }
    }

    #[test]
    fn rgb_adjustment_strict_leaf_semantic_attributes_and_namespaces() {
        for xml in ["<redMod/>","<redMod val='1' extra='0'/>","<redOff val='1'><alpha val='1'/></redOff>","<redOff val='1'>color</redOff>","<redMod xmlns:x='urn:test' x:val='1'/>","<red val='1'/>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert_eq!(frame_rgb_adjustment([128;3],&d.root),None);
        }
        let mut d=crate::xml::parse(b"<redMod val='1'/>").unwrap();d.root.attrs.push(("val".to_owned(),"2".to_owned()));assert_eq!(frame_rgb_adjustment([128;3],&d.root),None);
        let d=crate::xml::parse(b"<a:blueMod xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='50000'>\n</a:blueMod>").unwrap();assert_eq!(frame_rgb_adjustment([128;3],&d.root),Some([128,128,92]));
    }

    #[test]
    fn rgb_adjustment_frame_and_plot_rgb_equivalence_three_dpis() {
        for plot in [false,true] {
            for (name,rgb) in [("redMod","5C8080"),("greenMod","805C80"),("blueMod","80805C"),("redOff","988080"),("greenOff","809880"),("blueOff","808098")] {
                let value=if name.ends_with("Mod") {50000} else {10000};
                let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='808080'><a:{name} val='{value}'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference=format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make=|fill:&str|if plot {LONG_LEGEND.replace("<c:chart>","<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>",&format!("<c:plotArea>{fill}"))}else{LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))};
                let a=crate::xml::parse(make(&fill).as_bytes()).unwrap();let b=crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96,106,144] {let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();let render=|root:&Element|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();assert_eq!(render(&a.root),render(&b.root));}
            }
        }
    }

    #[test]
    fn rgb_adjustment_unverified_scheme_new_base_and_mixed_gates_remain_ni() {
        for xml in ["<schemeClr val='accent1'><redMod val='50000'/></schemeClr>","<schemeClr val='accent1'><blueOff val='10000'/></schemeClr>","<sysClr val='window' lastClr='FFFFFF'><redMod val='50000'/></sysClr>","<prstClr val='red'><redMod val='50000'/></prstClr>","<hslClr hue='0' sat='100000' lum='50000'><redMod val='50000'/></hslClr>","<scrgbClr r='0' g='0' b='0'><redMod val='50000'/></scrgbClr>","<srgbClr val='808080'><redMod val='50000'/><alpha val='100001'/></srgbClr>","<srgbClr val='808080'><redMod val='50000'/><redOff val='10000'/></srgbClr>","<srgbClr val='808080'><redOff val='10000'/><greenOff val='10000'/></srgbClr>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{xml}");
        }
    }

    #[test]
    fn rgb_adjustment_does_not_enable_slide_frame_rendering() {
        let fill="<c:spPr><a:solidFill><a:srgbClr val='808080'><a:redMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>";
        let a=crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();let b=crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();let render=|root:&Element,host|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,host,None,"",RenderDpi::DEFAULT).unwrap();assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn rgb_component_normal_intensity_replaces_only_requested_channel() {
        for (name, index) in [("red",0), ("green",1), ("blue",2)] {
            for (value, byte) in [(0,0), (313,10), (314,10), (25_000,137), (50_000,188), (100_000,255)] {
                let d = crate::xml::parse(format!("<{name} val='{value}'/>").as_bytes()).unwrap();
                for input in [[0x15,0x60,0x82], [0,255,0], [255,0,255], [31;3]] {
                    let mut expected = input;
                    expected[index] = byte;
                    assert_eq!(frame_rgb_component(input, &d.root), Some(expected));
                }
            }
        }
    }

    #[test]
    fn rgb_component_half_intensity_is_linear_not_encoded_percentage() {
        let d = crate::xml::parse(b"<red val='50000'/>").unwrap();
        assert_eq!(frame_rgb_component([21,96,130], &d.root), Some([188,96,130]));
        assert_ne!(frame_rgb_component([21,96,130], &d.root), Some([128,96,130]));
        assert_ne!(frame_rgb_component([21,96,130], &d.root), Some([186,96,130]));
    }

    #[test]
    fn rgb_component_no_guessed_clamp_or_broad_percentage_lexical_forms() {
        for name in ["red", "green", "blue"] {
            for raw in ["-1", "100001", "2147483648", "-2147483649", "+1", "50%", "1.0", " 1", "1 ", "", "NaN"] {
                let d = crate::xml::parse(format!("<{name} val='{raw}'/>").as_bytes()).unwrap();
                assert_eq!(frame_rgb_component([21,96,130], &d.root), None, "{name} {raw}");
            }
        }
    }

    #[test]
    fn rgb_component_strict_leaf_and_attribute_identity_preserve_namespaces() {
        for xml in ["<red/>", "<red val='1' extra='0'/>", "<red val='1'><alpha val='1'/></red>", "<red val='1'>color</red>", "<red xmlns:x='urn:test' x:val='1'/>", "<redOff val='1'/>", "<srgbClr val='123456'/>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(frame_rgb_component([21,96,130], &d.root), None, "{xml}");
        }
        let mut d = crate::xml::parse(b"<red val='0'/>").unwrap();
        d.root.attrs.push(("val".to_owned(), "50000".to_owned()));
        assert_eq!(frame_rgb_component([21,96,130], &d.root), None);
        let d = crate::xml::parse(b"<a:blue xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='50000'>\n\t</a:blue>").unwrap();
        assert_eq!(frame_rgb_component([21,96,130], &d.root), Some([21,96,188]));
    }

    #[test]
    fn rgb_component_srgb_and_scheme_resolve_same_verified_component_contract() {
        let scheme = theme_scheme(None, FALLBACK_ACCENTS);
        for (name,index) in [("red",0), ("green",1), ("blue",2)] {
            let d = crate::xml::parse(format!("<srgbClr val='156082'><{name} val='50000'/></srgbClr>").as_bytes()).unwrap();
            let mut expected = [21,96,130];expected[index]=188;
            assert_eq!(resolve_frame_solid(&d.root, &scheme).unwrap(), FrameFill::Solid(expected));
            let d = crate::xml::parse(format!("<schemeClr val='accent1'><{name} val='50000'/></schemeClr>").as_bytes()).unwrap();
            let mut expected = scheme.accents[0];expected[index]=188;
            assert_eq!(resolve_frame_solid(&d.root, &scheme).unwrap(), FrameFill::Solid(expected));
        }
    }

    #[test]
    fn rgb_component_frame_and_contrasting_plot_match_rgb_at_three_dpis() {
        for plot in [false,true] {
            for (name,rgb) in [("red","BC6082"), ("green","15BC82"), ("blue","1560BC")] {
                let fill=format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:{name} val='50000'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference=format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make=|fill:&str| if plot { LONG_LEGEND.replace("<c:chart>","<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>",&format!("<c:plotArea>{fill}")) } else {LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>"))};
                let a=crate::xml::parse(make(&fill).as_bytes()).unwrap();let b=crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96,106,144] {
                    let dpi=RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render=|root:&Element| render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();
                    assert_eq!(render(&a.root),render(&b.root),"{plot} {name} {value}");
                }
            }
        }
    }

    #[test]
    fn rgb_component_preserves_sysclr_other_base_and_mixed_transform_gates() {
        let scheme=theme_scheme(None,FALLBACK_ACCENTS);
        for xml in ["<sysClr val='window' lastClr='FFFFFF'/>", "<sysClr val='windowText' lastClr='FF0000'><red val='50000'/></sysClr>", "<hslClr hue='0' sat='100000' lum='50000'><red val='50000'/></hslClr>", "<scrgbClr r='0' g='0' b='0'><red val='50000'/></scrgbClr>", "<prstClr val='red'><blue val='50000'/></prstClr>", "<srgbClr val='156082'><red val='50000'/><alpha val='100001'/></srgbClr>", "<srgbClr val='156082'><alpha val='100001'/><red val='50000'/></srgbClr>", "<srgbClr val='156082'><red val='50000'/><green val='50000'/></srgbClr>", "<srgbClr val='156082'><redOff val='100001'/></srgbClr>", "<srgbClr val='156082'><blueMod val='100001'/></srgbClr>"] {
            let d=crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&d.root,&scheme).is_err(),"{xml}");
        }
    }

    #[test]
    fn rgb_component_does_not_enable_frame_in_slide_host() {
        let fill="<c:spPr><a:solidFill><a:srgbClr val='156082'><a:red val='50000'/></a:srgbClr></a:solidFill></c:spPr>";
        let a=crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();
        let b=crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let render=|root:&Element,host|render_chart_at(root,424,265,FALLBACK_ACCENTS,None,host,None,"",RenderDpi::DEFAULT).unwrap();
        assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));
        assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn scrgb_base_iec_transfer_matches_documented_gray_not_raw_or_power22() {
        for (v, b) in [(0,0), (1,0), (100,3), (313,10), (314,10), (400,13), (1000,25), (25_000,137), (50_000,188), (75_000,225), (100_000,255)] {
            assert_eq!(linear_srgb_channel(v), Some(b), "{v}");
        }
        assert_ne!(linear_srgb_channel(50_000), Some(128));
        assert_ne!(linear_srgb_channel(50_000), Some((0.5_f64.powf(1.0/2.2)*255.0).round() as u8));
    }

    #[test]
    fn scrgb_base_transfer_is_monotone_for_every_supported_integer_component() {
        let mut previous = 0;
        for value in 0..=100_000 {
            let current = linear_srgb_channel(value).unwrap();
            assert!(current >= previous, "{value}");
            previous = current;
        }
        assert_eq!(previous, 255);
        for value in [i32::MIN, -1, 100_001, i32::MAX] {
            assert_eq!(linear_srgb_channel(value), None);
        }
    }

    #[test]
    fn scrgb_base_normal_channels_and_theme_independence() {
        for (r,g,b,rgb) in [(100_000,0,0,[255,0,0]), (0,100_000,0,[0,255,0]), (0,0,100_000,[0,0,255]), (50_000,25_000,75_000,[188,137,225]), (50_000,50_000,50_000,[188;3])] {
            let d = crate::xml::parse(format!("<scrgbClr r='{r}' g='{g}' b='{b}'/>").as_bytes()).unwrap();
            assert_eq!(parse_frame_scrgb_color(&d.root), Some(rgb));
            for theme in [None, Some(OFFICE16_THEME.as_bytes())] {
                assert_eq!(resolve_frame_solid(&d.root, &theme_scheme(theme, FALLBACK_ACCENTS)).unwrap(), FrameFill::Solid(rgb));
            }
        }
    }

    #[test]
    fn scrgb_base_strict_leaf_attributes_and_no_guessed_range_or_percent_parser() {
        for xml in ["<scrgbClr/>", "<scrgbClr r='0' g='0'/>", "<scrgbClr r='0' g='0' b='0' val='1'/>", "<scrgbClr r='50%' g='50%' b='50%'/>", "<scrgbClr r='-1' g='0' b='0'/>", "<scrgbClr r='100001' g='0' b='0'/>", "<scrgbClr r='0' g='2147483648' b='0'/>", "<scrgbClr r='0' g='0' b='+1'/>", "<scrgbClr r='0' g='0' b=' 1'/>", "<scrgbClr r='0' g='0' b='1.0'/>", "<scrgbClr r='0' g='0' b='0'><alpha val='100000'/></scrgbClr>", "<scrgbClr r='0' g='0' b='0'>color</scrgbClr>", "<srgbClr r='0' g='0' b='0'/>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(parse_frame_scrgb_color(&d.root), None, "{xml}");
        }
        let d = crate::xml::parse(b"<a:scrgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' r='50000' g='50000' b='50000'>\n\t</a:scrgbClr>").unwrap();
        assert_eq!(parse_frame_scrgb_color(&d.root), Some([188;3]));
    }

    #[test]
    fn scrgb_base_duplicate_or_qualified_semantic_attributes_are_not_silently_picked() {
        let mut d = crate::xml::parse(b"<scrgbClr r='0' g='0' b='0'/>").unwrap();
        d.root.attrs.push(("r".to_owned(), "50000".to_owned()));
        assert_eq!(parse_frame_scrgb_color(&d.root), None);
        let d = crate::xml::parse(b"<scrgbClr xmlns:x='urn:test' x:r='0' g='0' b='0'/>").unwrap();
        assert_eq!(parse_frame_scrgb_color(&d.root), None);
    }

    #[test]
    fn scrgb_base_frame_and_plot_match_predeclared_rgb_at_three_dpis() {
        for plot in [false, true] {
            for (r,g,b,rgb) in [(50_000,50_000,50_000,"BCBCBC"), (50_000,25_000,75_000,"BC89E1"), (100,313,1000,"030A19")] {
                let fill = format!("<c:spPr><a:solidFill><a:scrgbClr r='{r}' g='{g}' b='{b}'/></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let d = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96,106,144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root,424,265,FALLBACK_ACCENTS,None,ChartHost::Excel,None,"",dpi).unwrap();
                    assert_eq!(render(&d.root), render(&reference.root), "plot={plot} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn scrgb_base_does_not_enable_the_slide_frame_host() {
        let fill = "<c:spPr><a:solidFill><a:scrgbClr r='50000' g='50000' b='50000'/></a:solidFill></c:spPr>";
        let a = crate::xml::parse(LONG_LEGEND.replace("<c:chart>",&format!("{fill}<c:chart>")).as_bytes()).unwrap();
        let b = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let render = |root: &Element, host| render_chart(root,424,265,FALLBACK_ACCENTS,None,host,None,"").unwrap();
        assert_eq!(render(&a.root,ChartHost::Slide),render(&b.root,ChartHost::Slide));
        assert_ne!(render(&a.root,ChartHost::Excel),render(&b.root,ChartHost::Excel));
    }

    #[test]
    fn scrgb_base_single_fill_other_models_and_mixed_gates_remain() {
        let d = crate::xml::parse(b"<chartSpace><spPr><solidFill><scrgbClr r='0' g='0' b='0'/><srgbClr val='000000'/></solidFill></spPr></chartSpace>").unwrap();
        assert!(read_frame_fill(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err());
        for xml in ["<sysClr val='window' lastClr='FFFFFF'/>", "<scrgbClr r='50000' g='50000' b='50000'><hue val='60000'/></scrgbClr>", "<prstClr val='red'><alpha val='50000'/></prstClr>", "<hslClr hue='0' sat='100000' lum='50000'><alpha val='50000'/></hslClr>", "<srgbClr val='156082'><hue val='60000'/><alpha val='50000'/></srgbClr>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&d.root,&theme_scheme(None,FALLBACK_ACCENTS)).is_err(),"{xml}");
        }
    }

    #[test]
    fn preset_base_contract_values_do_not_infer_css_aliases() {
        for (name, rgb) in [("aliceBlue", [240,248,255]), ("black", [0,0,0]), ("green", [0,128,0]), ("lime", [0,255,0]), ("fuchsia", [255,0,255]), ("grey", [128,128,128]), ("darkBlue", [0,0,139]), ("dkBlue", [0,0,139]), ("medPurple", [147,112,219]), ("mediumPurple", [147,112,219]), ("white", [255,255,255]), ("yellowGreen", [154,205,50])] {
            assert_eq!(drawingml_preset_rgb(name), Some(rgb), "{name}");
        }
        for name in ["", "AliceBlue", "aliceblue", "darkblue", "transparent", "rebeccaPurple", "red ", " red", "#FF0000"] {
            assert_eq!(drawingml_preset_rgb(name), None, "{name}");
        }
    }

    #[test]
    fn preset_base_normative_non_equivalent_names_remain_distinct() {
        assert_eq!(drawingml_preset_rgb("darkSeaGreen"), Some([143,188,143]));
        assert_eq!(drawingml_preset_rgb("dkSeaGreen"), Some([143,188,139]));
        assert_eq!(drawingml_preset_rgb("lightGoldenrodYellow"), Some([250,250,210]));
        assert_eq!(drawingml_preset_rgb("ltGoldenrodYellow"), Some([250,250,120]));
        // No CSS substitution, alias folding or historical Office2007 table shift.
        assert_eq!(drawingml_preset_rgb("turquoise"), Some([64,224,208]));
        assert_eq!(drawingml_preset_rgb("violet"), Some([238,130,238]));
    }

    #[test]
    fn preset_base_all_190_normative_names_parse_without_theme() {
        let names = ["aliceBlue", "antiqueWhite", "aqua", "aquamarine", "azure", "beige", "bisque", "black", "blanchedAlmond", "blue", "blueViolet", "brown", "burlyWood", "cadetBlue", "chartreuse", "chocolate", "coral", "cornflowerBlue", "cornsilk", "crimson", "cyan", "darkBlue", "darkCyan", "darkGoldenrod", "darkGray", "darkGreen", "darkGrey", "darkKhaki", "darkMagenta", "darkOliveGreen", "darkOrange", "darkOrchid", "darkRed", "darkSalmon", "darkSeaGreen", "darkSlateBlue", "darkSlateGray", "darkSlateGrey", "darkTurquoise", "darkViolet", "deepPink", "deepSkyBlue", "dimGray", "dimGrey", "dkBlue", "dkCyan", "dkGoldenrod", "dkGray", "dkGreen", "dkGrey", "dkKhaki", "dkMagenta", "dkOliveGreen", "dkOrange", "dkOrchid", "dkRed", "dkSalmon", "dkSeaGreen", "dkSlateBlue", "dkSlateGray", "dkSlateGrey", "dkTurquoise", "dkViolet", "dodgerBlue", "firebrick", "floralWhite", "forestGreen", "fuchsia", "gainsboro", "ghostWhite", "gold", "goldenrod", "gray", "green", "greenYellow", "grey", "honeydew", "hotPink", "indianRed", "indigo", "ivory", "khaki", "lavender", "lavenderBlush", "lawnGreen", "lemonChiffon", "lightBlue", "lightCoral", "lightCyan", "lightGoldenrodYellow", "lightGray", "lightGreen", "lightGrey", "lightPink", "lightSalmon", "lightSeaGreen", "lightSkyBlue", "lightSlateGray", "lightSlateGrey", "lightSteelBlue", "lightYellow", "lime", "limeGreen", "linen", "ltBlue", "ltCoral", "ltCyan", "ltGoldenrodYellow", "ltGray", "ltGreen", "ltGrey", "ltPink", "ltSalmon", "ltSeaGreen", "ltSkyBlue", "ltSlateGray", "ltSlateGrey", "ltSteelBlue", "ltYellow", "magenta", "maroon", "medAquamarine", "medBlue", "mediumAquamarine", "mediumBlue", "mediumOrchid", "mediumPurple", "mediumSeaGreen", "mediumSlateBlue", "mediumSpringGreen", "mediumTurquoise", "mediumVioletRed", "medOrchid", "medPurple", "medSeaGreen", "medSlateBlue", "medSpringGreen", "medTurquoise", "medVioletRed", "midnightBlue", "mintCream", "mistyRose", "moccasin", "navajoWhite", "navy", "oldLace", "olive", "oliveDrab", "orange", "orangeRed", "orchid", "paleGoldenrod", "paleGreen", "paleTurquoise", "paleVioletRed", "papayaWhip", "peachPuff", "peru", "pink", "plum", "powderBlue", "purple", "red", "rosyBrown", "royalBlue", "saddleBrown", "salmon", "sandyBrown", "seaGreen", "seaShell", "sienna", "silver", "skyBlue", "slateBlue", "slateGray", "slateGrey", "snow", "springGreen", "steelBlue", "tan", "teal", "thistle", "tomato", "turquoise", "violet", "wheat", "white", "whiteSmoke", "yellow", "yellowGreen"];
        assert_eq!(names.len(), 190);
        assert_eq!(names.iter().collect::<std::collections::HashSet<_>>().len(), 190);
        for name in names {
            let d = crate::xml::parse(format!("<prstClr val='{name}'/>").as_bytes()).unwrap();
            let rgb = drawingml_preset_rgb(name).unwrap();
            assert_eq!(parse_frame_preset_color(&d.root), Some(rgb));
            for theme in [None, Some(OFFICE16_THEME.as_bytes())] {
                assert_eq!(resolve_frame_solid(&d.root, &theme_scheme(theme, FALLBACK_ACCENTS)).unwrap(), FrameFill::Solid(rgb));
            }
        }
    }

    #[test]
    fn preset_base_requires_exact_val_leaf_and_preserves_transform_gate() {
        for xml in ["<prstClr/>", "<prstClr val='Red'/>", "<prstClr val='red' extra='1'/>", "<prstClr val='red'><alpha val='100000'/></prstClr>", "<prstClr val='red'><hue val='60000'/></prstClr>", "<prstClr val='red'>color</prstClr>", "<srgbClr val='red'/>", "<prstClr xmlns:x='urn:test' x:val='red'/>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(parse_frame_preset_color(&d.root), None, "{xml}");
        }
        let d = crate::xml::parse(b"<a:prstClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='red'>\n\t</a:prstClr>").unwrap();
        assert_eq!(parse_frame_preset_color(&d.root), Some([255,0,0]));
    }

    #[test]
    fn preset_base_duplicate_semantic_attributes_do_not_silently_pick_one() {
        let mut d = crate::xml::parse(b"<prstClr val='red'/>").unwrap();
        d.root.attrs.push(("val".to_owned(), "blue".to_owned()));
        assert_eq!(parse_frame_preset_color(&d.root), None);
    }

    #[test]
    fn preset_base_frame_and_plot_match_declared_normative_rgb_at_three_dpis() {
        for plot in [false, true] {
            for (name, rgb) in [("red", "FF0000"), ("dkSeaGreen", "8FBC8B"), ("ltGoldenrodYellow", "FAFA78")] {
                let fill = format!("<c:spPr><a:solidFill><a:prstClr val='{name}'/></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let d = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&d.root), render(&reference.root), "{name} plot={plot} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn preset_base_does_not_enable_frame_in_slide_host() {
        let fill = "<c:spPr><a:solidFill><a:prstClr val='red'/></a:solidFill></c:spPr>";
        let a = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")).as_bytes()).unwrap();
        let b = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let render = |root: &Element, host| render_chart(root, 424, 265, FALLBACK_ACCENTS, None, host, None, "").unwrap();
        assert_eq!(render(&a.root, ChartHost::Slide), render(&b.root, ChartHost::Slide));
        assert_ne!(render(&a.root, ChartHost::Excel), render(&b.root, ChartHost::Excel));
    }

    #[test]
    fn preset_base_duplicate_fill_other_models_and_mixed_gates_remain() {
        let d = crate::xml::parse(b"<chartSpace><spPr><solidFill><prstClr val='red'/><srgbClr val='FF0000'/></solidFill></spPr></chartSpace>").unwrap();
        assert!(read_frame_fill(&d.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err());
        for xml in ["<sysClr val='window' lastClr='FFFFFF'/>", "<scrgbClr r='100000' g='0' b='0'><alpha val='50000'/></scrgbClr>", "<srgbClr val='156082'><hue val='60000'/><alpha val='50000'/></srgbClr>", "<hslClr hue='0' sat='100000' lum='50000'><alpha val='50000'/></hslClr>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&d.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{xml}");
        }
    }

    #[test]
    fn hsl_base_normal_model_primary_secondary_and_nonsector_colors() {
        for (h, s, l, rgb) in [(0, 100_000, 50_000, [255, 0, 0]), (3_600_000, 100_000, 50_000, [255, 255, 0]), (7_200_000, 100_000, 50_000, [0, 255, 0]), (10_800_000, 100_000, 50_000, [0, 255, 255]), (14_400_000, 100_000, 50_000, [0, 0, 255]), (18_000_000, 100_000, 50_000, [255, 0, 255]), (0, 100_000, 25_000, [128, 0, 0]), (0, 100_000, 75_000, [255, 128, 128]), (2_700_000, 50_000, 50_000, [191, 159, 64])] {
            let xml = format!("<hslClr hue='{h}' sat='{s}' lum='{l}'/>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(parse_frame_hsl_color(&doc.root), Some(rgb));
        }
    }

    #[test]
    fn hsl_base_normal_model_gray_black_white_and_angle_boundaries() {
        for hue in [0, 1, 7_200_000, 21_599_999] {
            for (lum, gray) in [(0, 0), (25_000, 64), (50_000, 128), (75_000, 191), (100_000, 255)] {
                let doc = crate::xml::parse(format!("<hslClr hue='{hue}' sat='0' lum='{lum}'/>").as_bytes()).unwrap();
                assert_eq!(parse_frame_hsl_color(&doc.root), Some([gray; 3]));
            }
            for sat in [0, 25_000, 100_000] {
                for (lum, rgb) in [(0, [0; 3]), (100_000, [255; 3])] {
                    let doc = crate::xml::parse(format!("<hslClr hue='{hue}' sat='{sat}' lum='{lum}'/>").as_bytes()).unwrap();
                    assert_eq!(parse_frame_hsl_color(&doc.root), Some(rgb));
                }
            }
        }
    }

    #[test]
    fn hsl_base_requires_exact_three_named_attributes_and_no_transforms() {
        for xml in ["<hslClr/>", "<hslClr hue='0' sat='100000'/>", "<hslClr h='0' s='100000' l='50000'/>", "<hslClr hue='0' sat='100000' lum='50000' val='1'/>", "<hslClr hue='0' sat='100000' lum='50000'><alpha val='100000'/></hslClr>", "<hslClr hue='0' sat='100000' lum='50000'><hue val='60000'/></hslClr>", "<hslClr hue='0' sat='100000' lum='50000'>color</hslClr>", "<srgbClr hue='0' sat='100000' lum='50000'/>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(parse_frame_hsl_color(&d.root).is_none(), "{xml}");
        }
        let d = crate::xml::parse(b"<hslClr hue='0' sat='100000' lum='50000'> 
 </hslClr>").unwrap();
        assert_eq!(parse_frame_hsl_color(&d.root), Some([255, 0, 0]));
    }

    #[test]
    fn hsl_base_strict_integer_normal_ranges_do_not_guess_wrap_or_clamp() {
        for raw in ["", "+1", " 1", "1 ", "1.0", "50%", "1e2", "2147483648", "-2147483649", "--1"] {
            assert_eq!(parse_frame_integer(raw), None, "{raw}");
        }
        for (h, s, l) in [(-1, 50_000, 50_000), (21_600_000, 50_000, 50_000), (i32::MAX, 0, 0), (0, -1, 50_000), (0, 100_001, 50_000), (0, 50_000, -1), (0, 50_000, 100_001)] {
            let d = crate::xml::parse(format!("<hslClr hue='{h}' sat='{s}' lum='{l}'/>").as_bytes()).unwrap();
            assert_eq!(parse_frame_hsl_color(&d.root), None);
        }
        assert_eq!(parse_frame_integer("-2147483648"), Some(i32::MIN));
        assert_eq!(parse_frame_integer("2147483647"), Some(i32::MAX));
    }

    #[test]
    fn hsl_base_is_theme_independent_but_existing_srgb_theme_paths_remain() {
        let d = crate::xml::parse(b"<a:hslClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' hue='14400000' sat='100000' lum='50000'/>").unwrap();
        for theme in [None, Some(OFFICE16_THEME.as_bytes())] {
            assert_eq!(resolve_frame_solid(&d.root, &theme_scheme(theme, FALLBACK_ACCENTS)).unwrap(), FrameFill::Solid([0, 0, 255]));
        }
        let d = crate::xml::parse(b"<srgbClr val='156082'/>").unwrap();
        assert_eq!(resolve_frame_solid(&d.root, &theme_scheme(None, FALLBACK_ACCENTS)).unwrap(), FrameFill::Solid([21, 96, 130]));
    }

    #[test]
    fn hsl_base_frame_and_contrasting_plot_match_declared_rgb_at_three_dpis() {
        for plot in [false, true] {
            for (h, s, l, rgb) in [(3_600_000, 100_000, 25_000, "7F8000"), (14_400_000, 50_000, 75_000, "9F9FDF")] {
                let fill = format!("<c:spPr><a:solidFill><a:hslClr hue='{h}' sat='{s}' lum='{l}'/></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn hsl_base_does_not_enable_frame_rendering_in_the_slide_host() {
        let fill = "<c:spPr><a:solidFill><a:hslClr hue='0' sat='100000' lum='50000'/></a:solidFill></c:spPr>";
        let a = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")).as_bytes()).unwrap();
        let b = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let render = |root: &Element, host| render_chart(root, 424, 265, FALLBACK_ACCENTS, None, host, None, "").unwrap();
        assert_eq!(render(&a.root, ChartHost::Slide), render(&b.root, ChartHost::Slide));
        assert_ne!(render(&a.root, ChartHost::Excel), render(&b.root, ChartHost::Excel));
    }

    #[test]
    fn hsl_base_duplicate_fill_and_other_color_model_gates_are_preserved() {
        let d = crate::xml::parse(b"<chartSpace><spPr><solidFill><hslClr hue='0' sat='100000' lum='50000'/><srgbClr val='FF0000'/></solidFill></spPr></chartSpace>").unwrap();
        assert!(read_frame_fill(&d.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err());
        for xml in ["<sysClr val='window' lastClr='FFFFFF'/>", "<prstClr val='red'><alpha val='50000'/></prstClr>", "<scrgbClr r='100000' g='0' b='0'><alpha val='50000'/></scrgbClr>", "<srgbClr val='156082'><hue val='60000'/><alpha val='50000'/></srgbClr>"] {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&d.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{xml}");
        }
    }

    #[test]
    fn hue_absolute_replaces_h_with_six_primary_and_secondary_angles() {
        for (angle, expected) in [(0, [130, 21, 21]), (3_600_000, [130, 130, 21]), (7_200_000, [21, 130, 21]), (10_800_000, [21, 130, 130]), (14_400_000, [21, 21, 130]), (18_000_000, [130, 21, 130])] {
            assert_eq!(frame_absolute_hue([21, 96, 130], angle), Some(expected));
        }
        assert_eq!(frame_absolute_hue([210, 126, 74], 2_700_000), Some([210, 176, 74]));
    }

    #[test]
    fn hue_absolute_uses_fixed_angle_not_percentage_modulation_or_offset() {
        assert_eq!(frame_absolute_hue([21, 96, 130], 21_599_999), Some([130, 21, 21]));
        assert_ne!(frame_absolute_hue([21, 96, 130], 0), Some([21, 96, 130]));
        assert_ne!(frame_absolute_hue([21, 96, 130], 0), Some(frame_hue_offset([21, 96, 130], 0)));
        assert_ne!(frame_absolute_hue([21, 96, 130], 3_600_000), Some(frame_hue([21, 96, 130], 3_600_000)));
        assert_eq!(frame_absolute_hue([255, 0, 0], 60_000), Some([255, 4, 0]));
    }

    #[test]
    fn hue_absolute_achromatic_inputs_are_identity_at_every_declared_angle() {
        for gray in 0..=255 {
            for angle in [0, 1, 60_000, 7_200_000, 21_599_999] {
                assert_eq!(frame_absolute_hue([gray; 3], angle), Some([gray; 3]));
            }
        }
    }

    #[test]
    fn hue_absolute_strict_i32_leaf_and_positive_fixed_angle_domain() {
        for angle in [i32::MIN, -1, 21_600_000, 21_600_001, i32::MAX] {
            assert_eq!(frame_absolute_hue([21, 96, 130], angle), None);
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:hue val='{angle}'/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err());
        }
        for attrs in ["", "val='2147483648'", "val='-2147483649'", "val='+60000'", "val='60%'", "val='1.0'", "val=' 60000'", "val=''", "val='60000' other='1'"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:hue {attrs}/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{attrs}");
        }
        for angle in [0, 1, 21_599_999] {
            let doc = crate::xml::parse(format!("<hue val='{angle}'/>").as_bytes()).unwrap();
            assert_eq!(parse_signed_percentage(&doc.root), Some(angle));
            assert!(frame_absolute_hue([21, 96, 130], angle).is_some());
        }
    }

    #[test]
    fn hue_absolute_resolves_srgb_and_theme_with_the_same_angle_contract() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<a:{kind} xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='{base}'><a:hue val='14400000'/></a:{kind}>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &scheme).unwrap(), FrameFill::Solid([21, 21, 130]));
        }
    }

    #[test]
    fn hue_absolute_chart_and_contrasting_plot_match_declared_rgb() {
        for plot in [false, true] {
            for (angle, rgb) in [(3_600_000, "828215"), (14_400_000, "151582")] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:hue val='{angle}'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn hue_absolute_preserves_saturation_and_lightness_via_rgb_extrema() {
        for r in (0..=255).step_by(51) {
            for g in (0..=255).step_by(51) {
                for b in (0..=255).step_by(51) {
                    let base = [r, g, b];
                    for angle in [0, 2_700_000, 7_200_000, 14_400_000, 21_599_999] {
                        let out = frame_absolute_hue(base, angle).unwrap();
                        assert_eq!(out.iter().min(), base.iter().min());
                        assert_eq!(out.iter().max(), base.iter().max());
                    }
                }
            }
        }
    }

    #[test]
    fn hue_absolute_mixed_repeated_nested_and_existing_other_gates_remain_ni() {
        for transforms in ["<a:hue val='60000'/><a:alpha val='50000'/>", "<a:alpha val='50000'/><a:hue val='60000'/>", "<a:hue val='60000'/><a:hue val='7200000'/>", "<a:hueMod val='50000'/><a:hue val='60000'/>", "<a:hue val='60000'/><a:lum val='50000'/>", "<a:hue val='60000'><a:alpha val='50000'/></a:hue>", "<a:satOff val='50000'/>"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{transforms}");
        }
    }

    #[test]
    fn luminance_absolute_replaces_l_not_a_multiplier_or_offset() {
        for (value, expected) in [(0, [0, 0, 0]), (25_000, [18, 81, 110]), (50_000, [35, 162, 220]), (75_000, [145, 209, 237]), (100_000, [255, 255, 255])] {
            assert_eq!(frame_luminance([21, 96, 130], value), Some(expected));
        }
        assert_ne!(frame_luminance([21, 96, 130], 50_000), Some(frame_lum([21, 96, 130], Some(50_000), None)));
        assert_ne!(frame_luminance([21, 96, 130], 25_000), Some(frame_lum([21, 96, 130], None, Some(25_000))));
    }

    #[test]
    fn luminance_absolute_achromatic_inputs_keep_zero_saturation() {
        for gray in 0..=255 {
            for (value, expected) in [(0, 0), (25_000, 64), (50_000, 128), (75_000, 191), (100_000, 255)] {
                assert_eq!(frame_luminance([gray; 3], value), Some([expected; 3]));
            }
        }
    }

    #[test]
    fn luminance_absolute_endpoints_are_black_and_white_for_rgb_cube() {
        for r in (0..=255).step_by(51) {
            for g in (0..=255).step_by(51) {
                for b in (0..=255).step_by(51) {
                    assert_eq!(frame_luminance([r, g, b], 0), Some([0; 3]));
                    assert_eq!(frame_luminance([r, g, b], 100_000), Some([255; 3]));
                }
            }
        }
    }

    #[test]
    fn luminance_absolute_uses_strict_percentage_and_does_not_guess_clamp() {
        for value in [i32::MIN, -1, 100_001, i32::MAX] {
            let d = crate::xml::parse(format!("<lum val='{value}'/>").as_bytes()).unwrap();
            assert_eq!(parse_signed_percentage(&d.root), Some(value));
            assert_eq!(frame_luminance([21, 96, 130], value), None);
        }
        for attrs in ["", "val='2147483648'", "val='-2147483649'", "val='+25000'", "val='25%'", "val='0.25'", "val=' 25000'", "val=''", "val='25000' other='1'"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:lum {attrs}/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{attrs}");
        }
    }

    #[test]
    fn luminance_absolute_resolves_srgb_and_theme() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<a:{kind} xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='{base}'><a:lum val='50000'/></a:{kind}>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &scheme).unwrap(), FrameFill::Solid([35, 162, 220]));
        }
    }

    #[test]
    fn luminance_absolute_chart_and_contrasting_plot_match_explicit_rgb() {
        for plot in [false, true] {
            for (value, rgb) in [(25_000, "12516E"), (75_000, "91D1ED")] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lum val='{value}'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn luminance_absolute_agrees_with_existing_zero_mod_then_offset_contract() {
        for base in [[21, 96, 130], [210, 126, 74], [0; 3], [128; 3], [255; 3]] {
            for value in [0, 1, 25_000, 50_000, 75_000, 99_999, 100_000] {
                assert_eq!(frame_luminance(base, value), Some(frame_lum(base, Some(0), Some(value))));
            }
        }
    }

    #[test]
    fn luminance_absolute_mixed_repeated_nested_and_other_gates_stay_ni() {
        for transforms in ["<a:lum val='25000'/><a:alpha val='50000'/>", "<a:alpha val='50000'/><a:lum val='25000'/>", "<a:lum val='25000'/><a:lum val='75000'/>", "<a:lumMod val='50000'/><a:lum val='25000'/>", "<a:lum val='25000'/><a:satOff val='10000'/>", "<a:lum val='25000'><a:alpha val='50000'/></a:lum>", "<a:satOff val='50000'/>"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{transforms}");
        }
    }

    #[test]
    fn saturation_absolute_replaces_s_instead_of_multiplying_or_adding() {
        for (value, expected) in [(0, [76, 76, 76]), (25_000, [57, 83, 94]), (50_000, [38, 90, 113]), (75_000, [19, 97, 132]), (100_000, [0, 104, 151])] {
            assert_eq!(frame_saturation([21, 96, 130], value), Some(expected));
        }
        assert_ne!(frame_saturation([21, 96, 130], 50_000), Some(frame_sat([21, 96, 130], 50_000)));
        assert_ne!(frame_saturation([21, 96, 130], 25_000), frame_sat_offset([21, 96, 130], 25_000));
    }

    #[test]
    fn saturation_absolute_preserves_hue_lightness_for_chromatic_colors() {
        for base in [[21, 96, 130], [210, 126, 74], [255, 0, 0], [0, 0, 255]] {
            let (h, _, l) = rgb_hsl(base);
            for value in [25_000, 50_000, 75_000, 100_000] {
                let rgb = frame_saturation(base, value).unwrap();
                let (new_h, _, new_l) = rgb_hsl(rgb);
                assert!((h - new_h).abs() < 1.0 / 255.0);
                assert!((l - new_l).abs() <= 0.5 / 255.0);
            }
        }
        assert_eq!(frame_saturation([255, 0, 0], 100_000), Some([255, 0, 0]));
        assert_eq!(frame_saturation([255, 0, 0], 50_000), Some([191, 64, 64]));
    }

    #[test]
    fn saturation_absolute_zero_neutralizes_and_preserves_existing_grays() {
        for gray in 0..=255 { assert_eq!(frame_saturation([gray; 3], 0), Some([gray; 3])); }
        for base in [[21, 96, 130], [210, 126, 74], [255, 0, 0], [0, 0, 255]] {
            let rgb = frame_saturation(base, 0).unwrap();
            let mid = ((u16::from(*base.iter().min().unwrap()) + u16::from(*base.iter().max().unwrap()) + 1) / 2) as u8;
            assert_eq!(rgb, [mid; 3]);
        }
    }

    #[test]
    fn saturation_absolute_unverified_range_and_gray_hue_stay_ni() {
        for value in [i32::MIN, -100_000, -1, 100_001, i32::MAX] {
            assert_eq!(frame_saturation([21, 96, 130], value), None);
        }
        for gray in [0, 1, 128, 254, 255] {
            for value in [1, 25_000, 100_000] { assert_eq!(frame_saturation([gray; 3], value), None); }
        }
    }

    #[test]
    fn saturation_absolute_uses_strict_percentage_leaf_parser() {
        for value in [i32::MIN, -100_001, 0, 25_000, 100_001, i32::MAX] {
            let doc = crate::xml::parse(format!("<sat val='{value}'/>").as_bytes()).unwrap();
            assert_eq!(parse_signed_percentage(&doc.root), Some(value));
        }
        for attrs in ["", "val='2147483648'", "val='-2147483649'", "val='+25000'", "val='25%'", "val='0.25'", "val=' 25000'", "val=''", "val='25000' other='1'"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:sat {attrs}/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{attrs}");
        }
    }

    #[test]
    fn saturation_absolute_resolves_srgb_and_theme() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<a:{kind} xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='{base}'><a:sat val='50000'/></a:{kind}>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &scheme).unwrap(), FrameFill::Solid([38, 90, 113]));
        }
    }

    #[test]
    fn saturation_absolute_chart_and_contrasting_plot_match_rgb_references() {
        for plot in [false, true] {
            for (value, rgb) in [(25_000, "39535E"), (75_000, "136184")] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:sat val='{value}'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn saturation_absolute_mixed_repeated_nested_and_sat_offset_gates_stay_ni() {
        for transforms in ["<a:sat val='25000'/><a:alpha val='50000'/>", "<a:alpha val='50000'/><a:sat val='25000'/>", "<a:sat val='25000'/><a:sat val='75000'/>", "<a:satMod val='50000'/><a:sat val='25000'/>", "<a:sat val='25000'/><a:satOff val='10000'/>", "<a:sat val='25000'><a:alpha val='50000'/></a:sat>", "<a:satOff val='50000'/>"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{transforms}");
        }
    }

    #[test]
    fn saturation_offset_adds_percentage_points_not_a_scale() {
        for (base, offset, expected) in [
            ([21, 96, 130], -50_000, [59, 82, 92]),
            ([21, 96, 130], -25_000, [40, 89, 111]),
            ([21, 96, 130], 25_000, [2, 103, 149]),
            ([210, 126, 74], -25_000, [182, 133, 102]),
            ([210, 126, 74], 25_000, [238, 119, 46]),
            ([255, 0, 0], -25_000, [223, 32, 32]),
        ] {
            assert_eq!(frame_sat_offset(base, offset), Some(expected));
        }
        assert_ne!(frame_sat_offset([21, 96, 130], -50_000), Some(frame_sat([21, 96, 130], 50_000)));
    }

    #[test]
    fn saturation_offset_zero_is_exact_for_rgb_cube_and_grays() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(17) {
                    assert_eq!(frame_sat_offset([r, g, b], 0), Some([r, g, b]));
                }
            }
        }
        for gray in 0..=255 { assert_eq!(frame_sat_offset([gray; 3], 0), Some([gray; 3])); }
    }

    #[test]
    fn saturation_offset_keeps_hue_lightness_and_neutral_endpoint() {
        for offset in [-50_000, -25_000, 25_000] {
            let base = [21, 96, 130];
            let result = frame_sat_offset(base, offset).unwrap();
            assert_eq!(u16::from(*result.iter().min().unwrap()) + u16::from(*result.iter().max().unwrap()), 151);
            let (h, _, l) = rgb_hsl(base);
            let (new_h, _, new_l) = rgb_hsl(result);
            // Conversion to 8-bit RGB alone accounts for this bounded quantization.
            assert!((h - new_h).abs() < 1.0 / 255.0);
            assert_eq!(l, new_l);
        }
        assert_eq!(frame_sat_offset([255, 0, 0], -100_000), Some([128; 3]));
    }

    #[test]
    fn saturation_offset_unverified_extrapolation_and_gray_hue_stay_ni() {
        for offset in [i32::MIN, -100_001, -100_000, 50_000, 100_000, i32::MAX] {
            assert_eq!(frame_sat_offset([21, 96, 130], offset), None);
        }
        assert_eq!(frame_sat_offset([255, 0, 0], 1), None);
        for gray in [0, 1, 128, 254, 255] {
            for offset in [-1, 1, 25_000] { assert_eq!(frame_sat_offset([gray; 3], offset), None); }
        }
    }

    #[test]
    fn saturation_offset_uses_percentage_not_fixed_percentage_parser() {
        for value in [i32::MIN, -100_001, -25_000, 0, 100_001, i32::MAX] {
            let doc = crate::xml::parse(format!("<satOff val='{value}'/>").as_bytes()).unwrap();
            assert_eq!(parse_signed_percentage(&doc.root), Some(value));
        }
        for attrs in ["", "val='2147483648'", "val='-2147483649'", "val='+25000'", "val='25%'", "val='0.25'", "val=' 25000'", "val=''", "val='25000' other='1'"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'><a:satOff {attrs}/></a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{attrs}");
        }
    }

    #[test]
    fn saturation_offset_resolves_srgb_and_theme_standalone_only() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<a:{kind} xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='{base}'><a:satOff val='-25000'/></a:{kind}>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&doc.root, &scheme).unwrap(), FrameFill::Solid([40, 89, 111]));
        }
    }

    #[test]
    fn saturation_offset_chart_and_contrasting_plot_match_explicit_color() {
        for plot in [false, true] {
            for (offset, rgb) in [(-25_000, "28596F"), (25_000, "026795")] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:satOff val='{offset}'/></a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{rgb}'/></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot {
                    LONG_LEGEND.replace("<c:chart>", "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>").replace("<c:plotArea>", &format!("<c:plotArea>{fill}"))
                } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} offset={offset} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn saturation_offset_mixed_repeated_and_nested_transforms_stay_ni() {
        for transforms in ["<a:satOff val='-25000'/><a:alpha val='50000'/>", "<a:alpha val='50000'/><a:satOff val='-25000'/>", "<a:satOff val='-25000'/><a:satOff val='25000'/>", "<a:satMod val='50000'/><a:satOff val='25000'/>", "<a:satOff val='-25000'/><a:lumOff val='10000'/>", "<a:satOff val='-25000'><a:alpha val='50000'/></a:satOff>"] {
            let xml = format!("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='156082'>{transforms}</a:srgbClr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&doc.root, &theme_scheme(None, FALLBACK_ACCENTS)).is_err(), "{transforms}");
        }
    }

    #[test]
    fn saturation_modulation_scales_hsl_distance_without_early_clipping() {
        for (factor, expected) in [(-50_000, [103, 65, 48]), (0, [76, 76, 76]), (50_000, [48, 86, 103]), (100_000, [21, 96, 130]), (200_000, [0, 117, 185])] {
            assert_eq!(frame_sat([21, 96, 130], factor), expected);
        }
        assert_eq!(frame_sat([210, 126, 74], -50_000), [108, 150, 176]);
        assert_eq!(frame_sat([210, 126, 74], 200_000), [255, 110, 6]);
    }

    #[test]
    fn saturation_modulation_preserves_achromatic_and_identity_colors() {
        for rgb in [[0; 3], [128; 3], [255; 3], [21, 96, 130], [210, 126, 74]] {
            assert_eq!(frame_sat(rgb, 100_000), rgb);
            if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
                for factor in [i32::MIN, -50_000, 0, 200_000, i32::MAX] {
                    assert_eq!(frame_sat(rgb, factor), rgb);
                }
            }
        }
    }

    #[test]
    fn saturation_percentage_accepts_signed_integer_thousandths() {
        for value in [i32::MIN, -50_000, 0, 50_000, 200_000, i32::MAX] {
            let xml = crate::xml::parse(format!("<satMod val='{value}'/>").as_bytes()).unwrap();
            assert_eq!(parse_signed_percentage(&xml.root), Some(value));
        }
        for xml in ["<satMod/>", "<satMod val='2147483648'/>", "<satMod val='-2147483649'/>", "<satMod val='+50000'/>", "<satMod val='50%'/>", "<satMod val='0.5'/>", "<satMod val=' 50000'/>", "<satMod val=''/>", "<satMod val='50000' other='1'/>", "<satMod val='50000'><alpha val='50000'/></satMod>"] {
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(parse_signed_percentage(&doc.root), None, "{xml}");
        }
    }

    #[test]
    fn saturation_modulation_resolves_srgb_and_theme_frames() {
        let scheme = theme_scheme(Some(OFFICE16_THEME.as_bytes()), FALLBACK_ACCENTS);
        for (kind, base) in [("srgbClr", "156082"), ("schemeClr", "accent1")] {
            let xml = format!("<c:chartSpace xmlns:c='http://schemas.openxmlformats.org/drawingml/2006/chart' xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main'><c:spPr><a:solidFill><a:{kind} val='{base}'><a:satMod val='50000'/></a:{kind}></a:solidFill></c:spPr></c:chartSpace>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(read_frame_with(&doc.root, &scheme).unwrap().fill, FrameFill::Solid([48, 86, 103]));
        }
    }

    #[test]
    fn saturation_modulation_trailing_alpha_keeps_straight_color() {
        for (alpha, expected) in [(0, FrameFill::Translucent([48, 86, 103], 0)), (50_000, FrameFill::Translucent([48, 86, 103], 50_000)), (100_000, FrameFill::Solid([48, 86, 103]))] {
            let xml = format!("<c:spPr xmlns:c='http://schemas.openxmlformats.org/drawingml/2006/chart' xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main'><a:solidFill><a:srgbClr val='156082'><a:satMod val='50000'/><a:alpha val='{alpha}'/></a:srgbClr></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let color = doc.root.element_children().find(|c| c.local_name() == "solidFill").unwrap().element_children().find(|c| c.local_name() == "srgbClr").unwrap();
            assert_eq!(resolve_frame_solid(color, &theme_scheme(None, FALLBACK_ACCENTS)).unwrap(), expected);
        }
    }

    #[test]
    fn saturation_modulation_chart_and_plot_match_explicit_color_at_three_dpis() {
        for plot in [false, true] {
            for alpha in ["", "<a:alpha val='50000'/>"] {
                let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'><a:satMod val='50000'/>{alpha}</a:srgbClr></a:solidFill></c:spPr>");
                let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='305667'>{alpha}</a:srgbClr></a:solidFill></c:spPr>");
                let make = |fill: &str| if plot { LONG_LEGEND.replace("<c:plotArea>", &format!("<c:plotArea>{fill}")) } else { LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>")) };
                let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
                let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
                for value in [96, 106, 144] {
                    let dpi = RenderDpi::from_request(serde_json::json!({"dpi":value}).as_object().unwrap()).unwrap();
                    let render = |root: &Element| render_chart_at(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi).unwrap();
                    assert_eq!(render(&doc.root), render(&reference.root), "plot={plot} alpha={alpha} dpi={value}");
                }
            }
        }
    }

    #[test]
    fn saturation_modulation_unverified_sequences_stay_unsupported() {
        for transforms in ["<a:alpha val='50000'/><a:satMod val='50000'/>", "<a:satMod val='50000'/><a:satMod val='50000'/>", "<a:satMod val='50000'/><a:lumOff val='10000'/>", "<a:tint val='10000'/><a:satMod val='50000'/>", "<a:satMod val='50000'/><a:hueMod val='50000'/>"] {
            let config = format!("<c:spPr><a:solidFill><a:srgbClr val='156082'>{transforms}</a:srgbClr></a:solidFill></c:spPr>");
            let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>")).as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err(), "{transforms}");
        }
    }

    #[test]
    fn rectangular_chart_no_line_preserves_fill_and_alpha_to_the_edge() {
        for (fill, rgb, alpha) in [
            (
                "<a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill>",
                [234, 247, 247],
                255,
            ),
            ("<a:noFill/>", [0, 0, 0], 0),
        ] {
            let config = format!("<c:roundedCorners val='0'/><c:spPr>{fill}<a:ln w='12700' cap='flat' cmpd='sng' algn='ctr'><a:noFill/><a:prstDash val='solid'/><a:round/></a:ln><a:effectLst/><a:extLst><a:ext uri='{{91240B29-F687-4F45-9708-019B960494DF}}'><a14:hiddenLine><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a14:hiddenLine></a:ext></a:extLst></c:spPr>");
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let frame = read_frame(&doc.root).unwrap();
            assert!(!frame.border);
            for dpi_value in [96, 106, 144] {
                let dpi = RenderDpi::from_request(
                    serde_json::json!({"dpi":dpi_value}).as_object().unwrap(),
                )
                .unwrap();
                let png = render_chart_at(
                    &doc.root,
                    424,
                    265,
                    FALLBACK_ACCENTS,
                    None,
                    ChartHost::Excel,
                    None,
                    "",
                    dpi,
                )
                .unwrap();
                let mut decoder = png::Decoder::new(std::io::Cursor::new(png))
                    .read_info()
                    .unwrap();
                let mut buf = vec![0; decoder.output_buffer_size()];
                let info = decoder.next_frame(&mut buf).unwrap();
                let stride = if info.color_type == png::ColorType::Rgba {
                    4
                } else {
                    3
                };
                for (x, y) in [(0, 0), (100, 0), (423, 100), (100, 264)] {
                    let i = (y * 424 + x) * stride;
                    assert_eq!(&buf[i..i + 3], &rgb);
                    if stride == 4 {
                        assert_eq!(buf[i + 3], alpha);
                    }
                }
            }
        }
    }

    #[test]
    fn visible_frame_lines_effects_and_rounded_noline_export() {
        // Rounded no-line: fill is clipped with the locked corner geometry,
        // without the default gray border.
        let rounded = "<c:roundedCorners val='1'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill><a:ln><a:noFill/></a:ln></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{rounded}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert!(frame.rounded);
        assert!(!frame.border);
        assert!(frame.chart_line.is_none());
        let png = super::render_chart(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, stride, buf) = decode_rgba(&png);
        assert_eq!((w, h), (80, 40));
        let corner = (0usize * w + 0) * stride;
        assert!(
            stride == 4 && buf[corner + 3] < 32,
            "rounded no-line outside corner should be clipped, alpha={}",
            buf[corner + 3]
        );
        let inner = (0usize * w + w / 2) * stride;
        assert_eq!(&buf[inner..inner + 3], &[234, 247, 247]);
        if stride == 4 {
            assert_eq!(buf[inner + 3], 255);
        }

        // Visible red chart line, omitted width (close-enough 1pt default).
        let red_ln = "<c:roundedCorners val='0'/><c:spPr><a:ln><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{red_ln}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert!(!frame.border);
        assert_eq!(frame.chart_line.as_ref().map(|l| l.color), Some([255, 0, 0]));
        let png = super::render_chart(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, stride, buf) = decode_rgba(&png);
        assert_eq!((w, h), (80, 40));
        let mid_top = (0usize * w + (w / 2)) * stride;
        assert_eq!(buf[mid_top], 255);
        assert!(buf[mid_top + 1] < 40 && buf[mid_top + 2] < 40, "top edge should be red");

        // Bare `<a:ln w='25400'/>` (no fill) exports with close-enough black.
        let bare = "<c:spPr><a:ln w='25400'/></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{bare}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert!(read_frame(&doc.root).is_ok());
        assert!(super::render_chart(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .is_ok());

        // Plot-area line exports (chart default gray border stays).
        let plot_ln = "<c:roundedCorners val='0'/><c:chart><c:plotArea><c:spPr><a:ln w='12700'><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln></c:spPr>";
        // LONG_LEGEND already has <c:chart>; inject plot spPr more carefully below.
        let xml = LONG_LEGEND.replace(
            "<c:plotArea>",
            "<c:plotArea><c:spPr><a:ln w='12700'><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></a:ln></c:spPr>",
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert!(frame.border);
        assert_eq!(frame.plot_line.as_ref().map(|l| l.color), Some([255, 0, 0]));
        assert!(super::render_chart(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .is_ok());
        let _ = plot_ln;

        // Empty outerShdw still exports and does not grow the canvas.
        let empty_sh = "<c:roundedCorners val='0'/><c:spPr><a:effectLst><a:outerShdw/></a:effectLst></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{empty_sh}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert!(frame.chart_shadow.is_some());
        let png = super::render_chart(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, _, _) = decode_rgba(&png);
        assert_eq!((w, h), (80, 40));

        // Official-like 45° outerShdw grows the canvas and keeps the chart fill.
        let sh = "<c:roundedCorners val='0'/><c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill><a:effectLst><a:outerShdw blurRad='63500' dist='53881' dir='2700000' rotWithShape='0'><a:scrgbClr r='0' g='0' b='0'/></a:outerShdw></a:effectLst></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{sh}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        assert!(frame.chart_shadow.is_some());
        let png = super::render_chart(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, stride, buf) = decode_rgba(&png);
        assert!(w > 80 && h > 40, "outerShdw should expand the canvas, got {w}x{h}");
        // Blur padding sits on every side, so (0,0) is not the original
        // chart origin. The fill must still appear somewhere, and a
        // non-zero alpha dark pixel should appear in the expanded margin.
        let mut saw_fill = false;
        let mut saw_shadow = false;
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * stride;
                let rgb = &buf[i..i + 3];
                let a = if stride == 4 { buf[i + 3] } else { 255 };
                if rgb == [234, 247, 247] && a == 255 {
                    saw_fill = true;
                }
                let in_src = x < 80 && y < 40;
                if !in_src && a > 8 && rgb[0] < 40 && rgb[1] < 40 && rgb[2] < 40 {
                    saw_shadow = true;
                }
            }
        }
        assert!(saw_fill, "outerShdw composite should keep the chart fill");
        assert!(saw_shadow, "outerShdw should paint a dark margin");
    }

    fn decode_rgba(png: &[u8]) -> (usize, usize, usize, Vec<u8>) {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; decoder.output_buffer_size()];
        let info = decoder.next_frame(&mut buf).unwrap();
        let stride = if info.color_type == png::ColorType::Rgba {
            4
        } else {
            3
        };
        (info.width as usize, info.height as usize, stride, buf)
    }

    #[test]
    fn unsupported_frame_semantics_do_not_fall_back_to_white() {
        for config in [
            "<c:spPr><a:gradFill/></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'><a:alpha val='50000'/><a:alphaMod val='50000'/><a:hueMod val='50000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:sysClr val='windowText' lastClr='FF0000'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:sysClr val='window' lastClr='00FF00'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:schemeClr val='dk2'/></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:satMod val='50000'/><a:lumOff val='10000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:hueMod val='50000'/><a:lumOff val='10000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:lumOff val='40000'/><a:lumMod val='60000'/></a:srgbClr></a:solidFill></c:spPr>",
            "<c:spPr><a:solidFill><a:srgbClr val='156082'><a:tint val='60000'/><a:shade val='75000'/></a:srgbClr></a:solidFill></c:spPr>",
        ] {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{config}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(read_frame(&doc.root).is_err(), "unsupported frame must not silently lose semantics: {config}");
        }
    }

    fn sample_rgb(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * w + x) * 3) as usize;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }

    fn tiny_png(w: u32, h: u32, px: &[[u8; 3]]) -> Vec<u8> {
        assert_eq!(px.len(), (w * h) as usize);
        let mut rgb = Vec::with_capacity(px.len() * 3);
        for c in px {
            rgb.extend_from_slice(c);
        }
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(&rgb).unwrap();
        drop(writer);
        out
    }

    #[test]
    fn chart_area_lthorz_pattern_exports_horizontal_stripes() {
        let fill = "<c:roundedCorners val='0'/><c:spPr><a:pattFill prst='ltHorz'><a:fgClr><a:srgbClr val='0F6082'/></a:fgClr><a:bgClr><a:srgbClr val='EAF7F7'/></a:bgClr></a:pattFill></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        match frame.fill {
            FrameFill::Pattern(patt) => {
                assert_eq!(patt.fg, [15, 96, 130]);
                assert_eq!(patt.bg, [234, 247, 247]);
            }
            other => panic!("expected pattern, got {other:?}"),
        }
        let png = render_chart_at(
            &doc.root,
            80,
            60,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let (w, h, rgb) = decode_rgb(&png);
        assert_eq!((w, h), (80, 60));
        // At 96 dpi the 8x8 ltHorz cell has fg rows at y%8 == 0 or 4.
        let y_fg = sample_rgb(&rgb, w, 4, 8);
        let y_bg = sample_rgb(&rgb, w, 4, 10);
        assert_eq!(y_fg, [15, 96, 130]);
        assert_eq!(y_bg, [234, 247, 247]);
        assert_eq!(sample_rgb(&rgb, w, 20, 8), [15, 96, 130]);
    }

    #[test]
    fn chart_area_stretch_blip_exports_when_media_is_present() {
        let fill = "<c:roundedCorners val='0'/><c:spPr><a:blipFill dpi='0' rotWithShape='1'><a:blip r:embed='rId1'><a:alphaModFix/></a:blip><a:srcRect/><a:stretch><a:fillRect/></a:stretch></a:blipFill></c:spPr>";
        let xml = LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let frame = read_frame(&doc.root).unwrap();
        match frame.fill {
            FrameFill::Blip(blip) => assert_eq!(blip.embed, "rId1"),
            other => panic!("expected blip, got {other:?}"),
        }
        assert!(
            render_chart_at(
                &doc.root,
                80,
                40,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                RenderDpi::DEFAULT,
            )
            .is_err(),
            "missing media must not fall back to white"
        );
        let mut tile = Vec::new();
        for y in 0..16u32 {
            for x in 0..16u32 {
                tile.push(if x < 8 { [200, 0, 0] } else { [0, 0, 200] });
            }
            let _ = y;
        }
        let bytes = tiny_png(16, 16, &tile);
        let mut blips = std::collections::BTreeMap::new();
        blips.insert("rId1".to_string(), bytes);
        let png = render_chart_with_media(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            RenderDpi::DEFAULT,
            None,
            None,
            Some(&blips),
        )
        .unwrap();
        let (w, h, rgb) = decode_rgb(&png);
        assert_eq!((w, h), (80, 40));
        let left = sample_rgb(&rgb, w, 2, 2);
        let right = sample_rgb(&rgb, w, 77, 2);
        assert!(left[0] > right[0] && left[2] < right[2], "left={left:?} right={right:?}");
    }

    #[test]
    fn close_enough_frame_fills_export_instead_of_rejecting() {
        // Diagonal linear, extra pattern presets, path shade, alpha/missing/duplicate
        // stops, and a tiled blip must produce an image. This is a close-enough
        // export path, not a unique official-matrix lock.
        let diag = "<c:roundedCorners val='0'/><c:spPr><a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='EAF7F7'/></a:gs><a:gs pos='100000'><a:srgbClr val='0F6082'/></a:gs></a:gsLst><a:lin ang='2700000' scaled='1'/><a:tileRect/></a:gradFill></c:spPr>";
        let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{diag}<c:chart>")).as_bytes()).unwrap();
        assert!(matches!(read_frame(&doc.root).unwrap().fill, FrameFill::Linear(_)));
        assert!(render_chart_at(&doc.root, 80, 40, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", RenderDpi::DEFAULT).is_ok());

        let patt = "<c:roundedCorners val='0'/><c:spPr><a:pattFill prst='pct10'><a:fgClr><a:srgbClr val='0F6082'/></a:fgClr><a:bgClr><a:srgbClr val='EAF7F7'/></a:bgClr></a:pattFill></c:spPr>";
        let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{patt}<c:chart>")).as_bytes()).unwrap();
        assert!(matches!(read_frame(&doc.root).unwrap().fill, FrameFill::Pattern(_)));
        let png = render_chart_at(&doc.root, 80, 40, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", RenderDpi::DEFAULT).unwrap();
        let (w, _, rgb) = decode_rgb(&png);
        let mut fg = 0u32;
        let mut bg = 0u32;
        for y in 0..8u32 {
            for x in 0..8u32 {
                let c = sample_rgb(&rgb, w, x, y);
                if c == [15, 96, 130] { fg += 1; }
                if c == [234, 247, 247] { bg += 1; }
            }
        }
        assert!(fg > 0 && bg > fg, "pct10 should be a sparse fg dither, fg={fg} bg={bg}");

        let path = "<c:roundedCorners val='0'/><c:spPr><a:gradFill rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='FFFFFF'/></a:gs><a:gs pos='100000'><a:srgbClr val='000000'/></a:gs></a:gsLst><a:path path='circle'/></a:gradFill></c:spPr>";
        let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{path}<c:chart>")).as_bytes()).unwrap();
        match read_frame(&doc.root).unwrap().fill {
            FrameFill::Linear(grad) => assert_eq!(grad.path, Some(PathShade::Circle)),
            other => panic!("expected path gradient, got {other:?}"),
        }
        assert!(render_chart_at(&doc.root, 80, 40, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", RenderDpi::DEFAULT).is_ok());

        let mut tile = Vec::new();
        for y in 0..8u32 {
            for x in 0..8u32 {
                tile.push(if (x + y) % 2 == 0 { [200, 0, 0] } else { [0, 0, 200] });
            }
        }
        let bytes = tiny_png(8, 8, &tile);
        let mut blips = std::collections::BTreeMap::new();
        blips.insert("rId1".to_string(), bytes);
        let tiled = "<c:roundedCorners val='0'/><c:spPr><a:blipFill rotWithShape='1'><a:blip r:embed='rId1'/><a:tile sx='100000' sy='100000' tx='0' ty='0' flip='none' algn='tl'/></a:blipFill></c:spPr>";
        let doc = crate::xml::parse(LONG_LEGEND.replace("<c:chart>", &format!("{tiled}<c:chart>")).as_bytes()).unwrap();
        match read_frame(&doc.root).unwrap().fill {
            FrameFill::Blip(blip) => assert!(blip.tile.is_some()),
            other => panic!("expected tiled blip, got {other:?}"),
        }
        render_chart_with_media(
            &doc.root,
            80,
            40,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            RenderDpi::DEFAULT,
            None,
            None,
            Some(&blips),
        )
        .expect("tiled blip with media must export");
    }

    #[test]
    fn rounded_frame_has_straight_alpha_not_a_white_key() {
        let mut canvas = Canvas::new(80, 60);
        canvas.fill_rect(20, 20, 30, 30, [255; 3]);
        if canvas.chart_border {
            canvas.stroke_border(BORDER);
        }
        let alpha = canvas.alpha.as_ref().unwrap();
        assert_eq!(alpha[0], 0);
        assert_eq!(&canvas.px[..3], &[0; 3]);
        assert_eq!(alpha[79], 0);
        assert_eq!(alpha[59 * 80], 0);
        assert_eq!(alpha[60 * 80 - 1], 0);
        assert_eq!(alpha[25 * 80 + 25], 255, "internal white stays opaque");
        assert_eq!(
            &canvas.px[(25 * 80 + 25) * 3..(25 * 80 + 25) * 3 + 3],
            &[255; 3]
        );
        let mut partial = 0;
        for (i, a) in alpha.iter().enumerate() {
            if *a > 0 && *a < 255 {
                partial += 1;
                assert_eq!(
                    &canvas.px[i * 3..i * 3 + 3],
                    &BORDER,
                    "unassociated edge RGB prevents white halos"
                );
            }
        }
        assert!(partial > 0);
        let png = canvas.encode_png().unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    }

    #[test]
    fn rounded_white_composite_preserves_original_border_pixels() {
        for (w, h) in [(80, 60), (384, 240), (576, 336)] {
            let mut actual = Canvas::new(w, h);
            actual.stroke_border(BORDER);
            let mut old = Canvas::new(w, h);
            let (w, h) = (w as i32, h as i32);
            let r = CORNER_R.min(w / 2).min(h / 2);
            for (cx, cy, x0, x1, y0, y1) in [
                (r, r, 0, r, 0, r),
                (w - r, r, w - r, w - 1, 0, r),
                (r, h - r, 0, r, h - r, h - 1),
                (w - r, h - r, w - r, w - 1, h - r, h - 1),
            ] {
                let rad = f64::from(cx.min(cy).min(CORNER_R)) - 0.5;
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let dx = f64::from(x) + 0.5 - f64::from(cx);
                        let dy = f64::from(y) + 0.5 - f64::from(cy);
                        let cov = (1.0 - ((dx * dx + dy * dy).sqrt() - rad).abs()).clamp(0.0, 1.0);
                        old.blend(x, y, BORDER, (cov * 255.0).round() as u8);
                    }
                }
            }
            old.hline_i(r, w - 1 - r, 0, BORDER);
            old.hline_i(r, w - 1 - r, h - 1, BORDER);
            old.vline_i(0, r, h - 1 - r, BORDER);
            old.vline_i(w - 1, r, h - 1 - r, BORDER);
            for (i, a) in actual.alpha.as_ref().unwrap().iter().enumerate() {
                for k in 0..3 {
                    let composited = ((u16::from(actual.px[i * 3 + k]) * u16::from(*a)
                        + 255 * (255 - u16::from(*a))
                        + 127)
                        / 255) as u8;
                    assert_eq!(
                        composited,
                        old.px[i * 3 + k],
                        "white composite changed at {i}/{k}"
                    );
                }
            }
        }
    }

    #[test]
    fn unmasked_canvas_keeps_rgb_bytes_and_slide_export() {
        let canvas = Canvas::new(30, 20);
        assert_eq!(
            canvas.encode_png().unwrap(),
            encode_png(30, 20, &canvas.px, png::ColorType::Rgb, RenderDpi::DEFAULT).unwrap()
        );
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        for (host, expected) in [
            (ChartHost::Slide, png::ColorType::Rgb),
            (ChartHost::Excel, png::ColorType::Rgba),
        ] {
            let data =
                super::render_chart(&doc.root, 576, 336, FALLBACK_ACCENTS, None, host, None, "")
                    .unwrap();
            let reader = png::Decoder::new(std::io::Cursor::new(data))
                .read_info()
                .unwrap();
            assert_eq!(reader.info().color_type, expected);
        }
    }

    const LONG_LEGEND: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>2025 年</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>850</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:ser>
          <c:order val="1"/>
          <c:tx><c:v>2026 年</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>1160</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#;

    #[cfg(windows)]
    fn grid_right(xml: &str) -> i32 {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let bytes = &buf[..info.buffer_size()];
        let y = INSET_TOP;
        let mut right = None;
        for x in (0..w).rev() {
            let i = (y as usize * w as usize + x as usize) * 3;
            if bytes[i] == 0 && bytes[i + 1] == 0 && bytes[i + 2] == 0 {
                right = Some(x);
                break;
            }
        }
        right.expect("顶上网格不存在")
    }

    #[cfg(windows)]
    fn column_chart(name: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    #[cfg(windows)]
    #[test]
    fn legend_filled_label_sits_on_swatch_bottom() {
        // t201u: typographic baseline sits on the last swatch pixel. This
        // raises 106 dpi year ink by 2 px vs the old y-2 fudge, matching
        // official Chart.Export 2025 bottoms. The same ascent rule applies
        // at 96 dpi (y-4 vs the old y-2). Not a typography rewrite.
        let face96 = super::super::chart_gdi::Face::open(
            RenderDpi::DEFAULT.pt_to_px_font(LABEL_PT) as f32,
            false,
        )
        .expect("face 96");
        let sw96 = RenderDpi::DEFAULT.scale_px(LEGEND_SWATCH);
        let top96 = legend_filled_label_top(100, sw96, Some(&face96), None, 0.0);
        assert_eq!(top96, f64::from(100 + sw96 - 1) - face96.line_height());
        assert_eq!(face96.line_height(), 11.0);
        assert_eq!(top96, 96.0); // 100+8-1-11

        let dpi106 =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        let face106 = super::super::chart_gdi::Face::open(
            dpi106.pt_to_px_font(LABEL_PT) as f32,
            false,
        )
        .expect("face 106");
        let sw106 = dpi106.scale_px(LEGEND_SWATCH);
        let top106 = legend_filled_label_top(100, sw106, Some(&face106), None, 0.0);
        assert_eq!(top106, f64::from(100 + sw106 - 1) - face106.line_height());
        assert_eq!(sw106, 9);
        assert_eq!(face106.line_height(), 12.0);
        assert_eq!(top106, 96.0); // 100+9-1-12
        // 2 px higher typographic top than the previous y - scale_px(2) fudge.
        assert_eq!(top106, 100.0 - 4.0);
        assert!(top106 < f64::from(100 - dpi106.scale_px(2)));
    }

    #[test]
    fn legend_filled_key_gap_keeps_96_dpi_and_tightens_at_106() {
        // t200u approximation: 96 dpi oracles stay on LEGEND_GAP=6; 106 dpi
        // official-export years move closer with a 2px device gap.
        assert_eq!(legend_filled_key_gap(RenderDpi::DEFAULT), LEGEND_GAP);
        assert_eq!(
            legend_key_gap(PlotKind::ClusteredColumn, RenderDpi::DEFAULT),
            LEGEND_GAP
        );
        let dpi106 =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        assert_eq!(legend_filled_key_gap(dpi106), LEGEND_GAP_SCALED_DPI);
        assert_eq!(
            legend_key_gap(PlotKind::ClusteredColumn, dpi106),
            LEGEND_GAP_SCALED_DPI
        );
        // Line keys stay on the existing 1.5pt rule at both DPIs.
        assert_eq!(
            legend_key_gap(PlotKind::StandardLine, RenderDpi::DEFAULT),
            RenderDpi::DEFAULT.pt_to_px_font(1.5).round() as i32
        );
        assert_eq!(
            legend_key_gap(PlotKind::StandardLine, dpi106),
            dpi106.pt_to_px_font(1.5).round() as i32
        );
        // Plot reservation stays on the scaled 96-dpi gap so the swatch does
        // not shift (identity at 96; 7 px at 106).
        assert_eq!(
            legend_filled_key_reserve_gap(RenderDpi::DEFAULT),
            LEGEND_GAP
        );
        assert_eq!(legend_filled_key_reserve_gap(dpi106), dpi106.scale_px(LEGEND_GAP));
        assert_eq!(dpi106.scale_px(LEGEND_GAP), 7);
        assert_ne!(legend_filled_key_gap(dpi106), legend_filled_key_reserve_gap(dpi106));
    }

    #[cfg(windows)]
    #[test]
    fn long_legend_uses_layout_width_and_short_names_keep_the_inset() {
        assert!(
            super::super::chart_gdi::Face::open((10.0_f64 * 96.0 / 72.0) as f32, false).is_some(),
            "本机应有图表字体"
        );
        assert_eq!(grid_right(&column_chart("S1")), 576 - INSET_RIGHT);
        let px = (10.0_f64 * 96.0 / 72.0) as f32;
        let face = super::super::chart_gdi::Face::open(px, false).expect("本机应有图表字体");
        let widest = face.layout_width("South") as i32;
        let need = LEGEND_PLOT_GAP + LEGEND_SWATCH + LEGEND_GAP + widest + LEGEND_PAD;
        let edge = 576 - need.max(INSET_RIGHT);
        assert!(
            edge < 576 - INSET_RIGHT,
            "长名字应把绘图区向左推 edge={edge}"
        );
        assert_eq!(grid_right(&column_chart("South")), edge);
    }

    #[test]
    fn category_ticks_paint_over_bars() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>S1</c:v></c:tx>
          <c:cat><c:strLit><c:pt idx="0"><c:v>A</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:pt idx="0"><c:v>4</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#;
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let bytes = &buf[..info.buffer_size()];
        let at = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        let axis_y = 336 - INSET_BOTTOM;
        let tick_y = axis_y - 1;
        let bar = |x: i32, y: i32| {
            let (r, g, b) = at(x, y);
            let hi = r.max(g).max(b);
            let lo = r.min(g).min(b);
            hi > 80 && hi - lo > 40
        };
        let ink = |x: i32, y: i32| at(x, y) == (0, 0, 0);
        let kept = (0..w).any(|x| ink(x, tick_y) && bar(x, tick_y - 12));
        assert!(kept, "柱底把类目刻度盖住了");
    }

    #[test]
    fn long_legend_label_stays_off_the_right_edge() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let h = info.height as i32;
        let bytes = &buf[..info.buffer_size()];
        let px = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        for x in (w - LEGEND_PAD)..(w - 1) {
            for y in CORNER_R..(h - CORNER_R) {
                let (r, g, b) = px(x, y);
                assert!(
                    r > 250 && g > 250 && b > 250,
                    "右缘被写上了像素 x={x} y={y} {r},{g},{b}"
                );
            }
        }
        let mut dark = false;
        for y in 0..h {
            for x in (w / 2)..(w - LEGEND_PAD) {
                let (r, g, b) = px(x, y);
                if r < 40 && g < 40 && b < 40 {
                    dark = true;
                }
            }
        }
        assert!(dark, "图例文字没有画到右侧");
    }

    #[test]
    fn frame_is_rounded_and_category_ticks_drop() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let png = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let w = info.width as i32;
        let bytes = &buf[..info.buffer_size()];
        let px = |x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        let (r, g, b) = px(0, 0);
        assert!(r > 250 && g > 250 && b > 250, "圆角外被涂了 {r},{g},{b}");
        assert_eq!(px(0, 20), (0x89, 0x89, 0x89));
        assert_eq!(px(20, 0), (0x89, 0x89, 0x89));
        let mut canvas = Canvas::new(200, 40);
        draw_cat_ticks(&mut canvas, 2, 10.0, 190.0, 8);
        draw_value_minor_ticks(&mut canvas, 20, 0, 25, 1, VAL_MINOR_PARTS);
        let ink = |x: i32, y: i32| {
            let i = (y as usize * canvas.w as usize + x as usize) * 3;
            canvas.px[i] < 40
        };
        for x in [10, 100, 190] {
            assert!(ink(x, 5), "主刻度没有向上 x={x}");
            assert!(ink(x, 12), "主刻度没画到 x={x}");
            assert!(!ink(x, 4), "主刻度向上超过 3 像素 x={x}");
        }
        for x in [55, 145] {
            assert!(ink(x, 6), "次刻度没有向上 x={x}");
            assert!(!ink(x, 5), "次刻度向上超过 2 像素 x={x}");
            assert!(ink(x, 11), "次刻度没画到 x={x}");
            assert!(!ink(x, 12), "次刻度画到了第 4 像素 x={x}");
        }
        for y in [5, 10, 15, 20] {
            assert!(ink(18, y), "数值轴次刻度太短 y={y}");
            assert!(ink(22, y), "数值轴次刻度没有向右 y={y}");
            assert!(!ink(17, y), "数值轴次刻度向左超过 2 像素 y={y}");
        }
    }

    const TITLED: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:title><c:tx><c:v>标题</c:v></c:tx></c:title>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:order val="0"/>
          <c:tx><c:v>2025 年</c:v></c:tx>
          <c:val><c:numLit><c:pt idx="0"><c:v>850</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    #[test]
    fn slide_host_drops_the_plot_and_skips_the_frame() {
        let doc = crate::xml::parse(TITLED.as_bytes()).unwrap();
        let excel = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let slide = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Slide,
            None,
            "",
        )
        .unwrap();
        let decode = |png: Vec<u8>| {
            let decoder = png::Decoder::new(std::io::Cursor::new(png));
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            buf.truncate(info.buffer_size());
            (info.width as i32, info.height as i32, buf)
        };
        let (w, h, excel_px) = decode(excel);
        let (_, _, slide_px) = decode(slide);
        let at = |bytes: &[u8], x: i32, y: i32| {
            let i = (y as usize * w as usize + x as usize) * 3;
            (bytes[i], bytes[i + 1], bytes[i + 2])
        };
        assert_eq!(at(&excel_px, 0, 20), (0x89, 0x89, 0x89));
        assert_eq!(at(&slide_px, 0, 20), (255, 255, 255));
        let first_grid = |bytes: &[u8]| {
            (0..h)
                .find(|y| at(bytes, 80, *y) == (0, 0, 0))
                .expect("没有网格")
        };
        assert_eq!(
            first_grid(&slide_px),
            first_grid(&excel_px) + TITLE_DROP as i32
        );
    }

    #[test]
    fn slide_legend_keeps_the_category_span_from_growing() {
        let doc = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
        let render = |host| {
            let png = render_rgb_chart(&doc.root, 576, 336, FALLBACK_ACCENTS, None, host, None, "")
                .unwrap();
            let decoder = png::Decoder::new(std::io::Cursor::new(png));
            let mut reader = decoder.read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            buf.truncate(info.buffer_size());
            (info.width as i32, buf)
        };
        let (w, excel) = render(ChartHost::Excel);
        let (_, slide) = render(ChartHost::Slide);
        let last_dark = |bytes: &[u8], y: i32| {
            (0..w).rev().find(|x| {
                let i = (y as usize * w as usize + *x as usize) * 3;
                bytes[i] < 40 && bytes[i + 1] < 40 && bytes[i + 2] < 40
            })
        };
        let y = 336 - INSET_BOTTOM;
        let excel_r = last_dark(&excel, y).expect("excel 没有横轴");
        let slide_r = last_dark(&slide, y).expect("slide 没有横轴");
        assert_eq!(excel_r, slide_r, "Excel 和幻灯片应按同一段排版宽度留白");
        let face = super::super::chart_gdi::Face::open((10.0_f64 * 96.0 / 72.0) as f32, false);
        if let Some(face) = face {
            let adv = face.width("2025 年").max(face.width("2026 年")).ceil() as i32;
            let advance_edge = 576
                - (LEGEND_PLOT_GAP + LEGEND_SWATCH + LEGEND_GAP + adv + LEGEND_PAD)
                    .max(INSET_RIGHT)
                + 1;
            assert!(
                excel_r + 4 <= advance_edge,
                "排版宽度应把绘图区向左推 excel={excel_r} advance={advance_edge}"
            );
        }
    }

    fn one_series(max: f64) -> Series {
        Series {
            name: "S".to_string(),
            values: vec![Some(max)],
            color: None,
            slot: 0,
            ..Series::default()
        }
    }

    #[test]
    fn axis_headroom_matches_excel_samples() {
        // 95 -> 0..100/20 follows the t194 general auto-scale rule (same
        // padded range as official t191 auto-max90, which is 0..100/20, not
        // a 10-interval 0..100/10). 95 itself was not in that gold matrix.
        // 4 and 3.9 -> 0..5/1 follow the t196u looser 4..=8 interval preference
        // (official t191 max4 was the denser 0..4.5/0.5 9-interval leftover).
        let cases = [
            (4.0, 5.0, 1.0),
            (3.9, 5.0, 1.0),
            (95.0, 100.0, 20.0),
            (96.0, 120.0, 20.0),
            (1100.0, 1200.0, 200.0),
            (1150.0, 1400.0, 200.0),
            (1160.0, 1400.0, 200.0),
        ];
        for (max, axis, step) in cases {
            let (lo, hi, got) = axis_range(&[one_series(max)]);
            assert!(
                (lo - 0.0).abs() < 1e-6 && (hi - axis).abs() < 1e-6 && (got - step).abs() < 1e-6,
                "max={max} got {lo}..{hi} step {got}"
            );
        }
    }

    #[test]
    fn axis_follows_tick_width() {
        assert_eq!(axis_x_of(7, RenderDpi::DEFAULT), 27);
        assert_eq!(axis_x_of(17, RenderDpi::DEFAULT), 37);
        assert_eq!(axis_x_of(28, RenderDpi::DEFAULT), 48);
        let mut dpi_obj = serde_json::Map::new();
        dpi_obj.insert("dpi".into(), serde_json::json!(106));
        let dpi106 = RenderDpi::from_request(&dpi_obj).unwrap();
        assert_eq!(dpi106.scale_px(INSET_TOP), 15);
        assert_eq!(dpi106.scale_px(INSET_BOTTOM), 38);
        assert_eq!(dpi106.scale_px(LABEL_EDGE), 10);
        assert_eq!(dpi106.scale_px(TICK_GAP), 12);
        assert_eq!(axis_x_of(7, dpi106), 29);
        assert_eq!(axis_x_of(19, dpi106), 41);
    }

    #[test]
    fn plot_insets_and_regular_grids_match_official_106dpi_edges() {
        let dpi106 =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        assert_eq!(scale_plot_leading(dpi106, INSET_TOP), 16);
        assert_eq!(scale_plot_trailing(dpi106, INSET_BOTTOM), 37);
        assert_eq!(scale_plot_leading(RenderDpi::DEFAULT, INSET_TOP), 14);
        assert_eq!(scale_plot_trailing(RenderDpi::DEFAULT, INSET_BOTTOM), 34);
        let dpi144 =
            RenderDpi::from_request(serde_json::json!({"dpi": 144}).as_object().unwrap()).unwrap();
        assert_eq!(scale_plot_leading(dpi144, INSET_TOP), 21);
        assert_eq!(scale_plot_trailing(dpi144, INSET_BOTTOM), 51);
        // Official untitled 636x371 / 424x265 grids at 106dpi.
        assert_eq!(
            (0..=9)
                .map(|i| regular_grid_y(16, 334, i, 9))
                .collect::<Vec<_>>(),
            vec![16, 51, 86, 122, 157, 192, 228, 263, 298, 334]
        );
        assert_eq!(regular_grid_y(16, 228, 0, 9), 16);
        assert_eq!(regular_grid_y(16, 228, 9, 9), 228);

        let series = line_ser(
            0,
            "S1",
            &[Some(4.0), Some(2.0), Some(3.0), Some(1.0)],
            r#"<c:marker><c:symbol val="none"/></c:marker>"#,
        );
        let doc = crate::xml::parse(standard_line_xml(&series, "").as_bytes()).unwrap();
        let png = render_chart_at(
            &doc.root,
            636,
            371,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
            dpi106,
        )
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bpp = match info.color_type {
            png::ColorType::Rgb => 3usize,
            png::ColorType::Rgba => 4usize,
            other => panic!("unexpected color type {other:?}"),
        };
        let w = info.width;
        let h = info.height;
        assert_eq!((w, h), (636, 371));
        let mut rows = Vec::new();
        for y in 1..h - 1 {
            let mut ink = 0u32;
            for x in 80..400 {
                let i = ((y * w + x) * bpp as u32) as usize;
                let r = buf[i];
                let g = buf[i + 1];
                let b = buf[i + 2];
                let mx = r.max(g).max(b);
                let mn = r.min(g).min(b);
                if mx < 230 && mx - mn < 20 {
                    ink += 1;
                }
            }
            if ink >= 160 {
                rows.push(y as i32);
            }
        }
        // t196u looser auto scale for data max 4 is 0-5/1 (6 majors) rather than
        // official untitled 0-4.5/0.5 (10 rows). Plot edges 16 and 334 are unchanged.
        assert_eq!(
            rows,
            vec![16, 79, 143, 206, 270, 334],
            "106dpi 636x371 majors follow 0-5/1; plot edges stay 16 and 334, got {rows:?}"
        );
    }

    #[test]
    fn half_pixel_lands_on_even() {
        assert_eq!(round_half_even(149.5), 150);
        assert_eq!(round_half_even(165.5), 166);
        assert_eq!(round_half_even(250.5), 250);
        assert_eq!(round_half_even(394.5), 394);
        assert_eq!(round_half_even(88.25), 88);
        assert_eq!(round_half_even(54.2), 54);
    }

    #[test]
    fn two_cell_size_follows_columns_and_zoom() {
        let sheet = crate::xml::parse(
            br#"<worksheet>
              <sheetViews><sheetView zoomScale="95" workbookViewId="0"/></sheetViews>
              <cols>
                <col min="1" max="1" width="16.7109375"/>
                <col min="2" max="3" width="14.7109375"/>
                <col min="4" max="7" width="12.7109375"/>
                <col min="8" max="8" width="22.7109375"/>
              </cols>
            </worksheet>"#,
        )
        .unwrap();
        let anchor = crate::xml::parse(
            br#"<twoCellAnchor>
              <from><col>0</col><colOff>0</colOff><row>18</row><rowOff>0</rowOff></from>
              <to><col>7</col><colOff>0</colOff><row>32</row><rowOff>0</rowOff></to>
            </twoCellAnchor>"#,
        )
        .unwrap();
        let (w, h) = bitmap_px(
            &sheet.root,
            &anchor.root,
            crate::xlsx::picture::AnchorKind::TwoCell,
            &super::super::geometry::GeometryContext::calibri11(),
        )
        .unwrap();
        assert_eq!((w, h), (647, 252));
    }

    #[test]
    fn one_cell_at_full_zoom_keeps_ext() {
        let sheet = crate::xml::parse(br#"<worksheet/>"#).unwrap();
        let anchor = crate::xml::parse(
            br#"<oneCellAnchor><ext cx="5486400" cy="3200400"/></oneCellAnchor>"#,
        )
        .unwrap();
        let (w, h) = bitmap_px(
            &sheet.root,
            &anchor.root,
            crate::xlsx::picture::AnchorKind::OneCell,
            &super::super::geometry::GeometryContext::calibri11(),
        )
        .unwrap();
        assert_eq!((w, h), (576, 336));
    }

    #[test]
    fn missing_num_cache_reads_cell_values() {
        let xml =
            br#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
          <c:chart><c:plotArea><c:barChart>
            <c:barDir val="col"/><c:grouping val="clustered"/>
            <c:ser>
              <c:order val="0"/>
              <c:tx><c:strRef><c:f>Data!B1</c:f></c:strRef></c:tx>
              <c:cat><c:strRef><c:f>Data!A2:A3</c:f></c:strRef></c:cat>
              <c:val><c:numRef><c:f>Data!B2:B3</c:f></c:numRef></c:val>
            </c:ser>
          </c:barChart></c:plotArea></c:chart>
        </c:chartSpace>"#;
        let doc = crate::xml::parse(xml).unwrap();
        let missed = render_rgb_chart(
            &doc.root,
            320,
            200,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "Data",
        );
        assert!(missed.is_err());
        let cells = super::super::sparkline::CellCache::from_pairs(
            "Data",
            &[
                ((2, 1), serde_json::json!("系列")),
                ((1, 2), serde_json::json!("甲")),
                ((1, 3), serde_json::json!("乙")),
                ((2, 2), serde_json::json!(10)),
                ((2, 3), serde_json::json!(30)),
            ],
        );
        let png = render_rgb_chart(
            &doc.root,
            320,
            200,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            Some(&cells),
            "Data",
        )
        .unwrap();
        assert!(png.len() > 100);
    }

    fn line_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn standard_line_xml(series: &str, chart_extra: &str) -> String {
        line_space(
            &format!(
                r#"<c:lineChart>
        <c:grouping val="standard"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:lineChart>"#
            ),
            chart_extra,
        )
    }

    fn lit_pts(values: &[Option<f64>]) -> String {
        values
            .iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                value.map(|v| format!(r#"<c:pt idx="{idx}"><c:v>{v}</c:v></c:pt>"#))
            })
            .collect()
    }

    fn line_ser(order: usize, name: &str, values: &[Option<f64>], extra: &str) -> String {
        let cats: String = (0..values.len())
            .map(|i| {
                format!(
                    r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#,
                    (b'A' + i as u8) as char
                )
            })
            .collect();
        format!(
            r#"<c:ser>
          <c:order val="{order}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          {extra}
          <c:cat><c:strLit>{cats}</c:strLit></c:cat>
          <c:val><c:numLit>{pts}</c:numLit></c:val>
        </c:ser>"#,
            pts = lit_pts(values)
        )
    }

    fn render_line(xml: &str) -> Result<Vec<u8>, Error> {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
    }

    fn png_rgb(png: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let data = buf[..info.buffer_size()].to_vec();
        assert_eq!(info.color_type, png::ColorType::Rgb);
        (info.width, info.height, data)
    }

    fn near(a: [u8; 3], b: [u8; 3], tol: i16) -> bool {
        (0..3).all(|i| (i16::from(a[i]) - i16::from(b[i])).abs() <= tol)
    }

    fn count_in(
        rgb: &[u8],
        w: u32,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
        target: [u8; 3],
        tol: i16,
    ) -> usize {
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 3) as usize;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn standard_line_paints_series_colors_and_gap_breaks_the_stroke() {
        let continuous = standard_line_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(20.0), Some(20.0), Some(20.0)], ""),
                line_ser(1, "S2", &[Some(5.0), Some(15.0), Some(8.0)], "")
            ),
            "",
        );
        let png = render_line(&continuous).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a1, 18) >= 20,
            "first series color should appear on the default line and markers"
        );
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a2, 18) >= 20,
            "second series color should appear on the default line and markers"
        );

        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let solid = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), Some(20.0), Some(20.0)], none),
            "",
        );
        let gapped = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), None, Some(20.0)], none),
            "",
        );
        let (_, _, solid_rgb) = png_rgb(&render_line(&solid).unwrap());
        let (_, _, gap_rgb) = png_rgb(&render_line(&gapped).unwrap());
        let mid0 = w / 3;
        let mid1 = (w * 2) / 3;
        let solid_mid = count_in(&solid_rgb, w, mid0, mid1, 0, h, a1, 24);
        let gap_mid = count_in(&gap_rgb, w, mid0, mid1, 0, h, a1, 24);
        assert!(
            solid_mid >= 10,
            "a continuous line should cross the middle category, got {solid_mid}"
        );
        assert_eq!(
            gap_mid, 0,
            "a missing value must break the stroke, not span the gap; got {gap_mid}"
        );
        let gap_sides = count_in(&gap_rgb, w, 0, mid0, 0, h, a1, 24)
            + count_in(&gap_rgb, w, mid1, w, 0, h, a1, 24);
        assert!(
            gap_sides >= 8,
            "the remaining points of a gapped series should still be drawn"
        );
    }

    #[test]
    fn empty_cache_v_plots_as_zero_and_connects_the_line() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let zero = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), Some(0.0), Some(20.0)], none),
            "",
        );
        let empty_v = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), Some(0.0), Some(20.0)], none).replace(
                r#"<c:pt idx="1"><c:v>0</c:v></c:pt>"#,
                r#"<c:pt idx="1"><c:v></c:v></c:pt>"#,
            ),
            "",
        );
        let omitted = standard_line_xml(
            &line_ser(0, "S1", &[Some(20.0), None, Some(20.0)], none),
            "",
        );
        let zero_png = render_line(&zero).unwrap();
        let empty_png = render_line(&empty_v).unwrap();
        let omitted_png = render_line(&omitted).unwrap();
        assert_eq!(
            empty_png, zero_png,
            "an empty cache <c:v></c:v> must plot as 0 and keep the stroke connected"
        );
        assert_ne!(
            empty_png, omitted_png,
            "an omitted cache point must stay a gap, not the same as an empty <c:v>"
        );
        let (w, h, empty_rgb) = png_rgb(&empty_png);
        let (_, _, omit_rgb) = png_rgb(&omitted_png);
        let a1 = FALLBACK_ACCENTS[0];
        let mid0 = w / 3;
        let mid1 = (w * 2) / 3;
        let empty_mid = count_in(&empty_rgb, w, mid0, mid1, 0, h, a1, 24);
        let omit_mid = count_in(&omit_rgb, w, mid0, mid1, 0, h, a1, 24);
        assert!(
            empty_mid >= 10,
            "empty cache v should connect through the middle category like 0, got {empty_mid}"
        );
        assert_eq!(
            omit_mid, 0,
            "an omitted cache point must still break the stroke; got {omit_mid}"
        );
    }

    #[test]
    fn parse_chart_number_locks_empty_text_zero_and_keeps_blanks_and_errors_as_gaps() {
        assert_eq!(parse_chart_number(""), Some(0.0));
        assert_eq!(parse_chart_number("   "), Some(0.0));
        assert_eq!(parse_chart_number("20"), Some(20.0));
        assert_eq!(parse_chart_number("-1.5"), Some(-1.5));
        assert_eq!(parse_chart_number("#N/A"), None);
        assert_eq!(json_number(serde_json::json!("")), Some(0.0));
        assert_eq!(json_number(serde_json::json!(null)), None);
        assert_eq!(json_number(serde_json::Value::String("#N/A".into())), None);
        assert_eq!(json_number(serde_json::json!(20)), Some(20.0));
    }

    #[test]
    fn empty_scatter_cache_v_plots_as_zero_and_connects_the_line() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let zero = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(0.0), Some(8.0)],
                none,
            ),
            "",
        );
        let empty_v = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(0.0), Some(8.0)],
                none,
            )
            .replace(
                r#"<c:pt idx="1"><c:v>0</c:v></c:pt>"#,
                r#"<c:pt idx="1"><c:v></c:v></c:pt>"#,
            ),
            "",
        );
        let omitted = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), None, Some(8.0)],
                none,
            ),
            "",
        );
        let zero_png = render_scatter(&zero).unwrap();
        let empty_png = render_scatter(&empty_v).unwrap();
        assert_eq!(
            empty_png, zero_png,
            "an empty scatter cache <c:v></c:v> must plot as 0 and keep the stroke connected"
        );
        assert_ne!(
            empty_png,
            render_scatter(&omitted).unwrap(),
            "an omitted scatter cache point must stay a gap"
        );
    }

    #[test]
    fn signed_scatter_auto_zero_axes_and_labels_meet_at_origin() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let signed = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(-2.0), Some(-1.0), Some(1.0), Some(3.0)],
                &[Some(-1.0), Some(2.0), Some(-2.0), Some(3.0)],
                none,
            ),
            "",
        );
        let (w, h, rgb) = png_rgb(&render_scatter(&signed).unwrap());
        assert_eq!((w, h), (576, 336));
        let dark = [0_u8, 0, 0];
        let mut col = vec![0_u32; w as usize];
        let mut row = vec![0_u32; h as usize];
        for y in 0..h {
            for x in 0..w {
                if near(
                    [
                        rgb[((y * w + x) * 3) as usize],
                        rgb[((y * w + x) * 3 + 1) as usize],
                        rgb[((y * w + x) * 3 + 2) as usize],
                    ],
                    dark,
                    40,
                ) {
                    col[x as usize] += 1;
                    row[y as usize] += 1;
                }
            }
        }
        let axis_x = col
            .iter()
            .enumerate()
            .max_by_key(|(_, n)| *n)
            .map(|(x, _)| x as u32)
            .unwrap();
        assert!(
            axis_x > w / 4 && axis_x < (w * 3) / 4,
            "vertical autoZero axis should sit at the origin, not the left edge; x={axis_x}"
        );
        let mid_axis_y = (h / 4..(h * 3) / 4)
            .max_by_key(|&y| row[y as usize])
            .unwrap();
        assert!(
            row[mid_axis_y as usize] >= w / 2,
            "a full-width horizontal autoZero axis should sit at the origin, not only at the bottom; y={mid_axis_y} count={}",
            row[mid_axis_y as usize]
        );
        let leftover_gutter = count_in(&rgb, w, 0, 36, 0, h / 3, dark, 40);
        let origin_y_labels = count_in(&rgb, w, axis_x - 36, axis_x, 0, h / 3, dark, 40);
        assert!(
            origin_y_labels > leftover_gutter,
            "Y tick labels should sit left of the origin axis, not in the left gutter; origin={origin_y_labels} gutter={leftover_gutter}"
        );

        let positive = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(4.0), Some(8.0)],
                &[Some(1.0), Some(3.0), Some(2.0)],
                none,
            ),
            "",
        );
        let (_, _, pos_rgb) = png_rgb(&render_scatter(&positive).unwrap());
        let mut pos_col = vec![0_u32; w as usize];
        for y in 0..h {
            for x in 0..w {
                if near(
                    [
                        pos_rgb[((y * w + x) * 3) as usize],
                        pos_rgb[((y * w + x) * 3 + 1) as usize],
                        pos_rgb[((y * w + x) * 3 + 2) as usize],
                    ],
                    dark,
                    40,
                ) {
                    pos_col[x as usize] += 1;
                }
            }
        }
        let pos_axis_x = pos_col
            .iter()
            .enumerate()
            .max_by_key(|(_, n)| *n)
            .map(|(x, _)| x as u32)
            .unwrap();
        assert!(
            pos_axis_x < w / 4,
            "when 0 is on the left bound, the vertical axis should stay on the left edge; x={pos_axis_x}"
        );
    }

    #[test]
    fn line_marker_none_is_not_the_same_as_default_marker() {
        let values = [Some(8.0), Some(16.0), Some(12.0)];
        let default_xml = standard_line_xml(&line_ser(0, "S1", &values, ""), "");
        let none_xml = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &values,
                r#"<c:marker><c:symbol val="none"/></c:marker>"#,
            ),
            "",
        );
        let (_, _, default_rgb) = png_rgb(&render_line(&default_xml).unwrap());
        let (_, _, none_rgb) = png_rgb(&render_line(&none_xml).unwrap());
        let a1 = FALLBACK_ACCENTS[0];
        let default_n = count_in(&default_rgb, 576, 0, 576, 0, 336, a1, 18);
        let none_n = count_in(&none_rgb, 576, 0, 576, 0, 336, a1, 18);
        assert!(
            default_n > none_n,
            "omitted markers should paint the default symbol, so they must add ink; default={default_n} none={none_n}"
        );
        assert!(none_n >= 8, "a visible solid line should still leave ink");
    }

    fn marker_el(symbol: &str, size: f64) -> String {
        format!(r#"<c:marker><c:symbol val="{symbol}"/><c:size val="{size}"/></c:marker>"#)
    }

    const AUTO_MARKER_SYMBOLS: [&str; 9] = [
        "diamond", "square", "triangle", "x", "star", "circle", "plus", "dot", "dash",
    ];

    fn line_series_with_markers(
        count: usize,
        values: &[Option<f64>],
        extra: impl Fn(usize) -> String,
    ) -> String {
        (0..count)
            .map(|i| line_ser(i, &format!("S{}", i + 1), values, &extra(i)))
            .collect::<String>()
    }

    fn scatter_series_with_markers(
        count: usize,
        xs: &[Option<f64>],
        ys: &[Option<f64>],
        extra: impl Fn(usize) -> String,
    ) -> String {
        (0..count)
            .map(|i| scatter_ser(i, &format!("S{}", i + 1), xs, ys, &extra(i)))
            .collect::<String>()
    }

    #[test]
    fn omitted_line_markers_follow_excel_auto_cycle_at_size_9() {
        let values = [Some(8.0), Some(16.0), Some(12.0)];
        let omitted = standard_line_xml(
            &line_series_with_markers(10, &values, |_| String::new()),
            "",
        );
        let explicit = standard_line_xml(
            &line_series_with_markers(10, &values, |i| {
                marker_el(AUTO_MARKER_SYMBOLS[i % AUTO_MARKER_SYMBOLS.len()], 9.0)
            }),
            "",
        );
        assert_eq!(
            render_line(&omitted).unwrap(),
            render_line(&explicit).unwrap(),
            "omitted c:marker must match the 9-symbol auto cycle at 9pt, wrapping at series 10"
        );
        let size5 = standard_line_xml(
            &line_series_with_markers(10, &values, |i| {
                marker_el(AUTO_MARKER_SYMBOLS[i % AUTO_MARKER_SYMBOLS.len()], 5.0)
            }),
            "",
        );
        assert_ne!(
            render_line(&omitted).unwrap(),
            render_line(&size5).unwrap(),
            "omitted markers use 9pt, not the OOXML explicit-marker default of 5pt"
        );
    }

    #[test]
    fn explicit_line_marker_symbol_wins_over_the_auto_cycle() {
        let values = [Some(8.0), Some(16.0), Some(12.0)];
        let omitted = standard_line_xml(&line_ser(0, "S1", &values, ""), "");
        let circle = standard_line_xml(&line_ser(0, "S1", &values, &marker_el("circle", 9.0)), "");
        let square = standard_line_xml(&line_ser(0, "S1", &values, &marker_el("square", 9.0)), "");
        let omitted_png = render_line(&omitted).unwrap();
        let circle_png = render_line(&circle).unwrap();
        let square_png = render_line(&square).unwrap();
        assert_ne!(
            omitted_png, circle_png,
            "series 0 default is diamond, so an explicit 9pt circle must differ"
        );
        assert_ne!(
            omitted_png, square_png,
            "an explicit square on series 0 must not be replaced by the default diamond"
        );
        render_line(&standard_line_xml(
            &line_ser(0, "S1", &values, &marker_el("diamond", 5.0)),
            "",
        ))
        .unwrap();
    }

    #[test]
    fn omitted_scatter_markers_follow_the_same_auto_cycle() {
        let xs = [Some(0.0), Some(4.0), Some(8.0)];
        let ys = [Some(1.0), Some(3.0), Some(2.0)];
        let omitted = scatter_xml(
            "lineMarker",
            &scatter_series_with_markers(10, &xs, &ys, |_| String::new()),
            "",
        );
        let explicit = scatter_xml(
            "lineMarker",
            &scatter_series_with_markers(10, &xs, &ys, |i| {
                marker_el(AUTO_MARKER_SYMBOLS[i % AUTO_MARKER_SYMBOLS.len()], 9.0)
            }),
            "",
        );
        assert_eq!(
            render_scatter(&omitted).unwrap(),
            render_scatter(&explicit).unwrap(),
            "omitted scatter markers share the line auto cycle at 9pt"
        );
        let circle = scatter_xml(
            "lineMarker",
            &scatter_ser(0, "S1", &xs, &ys, &marker_el("circle", 9.0)),
            "",
        );
        let omitted_s1 = scatter_xml("lineMarker", &scatter_ser(0, "S1", &xs, &ys, ""), "");
        assert_ne!(
            render_scatter(&omitted_s1).unwrap(),
            render_scatter(&circle).unwrap(),
            "explicit circle must win over the default diamond on scatter series 0"
        );
    }

    #[test]
    fn non_bubble_flag_is_inert_but_real_label_semantics_are_not() {
        let base = standard_line_xml(&line_ser(0, "S1", &[Some(1.0), Some(2.0)], ""), "");
        let bubble = base.replace(
            "<c:varyColors",
            "<c:dLbls><c:showBubbleSize val='1'/></c:dLbls><c:varyColors",
        );
        assert_eq!(render_line(&base).unwrap(), render_line(&bubble).unwrap());
        for unsupported in [
            "<c:showCatName val='1'/>",
            "<c:showPercent val='1'/>",
            "<c:showLegendKey val='1'/>",
            "<c:dLbl><c:idx val='0'/><c:delete val='1'/></c:dLbl>",
        ] {
            let xml = base.replace(
                "<c:varyColors",
                &format!("<c:dLbls>{unsupported}</c:dLbls><c:varyColors"),
            );
            assert!(
                render_line(&xml).is_err(),
                "must not silently omit {unsupported}"
            );
        }
    }

    #[test]
    fn line_zero_span_and_na_are_distinct_in_values_and_actual_strokes() {
        let ser = line_ser(
            0,
            "S1",
            &[Some(1.0), None, Some(3.0)],
            "<c:marker><c:symbol val='none'/></c:marker>",
        );
        let plot = Rect {
            x: 0.0,
            y: 0.0,
            w: 299.0,
            h: 100.0,
        };
        let parse = |mode: &str, ser: &str| {
            let xml = standard_line_xml(ser, &format!("<c:dispBlanksAs val='{mode}'/>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap()
        };
        let mut canvases = Vec::new();
        for mode in ["gap", "zero", "span"] {
            let chart = parse(mode, &ser);
            assert_eq!(
                chart.series[0].values[1],
                if mode == "zero" { Some(0.0) } else { None }
            );
            let mut canvas = Canvas::new(300, 110);
            draw_lines(&mut canvas, &chart, &plot, 0.0, 4.0, None, false, None).unwrap();
            canvases.push(canvas);
        }
        let ink = |canvas: &Canvas, x: usize, y: usize| {
            canvas.px[(y * 300 + x) * 3..(y * 300 + x) * 3 + 3] != [255; 3]
        };
        assert!(!ink(&canvases[0], 150, 50), "gap must not connect");
        assert!(
            ink(&canvases[1], 150, 99),
            "zero must draw down to baseline"
        );
        assert!(
            ink(&canvases[2], 150, 50),
            "span must actually connect across missing point"
        );
        let na = ser.replace(
            "</c:numLit>",
            "<c:pt idx='1'><c:v>#N/A</c:v></c:pt></c:numLit>",
        );
        for mode in ["gap", "zero", "span"] {
            let chart = parse(mode, &na);
            assert_eq!(chart.series[0].values[1], None, "NA is never a zero point");
            assert!(chart.series[0].unavailable_points[1]);
            let mut canvas = Canvas::new(300, 110);
            draw_lines(&mut canvas, &chart, &plot, 0.0, 4.0, None, false, None).unwrap();
            assert!(
                ink(&canvas, 150, 50),
                "Excel16 NA probe connects across NA: {mode}"
            );
        }
        let error = na.replace("#N/A", "#DIV/0!");
        for mode in ["zero", "span"] {
            let xml = standard_line_xml(&error, &format!("<c:dispBlanksAs val='{mode}'/>"));
            assert!(
                render_line(&xml).is_err(),
                "unverified error semantics must not silently be a blank/zero"
            );
        }
    }

    #[test]
    fn na_as_blank_extension_is_not_silently_treated_as_ordinary_na() {
        let base = standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "<c:dispBlanksAs val='zero'/>",
        );
        let finite=base.replace("</c:chart>","<c:extLst><c:ext uri='dataDisplayOptions16'><dispNaAsBlank xmlns='http://schemas.microsoft.com/office/drawing/2017/03/chart' val='1'/></c:ext></c:extLst></c:chart>");
        assert!(
            render_line(&finite).is_ok(),
            "inactive option must not be a false export failure"
        );
        let with_na = finite.replace(
            "<c:pt idx=\"1\"><c:v>2</c:v></c:pt>",
            "<c:pt idx=\"1\"><c:v>#N/A</c:v></c:pt>",
        );
        assert_ne!(with_na, finite, "counterexample must really contain NA");
        assert!(
            render_line(&with_na).is_err(),
            "active NA-as-blank has different valid semantics still needing implementation"
        );
        assert!(render_line(&with_na.replace("val='1'", "val='0'")).is_ok());
    }

    #[test]
    fn sparse_unresolved_cache_domain_is_not_silently_shortened_or_allocated_unbounded() {
        let ser = line_ser(0, "S1", &[Some(1.0), None, Some(3.0)], "");
        for count in [5_u64, 4_000_000_000_u64] {
            let ser = ser.replace(
                "<c:numLit>",
                &format!("<c:numLit><c:ptCount val='{count}'/>"),
            );
            for mode in ["gap", "zero", "span"] {
                let xml = standard_line_xml(&ser, &format!("<c:dispBlanksAs val='{mode}'/>"));
                assert!(render_line(&xml).is_err(),"unresolved domain must not lose trailing slots or allocate by untrusted ptCount");
            }
        }
    }

    #[test]
    fn line_blank_modes_keep_trailing_categories_and_cell_error_identity() {
        let ser = line_ser(0, "S1", &[Some(1.0), None, Some(3.0), None], "");
        let xml = standard_line_xml(&ser, "<c:dispBlanksAs val='zero'/>");
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(
            chart.series[0].values,
            vec![Some(1.0), Some(0.0), Some(3.0), Some(0.0)]
        );
        let cells = super::super::sparkline::CellCache::from_pairs(
            "Data",
            &[
                ((2, 1), serde_json::json!(1)),
                ((2, 3), serde_json::json!("#N/A")),
                ((2, 4), serde_json::json!(4)),
            ],
        );
        let parent =
            crate::xml::parse(b"<val><numRef><f>Data!B1:B4</f><numCache/></numRef></val>").unwrap();
        let (errors, na) = series_error_points(Some(&parent.root), "Data", Some(&cells), 4);
        assert_eq!(errors, vec![false, false, true, false]);
        assert_eq!(na, vec![false, false, true, false]);
    }

    #[test]
    fn point_label_deletion_only_validates_indices_and_preserves_ni() {
        let valid = "<dLbl><idx val='1'/><delete val='1'/></dLbl>";
        let read = |body: &str, len| {
            let doc = crate::xml::parse(format!("<dLbls>{body}</dLbls>").as_bytes()).unwrap();
            read_deleted_label_points(Some(&doc.root), len)
        };
        let deleted = |body: &str, len: usize| read(body, len).unwrap().deleted;
        assert_eq!(deleted(valid, 3), vec![1]);
        assert_eq!(
            deleted(&valid.replace("val='1'/></dLbl>", "/></dLbl>"), 3),
            vec![1]
        );
        let metadata = valid.replace("</dLbl>", "<extLst><ext uri='{CE6537A1-D6FC-4f65-9D91-7224C49458BB}'/><ext uri='{C3380CC4-5D6E-409C-BE32-E72D297353CC}'><uniqueId val='{00000001-2765-4173-A018-3A6CF43989F2}'/></ext></extLst></dLbl>");
        assert_eq!(deleted(&metadata, 3), vec![1]);
        assert_eq!(
            deleted(&valid.replace("delete val='1'", "delete val='0'"), 3),
            Vec::<usize>::new()
        );
        assert_eq!(
            read(&valid.replace("delete val='1'", "delete val='0'"), 3)
                .unwrap()
                .shown,
            vec![1]
        );
        assert_eq!(
            deleted(&valid.replace("<delete val='1'/>", "<showVal val='0'/>"), 3),
            vec![1]
        );
        assert_eq!(
            deleted(&valid.replace("<delete val='1'/>", "<showVal val='1'/>"), 3),
            Vec::<usize>::new()
        );
        for bad in [
            valid.replace("idx val='1'", "idx val='-1'"),
            valid.replace("idx val='1'", "idx val='3'"),
            valid.replace("idx val='1'", "idx val='1.5'"),
            valid.replace("idx val='1'", "idx val=''"),
            valid.replace("idx val='1'", "idx val='184467440737095516160'"),
            valid.replace("<idx val='1'/>", ""),
            valid.replace("<delete val='1'/>", ""),
            valid.replace("</dLbl>", "<showVal val='1'/></dLbl>"),
            valid.replace("delete val='1'", "delete val='0'").replace("</dLbl>", "<showVal val='0'/></dLbl>"),
            valid.replace("</dLbl>", "<spPr><solidFill/></spPr></dLbl>"),
            valid.replace("</dLbl>", "<dLblPos val='outEnd'/></dLbl>"),
            valid.replace("</dLbl>", "<tx><rich/></tx></dLbl>"),
            metadata.replace("uniqueId", "showLeaderLines"),
            metadata.replace("{CE6537A1-D6FC-4f65-9D91-7224C49458BB}", "unknown"),
            format!("{valid}{valid}"),
        ] {
            assert_eq!(
                read(&bad, 3).unwrap_err().code(),
                crate::error::ErrorCode::NotImplemented,
                "{bad}"
            );
        }
        let wrapper = crate::xml::parse(b"<wrapper/>").unwrap();
        let ser = crate::xml::parse(
            format!("<ser><dLbls>{valid}<showVal val='1'/></dLbls></ser>").as_bytes(),
        )
        .unwrap();
        assert_eq!(
            read_value_label(&wrapper.root, &ser.root, PlotKind::ClusteredColumn).unwrap(),
            Some(ValueLabel::OutsideEnd)
        );
        assert!(
            read_value_label(&ser.root, &wrapper.root, PlotKind::ClusteredColumn).is_err(),
            "wrapper-level per-point overrides remain NI"
        );
        let other_kind = crate::xml::parse(format!("<dLbls>{valid}</dLbls>").as_bytes()).unwrap();
        assert!(
            data_labels_hidden(Some(&other_kind.root)).is_err(),
            "other chart kinds remain NI"
        );

        let keep = "<c:dLbls><c:dLbl><c:idx val='1'/><c:delete val='0'/></c:dLbl><c:showVal val='1'/></c:dLbls>";
        let hide = "<c:dLbls><c:dLbl><c:idx val='1'/><c:showVal val='0'/></c:dLbl><c:showVal val='1'/></c:dLbls>";
        let reveal = "<c:dLbls><c:dLbl><c:idx val='1'/><c:showVal val='1'/></c:dLbl><c:showVal val='0'/></c:dLbls>";
        for (labels, expect_deleted) in [(keep, Vec::<usize>::new()), (hide, vec![1])] {
            let ser = line_ser(0, "S1", &[Some(4.0), Some(6.0), Some(9.0)], labels);
            let xml = bar_xml(&ser, "", "minMax", Some("col"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let chart =
                read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
            assert_eq!(chart.series[0].deleted_label_points, expect_deleted);
            assert!(chart.series[0].value_label.is_some());
        }
        let ser = line_ser(0, "S1", &[Some(4.0), Some(6.0), Some(9.0)], reveal);
        let xml = bar_xml(&ser, "", "minMax", Some("col"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        assert!(
            read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "point showVal=1 on a hidden series stays NI until those labels can be painted"
        );
    }

    #[cfg(windows)]
    #[test]
    fn deleted_point_labels_remove_only_the_requested_glyphs() {
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            for kind in [
                PlotKind::ClusteredColumn,
                PlotKind::ClusteredBar,
                PlotKind::StandardLine,
            ] {
                let pos = if kind == PlotKind::StandardLine {
                    "r"
                } else {
                    "outEnd"
                };
                let labels =
                    format!("<c:dLbls><c:showVal val='1'/><c:dLblPos val='{pos}'/></c:dLbls>");
                let ser = line_ser(0, "S1", &[Some(4.0), Some(6.0), Some(9.0)], &labels);
                let base = match kind {
                    PlotKind::StandardLine => {
                        standard_line_xml(&ser, "<c:dispBlanksAs val='gap'/>")
                    }
                    PlotKind::ClusteredColumn => bar_xml(&ser, "", "minMax", Some("col")),
                    PlotKind::ClusteredBar => bar_xml(&ser, "", "minMax", Some("bar")),
                    _ => unreachable!(),
                };
                let deleted = base.replace(
                    "<c:dLbls>",
                    "<c:dLbls><c:dLbl><c:idx val='1'/><c:delete val='1'/></c:dLbl>",
                );
                let control_doc = crate::xml::parse(base.as_bytes()).unwrap();
                let deleted_doc = crate::xml::parse(deleted.as_bytes()).unwrap();
                let control_chart =
                    read_chart(&control_doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                let deleted_chart =
                    read_chart(&deleted_doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                assert_eq!(deleted_chart.series[0].deleted_label_points, vec![1]);
                assert_eq!(
                    deleted_chart.series[0].values, control_chart.series[0].values,
                    "deleting labels must not delete chart marks"
                );
                let face = super::super::chart_gdi::Face::open(14.0, false).unwrap();
                let plot = Rect {
                    x: 100.0,
                    y: 50.0,
                    w: 300.0,
                    h: 220.0,
                };
                let mut control = Canvas::with_dpi(576, 336, dpi);
                let mut actual = Canvas::with_dpi(576, 336, dpi);
                for (canvas, chart) in [
                    (&mut control, &control_chart),
                    (&mut actual, &deleted_chart),
                ] {
                    if kind == PlotKind::ClusteredBar {
                        draw_horizontal_value_labels(
                            canvas,
                            chart,
                            &plot,
                            0.0,
                            10.0,
                            None,
                            Some(&face),
                            14.0,
                            false,
                        )
                        .unwrap();
                    } else {
                        draw_value_labels(canvas, chart, &plot, 0.0, 10.0, None, Some(&face), 14.0)
                            .unwrap();
                    }
                }
                let mut removed = 0;
                let mut kept = 0;
                for y in 0..336 {
                    for x in 0..576 {
                        let i = (y * 576 + x) * 3;
                        let before = &control.px[i..i + 3];
                        let after = &actual.px[i..i + 3];
                        let middle = if kind == PlotKind::ClusteredBar {
                            (270..330).contains(&x) && (140..180).contains(&y)
                        } else {
                            (225..285).contains(&x) && (100..155).contains(&y)
                        };
                        if middle {
                            assert_eq!(after, [255; 3], "deleted glyph must disappear");
                            removed += usize::from(before != after);
                        } else {
                            assert_eq!(before, after, "unrelated point labels must not move");
                            kept += usize::from(after != [255; 3]);
                        }
                    }
                }
                assert!(
                    removed > 0 && kept > 0,
                    "both real deleted and retained glyphs required at {dpi_value}"
                );
            }
        }
    }

    #[test]
    fn inside_end_is_column_only_and_zero_is_explicit_ni() {
        let wrapper = crate::xml::parse(b"<wrapper/>").unwrap();
        let ser = crate::xml::parse(
            b"<ser><dLbls><showVal val='1'/><dLblPos val='inEnd'/></dLbls></ser>",
        )
        .unwrap();
        assert_eq!(
            read_value_label(&wrapper.root, &ser.root, PlotKind::ClusteredColumn).unwrap(),
            Some(ValueLabel::InsideEnd)
        );
        assert_eq!(
            read_value_label(&wrapper.root, &ser.root, PlotKind::ClusteredBar).unwrap(),
            Some(ValueLabel::InsideEnd)
        );
        assert!(read_value_label(&wrapper.root, &ser.root, PlotKind::StandardLine).is_err());
        let ser = line_ser(
            0,
            "S1",
            &[Some(0.0), Some(6.0), Some(9.0)],
            "<c:dLbls><c:showVal val='1'/><c:dLblPos val='inEnd'/></c:dLbls>",
        );
        let xml = bar_xml(&ser, "", "minMax", Some("col"));
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("column inEnd with a zero must parse so the 0 label can fall back outside");
        assert_eq!(chart.series[0].value_label, Some(ValueLabel::InsideEnd));
        let mixed = bar_xml(
            &line_ser(
                0,
                "S1",
                &[Some(-4.0), Some(0.0), Some(9.0)],
                "<c:dLbls><c:showVal val='1'/><c:dLblPos val='inEnd'/></c:dLbls>",
            ),
            "",
            "minMax",
            Some("col"),
        );
        let mixed_doc = crate::xml::parse(mixed.as_bytes()).unwrap();
        let mixed_chart = read_chart(
            &mixed_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .expect("mixed inEnd with a zero must export rather than drop the 0 label");
        assert_eq!(
            mixed_chart.series[0].value_label,
            Some(ValueLabel::InsideEnd)
        );
    }

    #[cfg(windows)]
    #[test]
    fn column_inside_end_labels_paint_inside_marks_and_reject_nonfitting_boxes() {
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            for sign in [-1.0, 1.0] {
                let values = [Some(4.0 * sign), Some(6.0 * sign), Some(9.0 * sign)];
                let ser = line_ser(
                    0,
                    "S1",
                    &values,
                    "<c:dLbls><c:showVal val='1'/><c:dLblPos val='inEnd'/></c:dLbls>",
                );
                let xml = bar_xml(&ser, "", "minMax", Some("col"));
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                let face = super::super::chart_gdi::Face::open(14.0, false).unwrap();
                let plot = Rect {
                    x: 80.0,
                    y: 30.0,
                    w: 400.0,
                    h: 280.0,
                };
                let (lo, hi) = if sign < 0.0 {
                    (-10.0, 0.0)
                } else {
                    (0.0, 10.0)
                };
                let mut canvas = Canvas::with_dpi(576, 336, dpi);
                draw_value_labels(&mut canvas, &chart, &plot, lo, hi, None, Some(&face), 14.0)
                    .unwrap();
                let slot = (plot.w + 1.0) / 3.0;
                let mark_width = slot / (1.0 + chart.gap_width / 100.0);
                let baseline = y_of(0.0, lo, hi, &plot);
                let mut ink = [0; 3];
                for y in 0..336 {
                    for x in 0..576 {
                        let i = (y * 576 + x) * 3;
                        if canvas.px[i..i + 3] == [255; 3] {
                            continue;
                        }
                        let mut found = false;
                        for ci in 0..3 {
                            let center = plot.x + slot * (ci as f64 + 0.5);
                            let end = y_of(values[ci].unwrap(), lo, hi, &plot);
                            if (x as f64 - center).abs() <= mark_width / 2.0
                                && (y as f64) >= baseline.min(end)
                                && (y as f64) <= baseline.max(end)
                            {
                                ink[ci] += 1;
                                found = true;
                            }
                        }
                        assert!(found, "inside-end glyph escaped every mark");
                    }
                }
                assert!(
                    ink.into_iter().all(|n| n > 0),
                    "every value must paint real glyph ink"
                );
                for small in [
                    Rect {
                        x: 80.0,
                        y: 30.0,
                        w: 20.0,
                        h: 280.0,
                    },
                    Rect {
                        x: 80.0,
                        y: 30.0,
                        w: 400.0,
                        h: 20.0,
                    },
                ] {
                    let mut canvas = Canvas::with_dpi(576, 336, dpi);
                    assert!(
                        draw_value_labels(
                            &mut canvas,
                            &chart,
                            &small,
                            lo,
                            hi,
                            None,
                            Some(&face),
                            14.0
                        )
                        .is_err(),
                        "short/narrow boxes remain NI instead of clipped/fitted"
                    );
                }
            }
        }
    }

    #[test]
    fn plain_value_labels_are_parsed_and_unsupported_styles_are_never_omitted() {
        for kind in [
            PlotKind::ClusteredColumn,
            PlotKind::ClusteredBar,
            PlotKind::StandardLine,
        ] {
            let wrapper = crate::xml::parse(
                b"<wrapper><dLbls><showVal val='1'/><showBubbleSize val='1'/></dLbls></wrapper>",
            )
            .unwrap();
            let ser = crate::xml::parse(b"<ser/>").unwrap();
            assert_eq!(
                read_value_label(&wrapper.root, &ser.root, kind).unwrap(),
                Some(if kind == PlotKind::StandardLine {
                    ValueLabel::Right
                } else {
                    ValueLabel::OutsideEnd
                })
            );
            for config in [
                "<showCatName val='1'/>",
                "<numFmt formatCode='0.00'/>",
                "<dLblPos val='bestFit'/>",
                "<txPr/>",
                "<dLbl><idx val='0'/></dLbl>",
                "<spPr><solidFill/></spPr>",
            ] {
                let ser = crate::xml::parse(
                    format!("<ser><dLbls><showVal val='1'/>{config}</dLbls></ser>").as_bytes(),
                )
                .unwrap();
                assert!(
                    read_value_label(&wrapper.root, &ser.root, kind).is_err(),
                    "must not ignore {config}"
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn horizontal_outside_end_labels_paint_and_keep_other_subranges_ni() {
        let base = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(-4.0), Some(0.0), Some(9.0)],
                "<c:dLbls><c:showVal val='1'/><c:dLblPos val='outEnd'/></c:dLbls>",
            ),
            "<c:dispBlanksAs val='gap'/>",
        )
        .replace("lineChart", "barChart")
        .replace(
            "<c:grouping val=\"standard\"/>",
            "<c:barDir val='bar'/><c:grouping val='clustered'/>",
        );
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            for orient in ["minMax", "maxMin"] {
                let xml = base.replace(
                    "<c:catAx/>",
                    &format!("<c:catAx><c:axPos val='l'/><c:scaling><c:orientation val='{orient}'/></c:scaling></c:catAx>"),
                );
                let xml = xml.replace("<c:valAx>", "<c:valAx><c:axPos val='b'/>");
                let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", dpi).unwrap();
                assert_eq!(chart.series[0].value_label, Some(ValueLabel::OutsideEnd));
                let mut actual = Canvas::with_dpi(576, 336, dpi);
                let face = super::super::chart_gdi::Face::open(14.0, false).unwrap();
                let plot = Rect {
                    x: 100.0,
                    y: 50.0,
                    w: 300.0,
                    h: 220.0,
                };
                draw_horizontal_value_labels(
                    &mut actual,
                    &chart,
                    &plot,
                    -10.0,
                    10.0,
                    None,
                    Some(&face),
                    14.0,
                    false,
                )
                .unwrap();
                // Left of the negative end, right of zero, and right of positive
                // end must each contain actual glyph ink (not success + omission).
                let pad = dpi.pt_to_px_font(6.0);
                for (ci, v) in [-4.0, 0.0, 9.0].into_iter().enumerate() {
                    let end = x_of(v, -10.0, 10.0, &plot);
                    let y = category_mid_y(&plot, ci, 3, chart.cats_from_bottom);
                    let xr = if v < 0.0 {
                        (end - pad - 25.0) as i32..(end - pad + 1.0) as i32
                    } else {
                        (end + pad) as i32..(end + pad + 20.0) as i32
                    };
                    let mut ink = 0;
                    for yy in (y - 16.0) as i32..(y + 16.0) as i32 {
                        for xx in xr.clone() {
                            if xx >= 0 && yy >= 0 && xx < 576 && yy < 336 {
                                let i = (yy as usize * 576 + xx as usize) * 3;
                                ink += usize::from(actual.px[i..i + 3] != [255; 3]);
                            }
                        }
                    }
                    assert!(ink > 0, "missing value {v} at {dpi_value}/{orient}");
                }
            }
        }
        let axes = |xml: String| {
            xml.replace("<c:valAx>", "<c:valAx><c:axPos val='b'/>")
                .replace("<c:catAx/>", "<c:catAx><c:axPos val='l'/></c:catAx>")
        };
        // Official inEnd zero falls back outside; the chart must export.
        render_line(&axes(base.replace("val='outEnd'", "val='inEnd'")))
            .expect("bar inEnd with a zero value should export via outside fallback");
        // General decimals on the locked outEnd path now export.
        assert!(
            render_line(&axes(base.replace("<c:v>9</c:v>", "<c:v>9.5</c:v>"))).is_ok(),
            "bar General decimal outEnd should export"
        );
        assert!(
            render_line(&axes(base.replace("<c:v>9</c:v>", "<c:v>#N/A</c:v>"))).is_err(),
            "bar error-point labels must remain NI"
        );
    }

    #[test]
    fn value_label_missing_and_error_semantics_are_never_silently_omitted() {
        let base = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(1.0), None, Some(3.0)],
                "<c:dLbls><c:showVal val='1'/></c:dLbls>",
            ),
            "<c:dispBlanksAs val='zero'/>",
        );
        let column = base.replace("lineChart", "barChart").replace(
            "<c:grouping val=\"standard\"/>",
            "<c:barDir val='col'/><c:grouping val='clustered'/>",
        );
        assert!(
            render_line(&column).is_err(),
            "unlocked column blank labels must not silently disappear"
        );
        let error = base
            .replace(
                "</c:numLit>",
                "<c:pt idx='1'><c:v>#DIV/0!</c:v></c:pt></c:numLit>",
            )
            .replace("val='zero'", "val='gap'");
        assert!(
            render_line(&error).is_err(),
            "unsupported error label semantics are not a successful omission"
        );
    }

    #[test]
    fn label_inheritance_and_integer_format_boundary_are_explicit() {
        let root =
            crate::xml::parse(b"<wrapper><dLbls><showVal val='0'/><txPr/></dLbls></wrapper>")
                .unwrap();
        let ser = crate::xml::parse(b"<ser><dLbls><showVal val='1'/></dLbls></ser>").unwrap();
        assert!(
            read_value_label(&root.root, &ser.root, PlotKind::StandardLine).is_err(),
            "must not silently drop inherited custom text styling"
        );
        assert_eq!(value_label_text(-0.0), Some("0".to_owned()));
        assert_eq!(value_label_text(-42.0), Some("-42".to_owned()));
        assert_eq!(value_label_text(1.25), Some("1.25".to_owned()));
        assert_eq!(value_label_text(4.5), Some("4.5".to_owned()));
        assert_eq!(value_label_text(6.3), Some("6.3".to_owned()));
        assert_eq!(
            value_label_text(9.1999999999999993),
            Some("9.2".to_owned())
        );
        for value in [f64::INFINITY, 100_000_000_000.0] {
            assert!(value_label_text(value).is_none());
        }
        let xml = standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(1.25), Some(2.0)],
                "<c:dLbls><c:showVal val='1'/></c:dLbls>",
            ),
            "",
        );
        assert!(
            render_line(&xml).is_ok(),
            "General decimal labels should export with the close-enough formatter"
        );
    }

    #[cfg(windows)]
    #[test]
    fn value_labels_use_actual_gdi_line_box_not_fallback_font_leading() {
        let face = super::super::chart_gdi::Face::open_dpi(15.0, false, 106.0).unwrap();
        assert_eq!(value_label_height(None, Some(&face), 50.0), face.em_box());
        assert_ne!(
            value_label_height(None, Some(&face), 50.0),
            text_height(None, 50.0)
        );
    }

    #[cfg(windows)]
    #[test]
    fn real_value_label_glyph_pixels_are_added_for_column_and_line_at_each_dpi() {
        let base = standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "",
        );
        for kind in [PlotKind::ClusteredColumn, PlotKind::StandardLine] {
            let hidden = if kind == PlotKind::ClusteredColumn {
                base.replace("lineChart", "barChart").replace(
                    "<c:grouping val=\"standard\"/>",
                    "<c:barDir val='col'/><c:grouping val='clustered'/>",
                )
            } else {
                base.clone()
            };
            let visible = hidden.replace(
                "<c:order",
                "<c:dLbls><c:showVal val='1'/></c:dLbls><c:order",
            );
            for dpi_value in [96, 106, 144] {
                let dpi = RenderDpi::from_request(
                    serde_json::json!({"dpi":dpi_value}).as_object().unwrap(),
                )
                .unwrap();
                let mut images = Vec::new();
                for xml in [&hidden, &visible] {
                    let doc = crate::xml::parse(xml.as_bytes()).unwrap();
                    let png = render_chart_at(
                        &doc.root,
                        636,
                        371,
                        FALLBACK_ACCENTS,
                        None,
                        ChartHost::Excel,
                        None,
                        "",
                        dpi,
                    )
                    .unwrap();
                    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
                        .read_info()
                        .unwrap();
                    let mut bytes = vec![0; reader.output_buffer_size()];
                    reader.next_frame(&mut bytes).unwrap();
                    images.push(bytes);
                }
                assert!(
                    images[0]
                        .iter()
                        .zip(&images[1])
                        .filter(|(a, b)| a != b)
                        .count()
                        > 30,
                    "labels must add actual glyph pixels, not just remove NI"
                );
            }
        }
    }

    #[test]
    fn unsupported_line_inputs_stay_not_implemented() {
        let base = line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], "");
        let cases = [
            standard_line_xml(&base, "")
                .replace(r#"grouping val="standard""#, r#"grouping val="stacked""#),
            standard_line_xml(&base, "").replace(
                r#"grouping val="standard""#,
                r#"grouping val="percentStacked""#,
            ),
            standard_line_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0), Some(3.0)],
                    r#"<c:smooth val="1"/>"#,
                ),
                "",
            ),
            standard_line_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0), Some(3.0)],
                    r#"<c:spPr><a:ln w="28575"><a:prstDash val="dash"/><a:solidFill><a:srgbClr val="156082"/></a:solidFill></a:ln></c:spPr>"#,
                ),
                "",
            ),
            standard_line_xml(&base, "").replace(r#"varyColors val="0""#, r#"varyColors val="1""#),
            line_space(
                r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:barChart><c:lineChart><c:grouping val="standard"/><c:ser>
          <c:order val="1"/><c:val><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:lineChart>"#,
                "",
            ),
            line_space(
                r#"<c:pieChart><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:pieChart>"#,
                "",
            ),
            standard_line_xml(&base, "").replace(
                r#"crossBetween val="between""#,
                r#"crossBetween val="midCat""#,
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_line(xml).expect_err(&format!("case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "case {i} should stay NotImplemented"
            );
        }

        // A supported standard line still renders, so the counterexamples are not a broken harness.
        render_line(&standard_line_xml(&base, "")).unwrap();

        render_line(&standard_line_xml(
            &line_ser(
                0,
                "S1",
                &[Some(1.0), Some(2.0), Some(3.0)],
                r#"<c:marker><c:symbol val="circle"/><c:size val="5"/></c:marker>"#,
            ),
            r#"<c:dispBlanksAs val="gap"/>"#,
        ))
        .unwrap();
    }

    fn scatter_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:valAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="b"/><c:crosses val="autoZero"/></c:valAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="l"/><c:majorGridlines/><c:crosses val="autoZero"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn scatter_xml(style: &str, series: &str, chart_extra: &str) -> String {
        scatter_space(
            &format!(
                r#"<c:scatterChart>
        <c:scatterStyle val="{style}"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:scatterChart>"#
            ),
            chart_extra,
        )
    }

    fn scatter_ser(
        order: usize,
        name: &str,
        xs: &[Option<f64>],
        ys: &[Option<f64>],
        extra: &str,
    ) -> String {
        format!(
            r#"<c:ser>
          <c:order val="{order}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          {extra}
          <c:xVal><c:numLit>{xs}</c:numLit></c:xVal>
          <c:yVal><c:numLit>{ys}</c:numLit></c:val>
        </c:ser>"#,
            xs = lit_pts(xs),
            ys = lit_pts(ys)
        )
        .replace("</c:val>", "</c:yVal>")
    }

    fn render_scatter(xml: &str) -> Result<Vec<u8>, Error> {
        render_line(xml)
    }

    #[test]
    fn marked_scatter_uses_numeric_xy_and_paints_series_colors() {
        let xml = scatter_xml(
            "lineMarker",
            &format!(
                "{}{}",
                scatter_ser(
                    0,
                    "S1",
                    &[Some(0.0), Some(8.0)],
                    &[Some(1.0), Some(1.0)],
                    ""
                ),
                scatter_ser(
                    1,
                    "S2",
                    &[Some(1.0), Some(7.0)],
                    &[Some(4.0), Some(6.0)],
                    ""
                )
            ),
            "",
        );
        let png = render_scatter(&xml).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a1, 18) >= 20,
            "first scatter series color should appear"
        );
        assert!(
            count_in(&rgb, w, 0, w, 0, h, a2, 18) >= 20,
            "second scatter series color should appear"
        );

        // A lone point at the right of the X scale must not sit on a category midpoint.
        let right = scatter_xml(
            "marker",
            &scatter_ser(0, "S1", &[Some(8.0)], &[Some(2.0)], ""),
            "",
        );
        let (_, _, right_rgb) = png_rgb(&render_scatter(&right).unwrap());
        let left_count = count_in(&right_rgb, w, 0, w / 2, 0, h, a1, 18);
        let right_count = count_in(&right_rgb, w, w / 2, w, 0, h, a1, 18);
        assert!(
            right_count > left_count,
            "x=8 on a 0..=8-like scale should land in the right half, not a single category mid-slot; left={left_count} right={right_count}"
        );
    }

    #[test]
    fn scatter_gap_breaks_the_stroke_and_hidden_line_drops_the_stroke() {
        let none = r#"<c:marker><c:symbol val="none"/></c:marker>"#;
        let solid = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                none,
            ),
            "",
        );
        let gapped = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), None, Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                none,
            ),
            "",
        );
        let (_, _, solid_rgb) = png_rgb(&render_scatter(&solid).unwrap());
        let (_, _, gap_rgb) = png_rgb(&render_scatter(&gapped).unwrap());
        let a1 = FALLBACK_ACCENTS[0];
        let mid0 = 576 / 3;
        let mid1 = (576 * 2) / 3;
        let solid_mid = count_in(&solid_rgb, 576, mid0, mid1, 0, 336, a1, 24);
        let gap_mid = count_in(&gap_rgb, 576, mid0, mid1, 0, 336, a1, 24);
        assert!(
            solid_mid >= 10,
            "a lineMarker series should cross the middle x when every point is present, got {solid_mid}"
        );
        assert_eq!(
            gap_mid, 0,
            "a missing x or y must break the scatter stroke; got {gap_mid}"
        );

        let markers = scatter_xml(
            "marker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                none,
            ),
            "",
        );
        let marker_png = render_scatter(&markers).unwrap();
        assert_eq!(
            marker_png,
            render_scatter(&solid).unwrap(),
            "scatterStyle=marker with no series line hide must connect like lineMarker"
        );

        let no_line = scatter_xml(
            "marker",
            &scatter_ser(
                0,
                "S1",
                &[Some(0.0), Some(5.0), Some(10.0)],
                &[Some(8.0), Some(8.0), Some(8.0)],
                r#"<c:spPr><a:ln><a:noFill/></a:ln></c:spPr><c:marker><c:symbol val="circle"/><c:size val="9"/></c:marker>"#,
            ),
            "",
        );
        let (_, _, hidden_rgb) = png_rgb(&render_scatter(&no_line).unwrap());
        let between = count_in(&hidden_rgb, 576, 576 / 6, mid0 - 10, 0, 336, a1, 18);
        let marker_mid = count_in(&hidden_rgb, 576, mid0 + 20, mid1 - 20, 0, 336, a1, 18);
        assert_eq!(
            between, 0,
            "an explicit series a:ln/a:noFill must drop the connecting stroke; got {between} (mid markers {marker_mid})"
        );
        assert!(
            marker_mid >= 1,
            "hiding the scatter line should still leave the middle marker, got {marker_mid}"
        );
    }

    #[test]
    fn unsupported_scatter_inputs_stay_not_implemented() {
        let ser = scatter_ser(
            0,
            "S1",
            &[Some(1.0), Some(2.0), Some(3.0)],
            &[Some(4.0), Some(5.0), Some(6.0)],
            "",
        );
        let cases = [
            scatter_xml("line", &ser, ""),
            scatter_xml("none", &ser, ""),
            scatter_xml("smoothMarker", &ser, ""),
            scatter_xml("smooth", &ser, ""),
            scatter_xml(
                "lineMarker",
                &scatter_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    &[Some(3.0), Some(4.0)],
                    r#"<c:spPr><a:ln w="28575"><a:prstDash val="dash"/><a:solidFill><a:srgbClr val="156082"/></a:solidFill></a:ln></c:spPr>"#,
                ),
                "",
            ),
            scatter_xml("lineMarker", &ser, r#"<c:dispBlanksAs val="zero"/>"#),
            scatter_xml("lineMarker", &ser, r#"<c:dispBlanksAs val="span"/>"#),
            scatter_xml("lineMarker", &ser, "")
                .replace(r#"varyColors val="0""#, r#"varyColors val="1""#),
            scatter_space(
                r#"<c:bubbleChart><c:ser>
          <c:order val="0"/><c:xVal><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:xVal>
          <c:yVal><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt></c:numLit></c:yVal>
        </c:ser></c:bubbleChart>"#,
                "",
            ),
            scatter_xml("lineMarker", &ser, "").replace(
                "</c:plotArea>",
                r#"<c:catAx><c:axId val="3"/><c:axPos val="b"/></c:catAx></c:plotArea>"#,
            ),
            scatter_xml(
                "lineMarker",
                &scatter_ser(0, "S1", &[None, None], &[Some(1.0), Some(2.0)], ""),
                "",
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_scatter(xml)
                .expect_err(&format!("scatter case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "scatter case {i} should stay NotImplemented"
            );
        }
        render_scatter(&scatter_xml("marker", &ser, "")).unwrap();
        render_scatter(&scatter_xml("lineMarker", &ser, "")).unwrap();
        // Omitted scatterStyle follows Excel's lineMarker default.
        render_scatter(
            &scatter_xml("lineMarker", &ser, "")
                .replace(r#"<c:scatterStyle val="lineMarker"/>"#, ""),
        )
        .unwrap();

        // Official Excel 16 scatter valAx writes midCat. Locked styles still
        // paint; this is not the category-slot midCat rule.
        let midcat = scatter_xml("lineMarker", &ser, "").replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        assert_eq!(
            midcat.matches(r#"<c:crossBetween val="midCat"/>"#).count(),
            2,
            "both official scatter valAx should carry midCat",
        );
        render_scatter(&midcat).unwrap();
        let line_no_markers = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(1.0), Some(2.0), Some(3.0)],
                &[Some(4.0), Some(5.0), Some(6.0)],
                r#"<c:marker><c:symbol val="none"/></c:marker><c:smooth val="0"/>"#,
            ),
            "",
        )
        .replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        render_scatter(&line_no_markers).unwrap();
        let markers_only = scatter_xml(
            "lineMarker",
            &scatter_ser(
                0,
                "S1",
                &[Some(1.0), Some(2.0), Some(3.0)],
                &[Some(4.0), Some(5.0), Some(6.0)],
                r#"<c:spPr><a:ln w="38100"><a:noFill/></a:ln></c:spPr><c:smooth val="0"/>"#,
            ),
            "",
        )
        .replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        render_scatter(&markers_only).unwrap();
        let mixed = scatter_xml("lineMarker", &ser, "").replacen(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
            1,
        );
        assert!(
            render_scatter(&mixed).is_err(),
            "mixed midCat/between scatter valAx stays NI"
        );
        let smooth_midcat = scatter_xml("smoothMarker", &ser, "").replace(
            "</c:valAx>",
            r#"<c:crossBetween val="midCat"/></c:valAx>"#,
        );
        assert!(
            render_scatter(&smooth_midcat).is_err(),
            "smoothMarker stays NI even with official midCat"
        );
    }

    fn bar_space(plot_body: &str, chart_extra: &str, cat_orient: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="{cat_orient}"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn bar_xml(series: &str, chart_extra: &str, cat_orient: &str, dir: Option<&str>) -> String {
        let dir_el = match dir {
            Some(dir) => format!(r#"<c:barDir val="{dir}"/>"#),
            None => String::new(),
        };
        bar_space(
            &format!(
                r#"<c:barChart>
        {dir_el}
        <c:grouping val="clustered"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:barChart>"#
            ),
            chart_extra,
            cat_orient,
        )
    }

    fn render_bar(xml: &str) -> Result<Vec<u8>, Error> {
        render_line(xml)
    }

    fn centroid_y(
        rgb: &[u8],
        w: u32,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
        target: [u8; 3],
        tol: i16,
    ) -> Option<f64> {
        let mut sum = 0u64;
        let mut n = 0u64;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 3) as usize;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                    sum += u64::from(y);
                    n += 1;
                }
            }
        }
        (n > 0).then(|| sum as f64 / n as f64)
    }

    #[test]
    fn clustered_bar_legend_key_is_a_filled_swatch_not_a_short_line() {
        // Excel 16.0 Chart.Export and PowerPoint Slide.Export both use a
        // filled swatch for clustered bars, just as for columns and areas.
        let color = [0x15, 0x60, 0x82];
        let mut actual = Canvas::new(24, 24);
        draw_plot_key(
            &mut actual,
            PlotKind::ClusteredBar,
            5,
            5,
            13,
            13,
            color,
            MarkerKind::None,
        );
        let mut expected = Canvas::new(24, 24);
        expected.fill_rect(5, 5, 13, 13, color);
        assert!(
            actual.px == expected.px,
            "bar legend keys must fill the swatch"
        );

        let ser = line_ser(0, "S1", &[Some(8.0), Some(1.0)], "");
        for orientation in ["maxMin", "minMax"] {
            let png = render_bar(&bar_xml(&ser, "", orientation, Some("bar"))).unwrap();
            let (w, h, rgb) = png_rgb(&png);
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for y in 0..h {
                for x in w - 55..w - 5 {
                    if pixel_at(&rgb, w, x, y) == color {
                        xs.push(x);
                        ys.push(y);
                    }
                }
            }
            assert!(!xs.is_empty(), "legend swatch must exist");
            let width = xs.iter().max().unwrap() - xs.iter().min().unwrap() + 1;
            let height = ys.iter().max().unwrap() - ys.iter().min().unwrap() + 1;
            assert_eq!((width, height), (8, 8), "filled bar swatch {orientation}");
            assert_eq!(xs.len(), 64, "no hollow or line-shaped key");
        }
    }

    #[test]
    fn clustered_bar_places_first_category_by_orientation() {
        let series = line_ser(0, "S1", &[Some(8.0), Some(1.0)], "");
        let a1 = FALLBACK_ACCENTS[0];
        let (w, h) = (576u32, 336u32);
        // 长条在右半边：maxMin 时第一类在上，minMax 时第一类在下。
        let top_xml = bar_xml(&series, "", "maxMin", Some("bar"));
        let (_, _, top_rgb) = png_rgb(&render_bar(&top_xml).unwrap());
        let top_right = count_in(&top_rgb, w, w / 2, w * 3 / 4, 20, h / 2, a1, 18);
        let bot_right = count_in(&top_rgb, w, w / 2, w * 3 / 4, h / 2, h - 20, a1, 18);
        assert!(
            top_right > bot_right,
            "maxMin should put the long first category in the top half; top={top_right} bottom={bot_right}"
        );

        let bot_xml = bar_xml(&series, "", "minMax", Some("bar"));
        let (_, _, bot_rgb) = png_rgb(&render_bar(&bot_xml).unwrap());
        let top_right = count_in(&bot_rgb, w, w / 2, w * 3 / 4, 20, h / 2, a1, 18);
        let bot_right = count_in(&bot_rgb, w, w / 2, w * 3 / 4, h / 2, h - 20, a1, 18);
        assert!(
            bot_right > top_right,
            "minMax should put the long first category in the bottom half; top={top_right} bottom={bot_right}"
        );
    }

    fn bar_dark_band(rgb: &[u8], w: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> u32 {
        let mut n = 0u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 3) as usize;
                if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn signed_bar_category_labels_follow_the_zero_crossing_axis() {
        let series = line_ser(0, "S1", &[Some(-8.0), Some(0.0), Some(8.0)], "");
        let (w, h, rgb) =
            png_rgb(&render_bar(&bar_xml(&series, "", "maxMin", Some("bar"))).unwrap());
        // Official signed-bar chart3 puts A/B/C immediately left of the
        // vertical zero axis. B must not remain stranded in the left inset.
        let left = bar_dark_band(&rgb, w, 5, 25, h / 2 - 10, h / 2 + 30);
        assert_eq!(
            left, 0,
            "category B must follow the zero axis, not the left gutter"
        );
        let at_origin = bar_dark_band(&rgb, w, w / 2 - 45, w / 2 - 15, h / 2 - 10, h / 2 + 30);
        assert!(
            at_origin > 0,
            "category label ink must exist by the zero axis"
        );
    }

    #[test]
    fn excel_chart_line_width_snaps_to_the_official_device_pen() {
        let px96 = RenderDpi::DEFAULT.pt_to_px_font(2.25);
        assert_eq!(px96, 3.0);
        assert_eq!(excel_chart_line_width_px(px96), 3.0);

        let dpi106 =
            RenderDpi::from_request(serde_json::json!({"dpi": 106}).as_object().unwrap()).unwrap();
        assert!((dpi106.pt_to_px_font(2.25) - 3.3125).abs() < 1e-9);
        assert_eq!(excel_chart_line_width_px(dpi106.pt_to_px_font(2.25)), 4.0);
        assert_eq!(excel_chart_line_width_px(dpi106.emu_to_px_f(28_575.0)), 4.0);

        let dpi144 =
            RenderDpi::from_request(serde_json::json!({"dpi": 144}).as_object().unwrap()).unwrap();
        assert!((dpi144.pt_to_px_font(2.25) - 4.5).abs() < 1e-9);
        assert_eq!(excel_chart_line_width_px(dpi144.pt_to_px_font(2.25)), 5.0);
    }

    #[test]
    fn snapped_device_pen_paints_an_aligned_opaque_core() {
        let color = [21, 96, 130];
        for (dpi_value, expect) in [(96u32, 3i32), (106, 4), (144, 5)] {
            let dpi = RenderDpi::from_request(
                serde_json::json!({ "dpi": dpi_value }).as_object().unwrap(),
            )
            .unwrap();
            let width = excel_chart_line_width_px(dpi.pt_to_px_font(2.25));
            assert_eq!(width, f64::from(expect));
            // Odd widths sit on a pixel centre; even widths sit on a pixel edge
            // so the opaque core is exactly `expect` device pixels.
            let y = if expect % 2 == 0 { 20.0 } else { 20.5 };
            let mut canvas = Canvas::with_dpi(80, 40, dpi);
            canvas.stroke_segment(10.0, y, 70.0, y, width, color);
            let x = 40u32;
            let mut run = 0i32;
            let mut best = 0i32;
            for row in 0..canvas.h {
                let i = ((row * canvas.w + x) * 3) as usize;
                let px = [canvas.px[i], canvas.px[i + 1], canvas.px[i + 2]];
                if px == color {
                    run += 1;
                    best = best.max(run);
                } else {
                    run = 0;
                }
            }
            assert_eq!(
                best, expect,
                "aligned 2.25pt core at dpi={dpi_value} should be {expect}px, got {best}"
            );
        }
    }

    #[test]
    fn line_legend_key_preserves_fractional_point_endpoints() {
        let mut canvas = Canvas::new(100, 40);
        canvas.dpi =
            RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap();
        let color = [21, 96, 130];
        for kind in [PlotKind::StandardLine, PlotKind::Scatter { connect: true }] {
            let mut actual = Canvas::new(100, 40);
            actual.dpi = canvas.dpi;
            draw_plot_key(&mut actual, kind, 20, 10, 49, 19, color, MarkerKind::None);
            // A 20pt centreline at 106dpi is 29.444px, not (rounded slot - 1).
            // The cap support must include the rightmost official sample pixel.
            let offset = ((14 * actual.w + 50) * 3) as usize;
            assert_ne!(&actual.px[offset..offset + 3], &[255, 255, 255]);
        }
    }

    #[test]
    fn line_and_scatter_keys_use_point_width_not_square_marker_width() {
        for dpi_value in [96, 106, 192] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi": dpi_value}).as_object().unwrap())
                    .unwrap();
            for marker_box in [7, 9, 14] {
                for kind in [
                    PlotKind::StandardLine,
                    PlotKind::Scatter { connect: true },
                    PlotKind::Scatter { connect: false },
                ] {
                    assert_eq!(
                        legend_key_width(kind, marker_box, dpi),
                        dpi.pt_to_px_font(20.0).round() as i32
                    );
                    assert_eq!(
                        legend_key_gap(kind, dpi),
                        dpi.pt_to_px_font(1.5).round() as i32
                    );
                }
                for kind in [
                    PlotKind::ClusteredBar,
                    PlotKind::ClusteredColumn,
                    PlotKind::StandardArea,
                    PlotKind::Pie,
                ] {
                    assert_eq!(legend_key_width(kind, marker_box, dpi), marker_box);
                }
            }
        }
    }

    #[test]
    fn clustered_bar_legend_follows_visual_series_order() {
        let series = format!(
            "{}{}",
            line_ser(0, "S1", &[Some(8.0), Some(1.0)], ""),
            line_ser(1, "S2", &[Some(2.0), Some(3.0)], "")
        );
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let (w, h) = (576u32, 336u32);
        // Restrict to the default right legend so plot bars cannot move the centroid.
        let x0 = 516;
        let x1 = 571;

        let (_, _, maxmin) =
            png_rgb(&render_bar(&bar_xml(&series, "", "maxMin", Some("bar"))).unwrap());
        let y1 = centroid_y(&maxmin, w, x0, x1, 0, h, a1, 2).expect("maxMin S1 key");
        let y2 = centroid_y(&maxmin, w, x0, x1, 0, h, a2, 2).expect("maxMin S2 key");
        assert!(
            y1 < y2,
            "maxMin must keep official S1-above-S2 legend order; y1={y1} y2={y2}"
        );

        let (_, _, minmax) =
            png_rgb(&render_bar(&bar_xml(&series, "", "minMax", Some("bar"))).unwrap());
        let y1 = centroid_y(&minmax, w, x0, x1, 0, h, a1, 2).expect("minMax S1 key");
        let y2 = centroid_y(&minmax, w, x0, x1, 0, h, a2, 2).expect("minMax S2 key");
        assert!(
            y2 < y1,
            "minMax must put official S2 above S1 in the legend; y1={y1} y2={y2}"
        );
    }

    #[test]
    fn clustered_bar_value_axis_follows_category_orientation() {
        let series = line_ser(0, "S1", &[Some(8.0), Some(1.0)], "");
        let (w, h) = (576u32, 336u32);
        let band = 36u32;
        let x0 = w / 5;
        let x1 = w * 3 / 4;

        let (_, _, top_rgb) =
            png_rgb(&render_bar(&bar_xml(&series, "", "maxMin", Some("bar"))).unwrap());
        let top = bar_dark_band(&top_rgb, w, x0, x1, 0, band);
        let bot = bar_dark_band(&top_rgb, w, x0, x1, h - band, h);
        assert!(
            top > bot * 3 && top > 80,
            "maxMin should put the value axis and tick labels at the top; top={top} bottom={bot}"
        );

        let (_, _, bot_rgb) =
            png_rgb(&render_bar(&bar_xml(&series, "", "minMax", Some("bar"))).unwrap());
        let top = bar_dark_band(&bot_rgb, w, x0, x1, 0, band);
        let bot = bar_dark_band(&bot_rgb, w, x0, x1, h - band, h);
        assert!(
            bot > top * 3 && bot > 80,
            "minMax should keep the value axis and tick labels at the bottom; top={top} bottom={bot}"
        );
    }

    #[test]
    fn clustered_bar_keeps_series_order_and_skips_missing_values() {
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let (w, h) = (576u32, 336u32);
        // 单类两系列：系列 0 在类目轴方向的槽起点（第一类在上时是槽顶）。
        let ordered = bar_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(8.0)], ""),
                line_ser(1, "S2", &[Some(8.0)], "")
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        let (_, _, rgb) = png_rgb(&render_bar(&ordered).unwrap());
        let y1 = centroid_y(&rgb, w, 80, 450, 20, 300, a1, 18).expect("series 0 color");
        let y2 = centroid_y(&rgb, w, 80, 450, 20, 300, a2, 18).expect("series 1 color");
        assert!(
            y1 < y2,
            "series 0 should sit above series 1 when the category starts at the top; y1={y1} y2={y2}"
        );

        // minMax：类目轴从下往上，槽起点在底部，系列 0 应在系列 1 下方。
        let ordered_minmax = bar_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(8.0)], ""),
                line_ser(1, "S2", &[Some(8.0)], "")
            ),
            "",
            "minMax",
            Some("bar"),
        );
        let (_, _, rgb_min) = png_rgb(&render_bar(&ordered_minmax).unwrap());
        let y1_min =
            centroid_y(&rgb_min, w, 80, 450, 20, 300, a1, 18).expect("series 0 color minMax");
        let y2_min =
            centroid_y(&rgb_min, w, 80, 450, 20, 300, a2, 18).expect("series 1 color minMax");
        assert!(
            y1_min > y2_min,
            "series 0 should sit below series 1 when the category starts at the bottom; y1={y1_min} y2={y2_min}"
        );

        // 缺值不把后续条挤进空位：S1 只在第一类，S2 只在第二类。
        let missing = bar_xml(
            &format!(
                "{}{}",
                line_ser(0, "S1", &[Some(8.0), None], ""),
                line_ser(1, "S2", &[None, Some(8.0)], "")
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        let (_, _, miss) = png_rgb(&render_bar(&missing).unwrap());
        let s1_top = count_in(&miss, w, 80, 450, 20, h / 2, a1, 18);
        let s1_bot = count_in(&miss, w, 80, 450, h / 2, 300, a1, 18);
        let s2_top = count_in(&miss, w, 80, 450, 20, h / 2, a2, 18);
        let s2_bot = count_in(&miss, w, 80, 450, h / 2, 300, a2, 18);
        assert!(
            s1_top > s1_bot && s2_bot > s2_top,
            "missing values should leave the other series in its own category; s1={s1_top}/{s1_bot} s2={s2_top}/{s2_bot}"
        );
    }

    #[test]
    fn clustered_bar_grows_from_zero_and_differs_from_columns() {
        let a1 = FALLBACK_ACCENTS[0];
        let (w, h) = (576u32, 336u32);
        let signed = bar_xml(
            &line_ser(
                0,
                "S1",
                &[Some(-8.0), Some(8.0)],
                "<c:invertIfNegative val=\"0\"/>",
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        let png = render_bar(&signed).unwrap();
        let (_, _, rgb) = png_rgb(&png);
        let top_left = count_in(&rgb, w, 40, w / 2, 20, h / 2, a1, 18);
        let top_right = count_in(&rgb, w, w / 2, 500, 20, h / 2, a1, 18);
        let bot_left = count_in(&rgb, w, 40, w / 2, h / 2, 300, a1, 18);
        let bot_right = count_in(&rgb, w, w / 2, 500, h / 2, 300, a1, 18);
        assert!(
            top_left > top_right,
            "negative first category should grow left in the top half; left={top_left} right={top_right}"
        );
        assert!(
            bot_right > bot_left,
            "positive second category should grow right in the bottom half; left={bot_left} right={bot_right}"
        );

        let maxed = bar_xml(
            &line_ser(0, "S1", &[Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        );
        let (_, _, max_rgb) = png_rgb(&render_bar(&maxed).unwrap());
        let near_right = count_in(&max_rgb, w, 400, 520, 20, 300, a1, 18);
        assert!(
            near_right > 0,
            "a maximum positive bar should reach the right of the value axis; count={near_right}"
        );

        // 同一组数据的柱形图是竖向的，像素不应与条形图相同。
        let col_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:varyColors val="0"/>
"#
        .to_string()
            + &line_ser(0, "S1", &[Some(8.0)], "")
            + r#"
        <c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="b"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="l"/><c:majorGridlines/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#;
        let col_png = render_bar(&col_xml).unwrap();
        assert_ne!(
            png, col_png,
            "clustered bars and clustered columns should not paint the same pixels"
        );
        let (_, _, col_rgb) = png_rgb(&col_png);
        let col_near_right = count_in(&col_rgb, w, 400, 520, 20, 300, a1, 18);
        assert!(
            near_right > col_near_right,
            "a horizontal max bar should occupy the right value edge more than a vertical column; bar={near_right} col={col_near_right}"
        );

        // 省略 barDir 时 OOXML 缺省为 bar。
        render_bar(&bar_xml(
            &line_ser(0, "S1", &[Some(4.0)], ""),
            "",
            "maxMin",
            None,
        ))
        .unwrap();
    }

    fn near_ink(rgb: [u8; 3]) -> bool {
        rgb[0] < 40 && rgb[1] < 40 && rgb[2] < 40
    }

    fn pixel_rgb(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * w + x) * 3) as usize;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }

    fn fill_x_span_limit(
        rgb: &[u8],
        w: u32,
        h: u32,
        color: [u8; 3],
        tol: i16,
        x_limit: u32,
    ) -> Option<(u32, u32)> {
        let mut min_x = u32::MAX;
        let mut max_x = 0u32;
        let mut found = false;
        let x_end = x_limit.min(w);
        for y in 0..h {
            for x in 0..x_end {
                if near(pixel_rgb(rgb, w, x, y), color, tol) {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    found = true;
                }
            }
        }
        found.then_some((min_x, max_x))
    }

    /// Long vertical ink columns in the plot, i.e. major grid / zero / max ticks.
    fn major_grid_xs(rgb: &[u8], w: u32, y0: u32, y1: u32, min_count: u32) -> Vec<u32> {
        let mut xs = Vec::new();
        for x in 0..w {
            let mut n = 0u32;
            for y in y0..y1 {
                if near_ink(pixel_rgb(rgb, w, x, y)) {
                    n += 1;
                }
            }
            if n >= min_count {
                xs.push(x);
            }
        }
        xs
    }

    #[test]
    fn clustered_bar_value_edge_lands_on_its_tick() {
        let color = FALLBACK_ACCENTS[0];
        let png = render_bar(&bar_xml(
            &line_ser(0, "S1", &[Some(8.0)], ""),
            "",
            "maxMin",
            Some("bar"),
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        // Skip the top value-axis band and the bottom gutter; grids run through the plot.
        let grids = major_grid_xs(&rgb, w, 50, 300, 80);
        assert_eq!(
            grids.len(),
            6,
            "t196u looser auto 0..10/2 should draw 6 vertical grids; grids={grids:?}"
        );
        let zero = *grids.first().expect("zero / min tick");
        let max_tick = *grids.last().expect("max tick");
        // Auto 0..10 step 2, value 8 is 8/10 of the axis and still a major tick.
        let expected = zero + ((max_tick - zero) * 8 + 5) / 10;
        let tick8 = grids
            .iter()
            .copied()
            .min_by_key(|x| x.abs_diff(expected))
            .unwrap();
        // Exclude the right-side legend swatch, which uses the same accent color.
        let (x0, x1) =
            fill_x_span_limit(&rgb, w, h, color, 8, max_tick + 1).expect("value-8 bar fill");
        assert_eq!(
            x1, tick8,
            "value-8 bar right edge must land on the 8 tick (no 1px gap); fill={x0}..{x1} zero={zero} max={max_tick} tick8={tick8} grids={grids:?}"
        );
        assert!(
            x0 == zero || x0 == zero + 1,
            "positive bar should start at the zero axis (axis ink may cover the first pixel); fill={x0}..{x1} zero={zero}"
        );
    }

    #[test]
    fn clustered_bar_signed_auto_major_uses_step_two() {
        // Office auto-scales -8..8 to -10..10 with majorUnit=2. This must not be
        // hardcoded for one fixture; the same path serves any clustered bar.
        let color = FALLBACK_ACCENTS[0];
        let png = render_bar(&bar_xml(
            &line_ser(
                0,
                "S1",
                &[Some(-8.0), Some(0.0), Some(8.0)],
                r#"<c:invertIfNegative val="0"/>"#,
            ),
            "",
            "maxMin",
            Some("bar"),
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let grids = major_grid_xs(&rgb, w, 50, 300, 80);
        assert_eq!(
            grids.len(),
            11,
            "auto major 2 on -10..10 should draw 11 vertical grid lines; grids={grids:?}"
        );
        let min_tick = grids[0];
        let max_tick = grids[10];
        let span = max_tick - min_tick;
        for (i, x) in grids.iter().copied().enumerate() {
            let expect = min_tick + ((span * i as u32) + 5) / 10;
            assert!(
                x.abs_diff(expect) <= 1,
                "grid {i} should be evenly spaced for step 2; x={x} expect={expect} grids={grids:?}"
            );
        }
        // Value 8 is the 9th tick (index 9: -10,-8,...,8,10). The bottom category
        // (C=+8, maxMin so first category on top) must meet that tick.
        let tick8 = grids[9];
        let tick0 = grids[5];
        // Restrict the fill hunt to the lower half so A=-8 does not mix in,
        // and stop before the legend swatch.
        let mut right = None;
        let mut left_pos = None;
        for y in (h / 2)..(h.saturating_sub(20)) {
            for x in 0..(max_tick + 1) {
                if near(pixel_rgb(&rgb, w, x, y), color, 8) {
                    left_pos = Some(left_pos.map_or(x, |cur: u32| cur.min(x)));
                    right = Some(right.map_or(x, |cur: u32| cur.max(x)));
                }
            }
        }
        let right = right.expect("positive C bar fill");
        let left_pos = left_pos.expect("positive C bar left");
        assert_eq!(
            right, tick8,
            "C=+8 bar right edge must land on the 8 tick; fill={left_pos}..{right} tick8={tick8} zero={tick0} grids={grids:?}"
        );
        assert!(
            left_pos.abs_diff(tick0) <= 1,
            "C bar should grow from the zero axis; left={left_pos} zero={tick0}"
        );
    }

    #[test]
    fn clustered_bar_inend_and_general_decimals_export() {
        let inend = bar_xml(
            &line_ser(
                0,
                "S1",
                &[Some(4.0), Some(6.0), Some(9.0)],
                r#"<c:dLbls><c:dLblPos val="inEnd"/><c:showVal val="1"/></c:dLbls>"#,
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        render_bar(&inend).expect("bar inEnd General integers should export");

        let decimal = bar_xml(
            &line_ser(
                0,
                "S1",
                &[Some(4.5), Some(6.3), Some(9.1999999999999993)],
                r#"<c:dLbls><c:dLblPos val="outEnd"/><c:showVal val="1"/></c:dLbls>"#,
            ),
            "",
            "maxMin",
            Some("bar"),
        );
        render_bar(&decimal).expect("bar General decimals should export");
    }

    #[test]
    fn clustered_bar_unsupported_inputs_stay_not_implemented() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(2.0)], "");
        let two = format!(
            "{}{}",
            line_ser(0, "S1", &[Some(1.0), Some(2.0)], ""),
            line_ser(1, "S2", &[Some(3.0), Some(4.0)], "")
        );
        let base = bar_xml(&ser, "", "maxMin", Some("bar"));
        let cases = [
            bar_xml(&ser, "", "maxMin", Some("bar")).replacen(
                r#"grouping val="clustered""#,
                r#"grouping val="stacked""#,
                1,
            ),
            bar_xml(&ser, "", "maxMin", Some("bar")).replacen(
                r#"grouping val="clustered""#,
                r#"grouping val="percentStacked""#,
                1,
            ),
            bar_xml(&two, "", "maxMin", Some("bar")).replacen(
                r#"varyColors val="0""#,
                r#"varyColors val="1""#,
                1,
            ),
            // 柱形轴位不能 silently 当条形画。
            bar_xml(&ser, "", "maxMin", Some("bar"))
                .replace(r#"<c:axPos val="l"/>"#, r#"<c:axPos val="b"/>"#),
            bar_xml(&ser, "", "maxMax", Some("bar")),
            base.replace("<c:catAx>", r#"<c:catAx><c:majorGridlines/>"#),
            base.replacen(r#"<c:delete val="0"/>"#, r#"<c:delete val="1"/>"#, 1),
            base.replace(
                "<c:catAx>",
                r#"<c:catAx><c:scaling><c:logBase val="10"/></c:scaling>"#,
            )
            .replacen(
                r#"<c:scaling><c:orientation val="maxMin"/></c:scaling>"#,
                "",
                1,
            ),
            base.replace("</c:plotArea>", "<c:dateAx/></c:plotArea>"),
            base.replace("<c:barChart>", "<c:bar3DChart>")
                .replace("</c:barChart>", "</c:bar3DChart>"),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err =
                render_bar(xml).expect_err(&format!("bar case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "bar case {i} should stay NotImplemented"
            );
        }
        // F13: unlabeled clustered bar dispBlanksAs=zero uses the same
        // general rule as official column Chart.Export. A complete numeric
        // cache is inert and must export.
        render_bar(&bar_xml(
            &ser,
            r#"<c:dispBlanksAs val="zero"/>"#,
            "maxMin",
            Some("bar"),
        ))
        .expect("bar dispBlanksAs=zero with a complete numeric cache should export");
        // F14: unlabeled clustered bar dispBlanksAs=span uses the same
        // general rule as official column Chart.Export (do not synthesize
        // a mid value). A complete numeric cache is inert and must export.
        render_bar(&bar_xml(
            &ser,
            r#"<c:dispBlanksAs val="span"/>"#,
            "maxMin",
            Some("bar"),
        ))
        .expect("bar dispBlanksAs=span with a complete numeric cache should export");
        // 隐藏数据标签与空值缺口仍可画；gap 是缺省。
        render_bar(&bar_xml(
            &ser,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "maxMin",
            Some("bar"),
        )
        .replacen(
            r#"</c:barChart>"#,
            r#"<c:dLbls><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showLegendKey val="0"/><c:showBubbleSize val="0"/></c:dLbls></c:barChart>"#,
            1,
        ))
        .unwrap();
        // 已支持的簇状柱仍可画，不因条形检查被误伤。
        let col_ok = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
"#
        .to_string()
            + &ser
            + r#"
      </c:barChart>
      <c:valAx><c:majorGridlines/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;
        render_bar(&col_ok).unwrap();
    }

    #[test]
    fn column_and_bar_disp_blanks_as_zero_fill_true_blanks_and_skip_na() {
        // Official F13 Chart.Export of clustered-column dispBlanksAs=zero
        // exists. True blanks become 0 (same general rule as lines). A
        // 0-height mark is not painted. #N/A is not a blank and stays
        // None / unpainted. Other errors stay NI. Labeled blanks stay
        // rejected so a showVal cannot silently disappear. Documented
        // general rule, not a unique official-matrix lock.
        let omitted = line_ser(0, "S1", &[Some(-4.0), None, Some(-9.0)], "");
        let explicit = line_ser(0, "S1", &[Some(-4.0), Some(0.0), Some(-9.0)], "");
        let zero_extra = r#"<c:dispBlanksAs val="zero"/>"#;
        for dir in ["col", "bar"] {
            let omitted_xml = bar_xml(&omitted, zero_extra, "maxMin", Some(dir));
            let explicit_xml = bar_xml(&explicit, zero_extra, "maxMin", Some(dir));
            let doc = crate::xml::parse(omitted_xml.as_bytes()).unwrap();
            let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
                .unwrap();
            assert_eq!(
                chart.series[0].values,
                vec![Some(-4.0), Some(0.0), Some(-9.0)],
                "dispBlanksAs=zero must fill a true blank as 0 on {dir}"
            );
            let omitted_png = render_bar(&omitted_xml).unwrap();
            let explicit_png = render_bar(&explicit_xml).unwrap();
            assert_eq!(
                omitted_png, explicit_png,
                "unlabeled {dir} zero-filled blank must match an explicit 0"
            );
        }

        let na = line_ser(0, "S1", &[Some(-4.0), Some(0.0), Some(-9.0)], "").replace(
            r#"<c:pt idx="1"><c:v>0</c:v></c:pt>"#,
            r#"<c:pt idx="1"><c:v>#N/A</c:v></c:pt>"#,
        );
        let na_xml = bar_xml(&na, zero_extra, "maxMin", Some("col"));
        let na_doc = crate::xml::parse(na_xml.as_bytes()).unwrap();
        let na_chart =
            read_chart(&na_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(
            na_chart.series[0].values,
            vec![Some(-4.0), None, Some(-9.0)],
            "column #N/A must not be filled as 0 under dispBlanksAs=zero"
        );
        render_bar(&na_xml).expect("unlabeled column #N/A + zero must export");

        let div = na.replace("#N/A", "#DIV/0!");
        assert!(
            render_bar(&bar_xml(&div, zero_extra, "maxMin", Some("col"))).is_err(),
            "column #DIV/0! + dispBlanksAs=zero stays NotImplemented"
        );
        assert!(
            render_bar(&bar_xml(&div, zero_extra, "maxMin", Some("bar"))).is_err(),
            "bar #DIV/0! + dispBlanksAs=zero stays NotImplemented"
        );
    }

    #[test]
    fn column_and_bar_disp_blanks_as_span_skips_blanks_and_does_not_fill_zero() {
        // Official F14 Chart.Export of clustered-column dispBlanksAs=span
        // exists and has no mid column. Span does not synthesize a mid
        // value: omitted blanks stay None (not filled as 0, unlike zero
        // mode) and are skipped. Unlabeled official gold cannot uniquely
        // distinguish that from gap. Documented general rule, not a unique
        // official-matrix lock. Labeled blanks stay rejected.
        let omitted = line_ser(0, "S1", &[Some(4.0), None, Some(9.0)], "");
        let span_extra = r#"<c:dispBlanksAs val="span"/>"#;
        let gap_extra = r#"<c:dispBlanksAs val="gap"/>"#;
        for dir in ["col", "bar"] {
            let span_xml = bar_xml(&omitted, span_extra, "maxMin", Some(dir));
            let gap_xml = bar_xml(&omitted, gap_extra, "maxMin", Some(dir));
            let doc = crate::xml::parse(span_xml.as_bytes()).unwrap();
            let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
                .unwrap();
            assert_eq!(
                chart.series[0].values,
                vec![Some(4.0), None, Some(9.0)],
                "dispBlanksAs=span must not fill a true blank as 0 on {dir}"
            );
            let span_png = render_bar(&span_xml).unwrap();
            let gap_png = render_bar(&gap_xml).unwrap();
            assert_eq!(
                span_png, gap_png,
                "unlabeled {dir} span must skip the blank like gap"
            );
        }

        let na = line_ser(0, "S1", &[Some(4.0), Some(0.0), Some(9.0)], "").replace(
            r#"<c:pt idx="1"><c:v>0</c:v></c:pt>"#,
            r#"<c:pt idx="1"><c:v>#N/A</c:v></c:pt>"#,
        );
        let na_xml = bar_xml(&na, span_extra, "maxMin", Some("col"));
        let na_doc = crate::xml::parse(na_xml.as_bytes()).unwrap();
        let na_chart =
            read_chart(&na_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(
            na_chart.series[0].values,
            vec![Some(4.0), None, Some(9.0)],
            "column #N/A must not be filled as 0 under dispBlanksAs=span"
        );
        render_bar(&na_xml).expect("unlabeled column #N/A + span must export");
    }

    fn area_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
      <c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn standard_area_xml(series: &str, chart_extra: &str) -> String {
        area_space(
            &format!(
                r#"<c:areaChart>
        <c:grouping val="standard"/>
        <c:varyColors val="0"/>
        {series}
        <c:axId val="1"/><c:axId val="2"/>
      </c:areaChart>"#
            ),
            chart_extra,
        )
    }

    fn area_fill(hex: &str) -> String {
        format!(r#"<c:spPr><a:solidFill><a:srgbClr val="{hex}"/></a:solidFill></c:spPr>"#)
    }

    fn render_area(xml: &str) -> Result<Vec<u8>, Error> {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
    }

    fn pixel_at(rgb: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * w + x) * 3) as usize;
        [rgb[i], rgb[i + 1], rgb[i + 2]]
    }

    #[test]
    fn standard_area_fills_to_zero_covers_later_series_and_breaks_on_gaps() {
        let s1 = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(2.0), Some(2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let s2 = line_ser(
            1,
            "S2",
            &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)],
            &area_fill("E97132"),
        );
        let png = render_area(&standard_area_xml(&format!("{s1}{s2}"), "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = [0x15, 0x60, 0x82];
        let a2 = [0xE9, 0x71, 0x32];
        assert!(
            count_in(&rgb, w, 80, w - 80, 40, h - 40, a1, 2) >= 80,
            "the lower series should remain visible above the covering series"
        );
        assert!(
            count_in(&rgb, w, 80, w - 80, 40, h - 40, a2, 2) >= 80,
            "the later series should cover the first near the baseline"
        );

        // Gap in the middle must not be filled across.
        let gapped = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(2.0), None, Some(2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let png = render_area(&standard_area_xml(
            &gapped,
            r#"<c:dispBlanksAs val="gap"/>"#,
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert!(count_in(&rgb, w, 80, w - 80, 40, h - 40, a1, 2) >= 40);
        // Mid-category of the missing point should stay near the plot background.
        let cx = w / 2;
        let mut mid_hits = 0usize;
        for y in 40..h - 50 {
            if near(pixel_at(&rgb, w, cx, y), a1, 8) {
                mid_hits += 1;
            }
        }
        assert!(
            mid_hits < 8,
            "a gap must not leave a filled band down the missing category, got {mid_hits}"
        );

        // Official Excel 16.0: omitted dispBlanksAs plots the missing area
        // point as 0 (a V to the baseline), not a hole. Explicit 0 matches.
        let as_zero = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(2.0), Some(0.0), Some(2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let omitted_png = render_area(&standard_area_xml(&gapped, "")).unwrap();
        let zero_png = render_area(&standard_area_xml(&as_zero, "")).unwrap();
        assert_eq!(
            omitted_png, zero_png,
            "omitted dispBlanksAs plus a missing area point must fill like an explicit 0"
        );
        assert_ne!(
            omitted_png, png,
            "explicit dispBlanksAs=gap must still leave a hole, unlike the omitted default"
        );
        let explicit_zero_png = render_area(&standard_area_xml(
            &gapped,
            r#"<c:dispBlanksAs val="zero"/>"#,
        ))
        .unwrap();
        assert_eq!(
            explicit_zero_png, omitted_png,
            "explicit area zero must fill the same V-to-baseline as omitted default"
        );
        let na = gapped.replace("<c:v>2</c:v>", "<c:v>#N/A</c:v>");
        let div = gapped.replace("<c:v>2</c:v>", "<c:v>#DIV/0!</c:v>");
        for body in [&na, &div] {
            assert!(
                render_area(&standard_area_xml(body, r#"<c:dispBlanksAs val="zero"/>"#)).is_err(),
                "unverified area error points must not silently become baseline zero"
            );
        }
        let explicit_span_png = render_area(&standard_area_xml(
            &gapped,
            r#"<c:dispBlanksAs val="span"/>"#,
        ))
        .unwrap();
        assert_eq!(
            explicit_span_png, omitted_png,
            "explicit area span must fill the same V-to-baseline as omitted default"
        );
        // Mid-only #N/A / #DIV/0! — do not reuse the all-points error XML
        // used for the explicit-zero NI asserts above.
        let na_mid = gapped.replace(
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt>"#,
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>#N/A</c:v></c:pt>"#,
        );
        let div_mid = gapped.replace(
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt>"#,
            r#"<c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>#DIV/0!</c:v></c:pt>"#,
        );
        assert!(
            render_area(&standard_area_xml(
                &na_mid,
                r#"<c:dispBlanksAs val="span"/>"#
            ))
            .is_ok(),
            "official area span paints #N/A as the same baseline V"
        );
        let span_na_png = render_area(&standard_area_xml(
            &na_mid,
            r#"<c:dispBlanksAs val="span"/>"#,
        ))
        .unwrap();
        assert_eq!(
            span_na_png, explicit_span_png,
            "area span + #N/A must match the blank-mid span fill"
        );
        assert!(
            render_area(&standard_area_xml(
                &div_mid,
                r#"<c:dispBlanksAs val="span"/>"#
            ))
            .is_err(),
            "unverified area #DIV/0! under span must stay not implemented"
        );
    }

    fn count_fill_band(rgb: &[u8], w: u32, x: u32, y0: u32, y1: u32, color: [u8; 3]) -> usize {
        (y0..y1)
            .filter(|&y| near(pixel_at(rgb, w, x, y), color, 8))
            .count()
    }

    #[test]
    fn standard_area_crosses_zero_and_slide_host_paints_fill() {
        let ser = line_ser(
            0,
            "S1",
            &[Some(2.0), Some(-2.0), Some(2.0)],
            &area_fill("156082"),
        );
        let png = render_area(&standard_area_xml(&ser, "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a1 = [0x15, 0x60, 0x82];
        // With +/- values the auto axis includes zero near mid-height.
        // First category (near w/4) is positive: fill sits in the upper half.
        // Second category (near w/2) is negative: fill sits in the lower half.
        let left = w / 4;
        let mid = w / 2;
        let pos_above = count_fill_band(&rgb, w, left, 40, h / 2, a1);
        let pos_below = count_fill_band(&rgb, w, left, h / 2, h - 40, a1);
        let neg_above = count_fill_band(&rgb, w, mid, 40, h / 2, a1);
        let neg_below = count_fill_band(&rgb, w, mid, h / 2, h - 40, a1);
        assert!(
            pos_above > 20 && pos_above > pos_below * 3,
            "positive values should fill above the zero crossing; above={pos_above} below={pos_below}"
        );
        assert!(
            neg_below > 20 && neg_below > neg_above * 3,
            "negative values should fill below zero, not above; below={neg_below} above={neg_above}"
        );

        let doc = crate::xml::parse(standard_area_xml(&ser, "").as_bytes()).unwrap();
        let slide = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Slide,
            None,
            "",
        )
        .unwrap();
        let (sw, _, srgb) = png_rgb(&slide);
        assert_eq!(sw, 576);
        assert!(
            count_in(&srgb, sw, 80, sw - 80, 40, 300, a1, 2) >= 80,
            "slide host shares the area painter"
        );
    }

    #[test]
    fn unsupported_area_inputs_stay_not_implemented() {
        let ser = line_ser(
            0,
            "S1",
            &[Some(1.0), Some(2.0), Some(3.0)],
            &area_fill("156082"),
        );
        let base = standard_area_xml(&ser, "");
        let cases = [
            base.replace(r#"grouping val="standard""#, r#"grouping val="stacked""#),
            base.replace(
                r#"grouping val="standard""#,
                r#"grouping val="percentStacked""#,
            ),
            base.replacen(r#"varyColors val="0""#, r#"varyColors val="1""#, 1),
            base.replacen(
                r#"</c:areaChart>"#,
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:areaChart>"#,
                1,
            ),
            base.replacen(r#"</c:areaChart>"#, r#"<c:dropLines/></c:areaChart>"#, 1),
            standard_area_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:spPr><a:solidFill><a:srgbClr val="156082"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr>"#,
                ),
                "",
            ),
            standard_area_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:marker><c:symbol val="circle"/><c:size val="5"/></c:marker>"#,
                ),
                "",
            ),
            area_space(
                r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser>
          <c:order val="0"/><c:val><c:numLit><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:barChart><c:areaChart><c:grouping val="standard"/><c:ser>
          <c:order val="1"/><c:val><c:numLit><c:pt idx="0"><c:v>2</c:v></c:pt></c:numLit></c:val>
        </c:ser></c:areaChart>"#,
                "",
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err =
                render_area(xml).expect_err(&format!("area case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "area case {i} should stay NotImplemented"
            );
        }
        // Official Excel 16 standard area writes midCat (first/last categories
        // on the plot edges). That encoding must export, not stay NI.
        render_area(
            &base.replace(
                r#"crossBetween val="between""#,
                r#"crossBetween val="midCat""#,
            ),
        )
        .expect("standard area midCat must export");
        // Omitted grouping, gap blanks, hidden labels, and marker none stay supported.
        render_area(
            &area_space(
                &format!(
                    r#"<c:areaChart>
        <c:varyColors val="0"/>
        {ser}
        <c:dLbls><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showLegendKey val="0"/><c:showBubbleSize val="0"/></c:dLbls>
        <c:axId val="1"/><c:axId val="2"/>
      </c:areaChart>"#
                ),
                r#"<c:dispBlanksAs val="gap"/>"#,
            )
            .replacen(
                &area_fill("156082"),
                &format!(
                    r#"{}<c:marker><c:symbol val="none"/></c:marker>"#,
                    area_fill("156082")
                ),
                1,
            ),
        )
        .unwrap();
        // A supported line is not broken by the new arm.
        render_line(&standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "",
        ))
        .unwrap();
    }

    fn pie_space(plot_body: &str, chart_extra: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {chart_extra}
    <c:plotArea>
      {plot_body}
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn standard_pie_xml(series: &str, chart_extra: &str, extra: &str) -> String {
        pie_space(
            &format!(
                r#"<c:pieChart>
        <c:varyColors val="1"/>
        {series}
        {extra}
      </c:pieChart>"#
            ),
            chart_extra,
        )
    }

    fn render_pie(xml: &str) -> Result<Vec<u8>, Error> {
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
    }

    fn color_centroid(
        rgb: &[u8],
        w: u32,
        h: u32,
        target: [u8; 3],
        tol: i16,
    ) -> Option<(f64, f64, usize)> {
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut n = 0usize;
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], target, tol) {
                    sx += f64::from(x) + 0.5;
                    sy += f64::from(y) + 0.5;
                    n += 1;
                }
            }
        }
        (n > 0).then_some((sx / n as f64, sy / n as f64, n))
    }

    #[test]
    fn pie_opaque_plot_fill_paints_bounding_square_before_slices() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)], "");
        for dpi_value in [96, 106, 144] {
            let dpi =
                RenderDpi::from_request(serde_json::json!({"dpi":dpi_value}).as_object().unwrap())
                    .unwrap();
            let xml = standard_pie_xml(&ser, "", "").replace(r#"<c:legend><c:legendPos val="r"/></c:legend>"#, "").replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='FFFF00'/></a:solidFill></c:spPr>",
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let png = render_chart_at(
                &doc.root,
                576,
                336,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                dpi,
            )
            .unwrap();
            let (w, h, rgb) = decode_rgb(&png);
            let rect = pie_plot_bounds(576.0, 336.0, 0.0, 0.0, dpi).unwrap();
            let x = rect.x.ceil() as u32 + 2;
            let y = rect.y.ceil() as u32 + 2;
            let i = ((y * w + x) * 3) as usize;
            assert_eq!(
                &rgb[i..i + 3],
                &[255, 255, 0],
                "bounding-square corner, not just wedge"
            );
            let outside = ((y * w + x - 5) * 3) as usize;
            assert_eq!(
                &rgb[outside..outside + 3],
                &[255, 255, 255],
                "not full host fill"
            );
            for accent in &FALLBACK_ACCENTS[..4] {
                assert!(
                    color_centroid(&rgb, w, h, *accent, 0).unwrap().2 > 100,
                    "fill must stay below slices"
                );
            }
        }
        // Translucent and linear plot fills are painted on the same fractional
        // square as the opaque solid case. Diagonal/path fills use close-enough
        // geometry so they export; residuals stay on the review page.
        let grad = "<a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='FFFF00'/></a:gs><a:gs pos='100000'><a:srgbClr val='0000FF'/></a:gs></a:gsLst><a:lin ang='5400000' scaled='1'/><a:tileRect/></a:gradFill>";
        let xml = standard_pie_xml(&ser, "", "").replace(
            r#"<c:legend><c:legendPos val="r"/></c:legend>"#,
            "",
        ).replace("<c:plotArea>", &format!("<c:plotArea><c:spPr>{grad}</c:spPr>"));
        let png = render_pie(&xml).expect("locked vertical pie plot gradient must export");
        let (w, _h, rgb) = decode_rgb(&png);
        let rect = pie_plot_bounds(576.0, 336.0, 0.0, 0.0, RenderDpi::DEFAULT).unwrap();
        // Same inset as the opaque solid corner: mid-top sits inside a slice.
        let x = rect.x.ceil() as u32 + 2;
        let y_top = rect.y.ceil() as u32 + 2;
        let y_bot = (rect.y + rect.h).floor() as u32 - 2;
        let top = sample_rgb(&rgb, w, x, y_top);
        let bot = sample_rgb(&rgb, w, x, y_bot);
        assert!(top[0] > 200 && top[2] < 80, "top of plot square should be yellow, got {top:?}");
        assert!(bot[2] > 200 && bot[0] < 80, "bottom of plot square should be blue, got {bot:?}");
        let alpha = "<a:solidFill><a:srgbClr val='FFFF00'><a:alpha val='50000'/></a:srgbClr></a:solidFill>";
        let xml = standard_pie_xml(&ser, "", "").replace(
            r#"<c:legend><c:legendPos val="r"/></c:legend>"#,
            "",
        ).replace("<c:plotArea>", &format!("<c:plotArea><c:spPr>{alpha}</c:spPr>"));
        let png = render_pie(&xml).expect("locked translucent pie plot fill must export");
        let (_, _, rgb) = decode_rgb(&png);
        let corner = sample_rgb(&rgb, w, (rect.x + 2.0) as u32, (rect.y + 2.0) as u32);
        // Straight 50% yellow over the default white chart area.
        assert!(
            corner[0] > 200 && corner[1] > 200 && corner[2] < 160,
            "translucent yellow plot corner, got {corner:?}"
        );
        let diag = "<a:gradFill flip='none' rotWithShape='1'><a:gsLst><a:gs pos='0'><a:srgbClr val='FFFF00'/></a:gs><a:gs pos='100000'><a:srgbClr val='0000FF'/></a:gs></a:gsLst><a:lin ang='2700000' scaled='1'/><a:tileRect/></a:gradFill>";
        let xml = standard_pie_xml(&ser, "", "").replace("<c:plotArea>", &format!("<c:plotArea><c:spPr>{diag}</c:spPr>"));
        render_pie(&xml).expect("close-enough diagonal pie plot fill must export");
    }

    #[test]
    fn standard_pie_assigns_point_colors_clockwise_from_up() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(&ser, "", "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let a4 = FALLBACK_ACCENTS[3];
        let c1 = color_centroid(&rgb, w, h, a1, 2).expect("first slice color");
        let c2 = color_centroid(&rgb, w, h, a2, 2).expect("second slice color");
        let c3 = color_centroid(&rgb, w, h, a3, 2).expect("third slice color");
        let c4 = color_centroid(&rgb, w, h, a4, 2).expect("fourth slice color");
        let cx = (c1.0 + c2.0 + c3.0 + c4.0) / 4.0;
        let cy = (c1.1 + c2.1 + c3.1 + c4.1) / 4.0;
        // Clockwise from 12 o'clock: accent1 upper-right, 2 lower-right, 3 lower-left, 4 upper-left.
        assert!(
            c1.0 > cx && c1.1 < cy,
            "first slice should sit in the upper-right; c1={c1:?} center=({cx},{cy})"
        );
        assert!(
            c2.0 > cx && c2.1 > cy,
            "second slice should sit in the lower-right; c2={c2:?}"
        );
        assert!(
            c3.0 < cx && c3.1 > cy,
            "third slice should sit in the lower-left; c3={c3:?}"
        );
        assert!(
            c4.0 < cx && c4.1 < cy,
            "fourth slice should sit in the upper-left; c4={c4:?}"
        );
        for (_, _, n) in [c1, c2, c3, c4] {
            assert!(
                n >= 80,
                "each equal slice should cover a real area, got {n}"
            );
        }
    }

    #[test]
    fn pie_first_slice_angle_rotates_and_dpt_overrides_color() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(1.0), Some(1.0), Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(
            &ser,
            "",
            r#"<c:firstSliceAng val="90"/>"#,
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let a4 = FALLBACK_ACCENTS[3];
        let c1 = color_centroid(&rgb, w, h, a1, 2).unwrap();
        let c2 = color_centroid(&rgb, w, h, a2, 2).unwrap();
        let c3 = color_centroid(&rgb, w, h, a3, 2).unwrap();
        let c4 = color_centroid(&rgb, w, h, a4, 2).unwrap();
        let cx = (c1.0 + c2.0 + c3.0 + c4.0) / 4.0;
        let cy = (c1.1 + c2.1 + c3.1 + c4.1) / 4.0;
        // Starting at 3 o'clock, first slice fills the lower-right quadrant.
        assert!(
            c1.0 > cx && c1.1 > cy,
            "firstSliceAng=90 should move the first slice into the lower-right; c1={c1:?} center=({cx},{cy})"
        );
        assert!(
            c2.0 < cx && c2.1 > cy,
            "second slice should sit in the lower-left; c2={c2:?}"
        );

        let override_ser = line_ser(
            0,
            "S1",
            &[Some(1.0), Some(1.0)],
            r#"<c:dPt><c:idx val="0"/><c:spPr><a:solidFill><a:srgbClr val="CC0000"/></a:solidFill></c:spPr></c:dPt>"#,
        );
        let png = render_pie(&standard_pie_xml(&override_ser, "", "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let red = [0xCC, 0x00, 0x00];
        let hits = color_centroid(&rgb, w, h, red, 2).expect("dPt solid fill");
        assert!(
            hits.2 >= 80,
            "dPt should paint the first slice, got {}",
            hits.2
        );
        // The second point still uses accent2, not accent1.
        let a2 = FALLBACK_ACCENTS[1];
        assert!(color_centroid(&rgb, w, h, a2, 2).is_some());
    }

    #[test]
    fn pie_skips_blank_and_zero_slices_but_keeps_both_in_the_legend() {
        let gapped = line_ser(0, "S1", &[Some(1.0), None, Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(
            &gapped,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let c1 = color_centroid(&rgb, w, h, a1, 2).unwrap();
        let c3 = color_centroid(&rgb, w, h, a3, 2).unwrap();
        // Blank point is skipped as a slice, but official Excel still shows it in the legend.
        assert!(c1.2 >= 80 && c3.2 >= 80);
        assert!(
            (c1.0 - c3.0).abs() > 20.0,
            "the two remaining slices should sit on opposite sides"
        );
        let blank_hits = color_centroid(&rgb, w, h, a2, 2).map(|c| c.2).unwrap_or(0);
        assert!(
            blank_hits > 0 && blank_hits < 80,
            "a blank should appear in the legend, not as a slice; hits={blank_hits}"
        );

        let with_zero = line_ser(0, "S1", &[Some(1.0), Some(0.0), Some(1.0)], "");
        let png = render_pie(&standard_pie_xml(&with_zero, "", "")).unwrap();
        let (w, h, rgb) = png_rgb(&png);
        // Zero has no wedge, but the category remains in the legend swatch.
        let zero_hits = color_centroid(&rgb, w, h, a2, 2).map(|c| c.2).unwrap_or(0);
        assert!(
            zero_hits > 0 && zero_hits < 80,
            "a zero should appear in the legend, not as a slice; hits={zero_hits}"
        );

        let doc = crate::xml::parse(standard_pie_xml(&gapped, "", "").as_bytes()).unwrap();
        let slide = render_rgb_chart(
            &doc.root,
            576,
            336,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Slide,
            None,
            "",
        )
        .unwrap();
        let (sw, _, srgb) = png_rgb(&slide);
        assert_eq!(sw, 576);
        assert!(color_centroid(&srgb, sw, 336, a1, 2).is_some());
    }

    #[test]
    fn pie_all_zero_all_empty_and_all_negative_follow_official_empty_or_abs_plot() {
        let all_neg = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(-10.0), Some(-20.0), Some(-30.0), Some(-40.0)], ""),
            "",
            "",
        ))
        .unwrap();
        let all_abs = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)], ""),
            "",
            "",
        ))
        .unwrap();
        assert_eq!(all_neg, all_abs, "all-negative pies should paint as |v|");

        let zero_neg = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(0.0), Some(-20.0), Some(0.0), Some(0.0)], ""),
            "",
            "",
        ))
        .unwrap();
        let zero_abs = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(0.0), Some(20.0), Some(0.0), Some(0.0)], ""),
            "",
            "",
        ))
        .unwrap();
        assert_eq!(
            zero_neg, zero_abs,
            "a lone finite negative among zeros should paint as |v|"
        );
        assert_ne!(zero_neg, all_abs, "zero-and-negative is not a four-slice abs pie");

        let all_zero = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[Some(0.0), Some(0.0), Some(0.0), Some(0.0)], ""),
            "",
            "",
        ))
        .unwrap();
        let all_empty_ser = r#"<c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>S1</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="4"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt><c:pt idx="2"><c:v>C</c:v></c:pt><c:pt idx="3"><c:v>D</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="4"/></c:numLit></c:val>
        </c:ser>"#;
        let all_empty = render_pie(&standard_pie_xml(
            all_empty_ser,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        assert_eq!(
            all_zero, all_empty,
            "official-like all-empty (ptCount, no val pts) should match all-zero"
        );
        // line_ser omits val pts and ptCount, but still writes category pts.
        // That is the same locked domain rule, not a second painter.
        let omitted_pts = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &[None, None, None, None], ""),
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        assert_eq!(
            all_zero, omitted_pts,
            "omitted val pts with category pts should match all-zero"
        );
        let (w, h, rgb) = png_rgb(&all_zero);
        assert_eq!((w, h), (576, 336));
        let a1 = FALLBACK_ACCENTS[0];
        let a2 = FALLBACK_ACCENTS[1];
        let a3 = FALLBACK_ACCENTS[2];
        let a4 = FALLBACK_ACCENTS[3];
        for (i, accent) in [a1, a2, a3, a4].iter().enumerate() {
            let hits = color_centroid(&rgb, w, h, *accent, 2).map(|c| c.2).unwrap_or(0);
            assert!(
                hits > 0 && hits < 80,
                "all-zero/empty should keep legend swatch {i}, not a slice; hits={hits}"
            );
        }
    }

    #[test]
    fn pie_finite_negatives_use_absolute_slice_weight_and_stay_in_legend() {
        let cases: &[(&[Option<f64>], &[Option<f64>])] = &[
            (
                &[Some(10.0), Some(-20.0), Some(30.0), Some(40.0)],
                &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
            ),
            (
                &[Some(-10.0), Some(20.0), Some(30.0), Some(40.0)],
                &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
            ),
            (
                &[Some(10.0), Some(-20.0), Some(-30.0), Some(40.0)],
                &[Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
            ),
            (
                &[Some(1.0), Some(-2.0), Some(3.0)],
                &[Some(1.0), Some(2.0), Some(3.0)],
            ),
        ];
        for (signed, absed) in cases {
            let signed_png = render_pie(&standard_pie_xml(
                &line_ser(0, "S1", signed, ""),
                "",
                "",
            ))
            .unwrap();
            let abs_png = render_pie(&standard_pie_xml(
                &line_ser(0, "S1", absed, ""),
                "",
                "",
            ))
            .unwrap();
            assert_eq!(
                signed_png, abs_png,
                "finite negatives should paint as |v|, signed={signed:?}"
            );
        }
        let mixed = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &[Some(10.0), Some(-20.0), Some(30.0), Some(40.0)],
                "",
            ),
            "",
            "",
        ))
        .unwrap();
        let skip_zero = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &[Some(10.0), Some(0.0), Some(30.0), Some(40.0)],
                "",
            ),
            "",
            "",
        ))
        .unwrap();
        assert_ne!(
            mixed, skip_zero,
            "finite negatives are not skipped as zero"
        );
    }

    #[test]
    fn pie_explosion_follows_official_series_shrink_and_point_bisector() {
        let vals = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let ctrl = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, ""),
            "",
            "",
        ))
        .unwrap();
        let exp0 = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, r#"<c:explosion val="0"/>"#),
            "",
            "",
        ))
        .unwrap();
        assert_eq!(ctrl, exp0, "explicit explosion 0 should match an omitted explosion");

        let ser10 = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, r#"<c:explosion val="10"/>"#),
            "",
            "",
        ))
        .unwrap();
        let ser25 = render_pie(&standard_pie_xml(
            &line_ser(0, "S1", &vals, r#"<c:explosion val="25"/>"#),
            "",
            "",
        ))
        .unwrap();
        assert_ne!(ser10, ctrl, "series explosion 10 should move the slices");
        assert_ne!(ser25, ser10, "series explosion 25 is not the same as 10");

        let (w, h, ctrl_rgb) = png_rgb(&ctrl);
        let (_, _, s10_rgb) = png_rgb(&ser10);
        let (_, _, s25_rgb) = png_rgb(&ser25);
        let accents = [
            FALLBACK_ACCENTS[0],
            FALLBACK_ACCENTS[1],
            FALLBACK_ACCENTS[2],
            FALLBACK_ACCENTS[3],
        ];
        let ctrl_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&ctrl_rgb, w, h, *c, 2).expect("ctrl slice"))
            .collect();
        let s10_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&s10_rgb, w, h, *c, 2).expect("ser10 slice"))
            .collect();
        let s25_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&s25_rgb, w, h, *c, 2).expect("ser25 slice"))
            .collect();
        let cx: f64 = ctrl_c.iter().map(|c| c.0).sum::<f64>() / 4.0;
        let cy: f64 = ctrl_c.iter().map(|c| c.1).sum::<f64>() / 4.0;
        for i in 0..4 {
            let r0 = ((ctrl_c[i].0 - cx).hypot(ctrl_c[i].1 - cy)).max(1.0);
            let r10 = (s10_c[i].0 - cx).hypot(s10_c[i].1 - cy);
            let r25 = (s25_c[i].0 - cx).hypot(s25_c[i].1 - cy);
            assert!(
                r10 > r0 && r25 > r10,
                "series explosion should move slice {i} outward; r0={r0:.2} r10={r10:.2} r25={r25:.2}"
            );
            assert!(
                s10_c[i].2 < ctrl_c[i].2 && s25_c[i].2 < s10_c[i].2,
                "series explosion should shrink slice {i} area; n0={} n10={} n25={}",
                ctrl_c[i].2,
                s10_c[i].2,
                s25_c[i].2
            );
        }

        let pt_a = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:dPt><c:idx val="0"/><c:bubble3D val="0"/><c:explosion val="20"/></c:dPt>"#,
            ),
            "",
            "",
        ))
        .unwrap();
        let pt_b = render_pie(&standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:dPt><c:idx val="1"/><c:bubble3D val="0"/><c:explosion val="20"/></c:dPt>"#,
            ),
            "",
            "",
        ))
        .unwrap();
        assert_ne!(pt_a, ctrl);
        assert_ne!(pt_b, ctrl);
        assert_ne!(pt_a, pt_b);
        let (_, _, a_rgb) = png_rgb(&pt_a);
        let (_, _, b_rgb) = png_rgb(&pt_b);
        let a_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&a_rgb, w, h, *c, 2).expect("ptA slice"))
            .collect();
        let b_c: Vec<_> = accents
            .iter()
            .map(|c| color_centroid(&b_rgb, w, h, *c, 2).expect("ptB slice"))
            .collect();
        let dx_a = a_c[0].0 - ctrl_c[0].0;
        let dy_a = a_c[0].1 - ctrl_c[0].1;
        let ang_a = dx_a.atan2(-dy_a).to_degrees();
        assert!(
            dy_a < -8.0 && ang_a > 10.0 && ang_a < 26.0,
            "point A explosion should move the first slice up along its ~18 deg bisector; d=({dx_a:.2},{dy_a:.2}) ang={ang_a:.1}"
        );
        for i in 1..4 {
            let d = (a_c[i].0 - ctrl_c[i].0).hypot(a_c[i].1 - ctrl_c[i].1);
            assert!(
                d < 2.0,
                "unexploded slices should stay put on a point-A explosion; slice {i} moved {d:.2}"
            );
        }
        let dx_b = b_c[1].0 - ctrl_c[1].0;
        let dy_b = b_c[1].1 - ctrl_c[1].1;
        let ang_b = dx_b.atan2(-dy_b).to_degrees();
        assert!(
            dx_b > 8.0 && ang_b > 64.0 && ang_b < 80.0,
            "point B explosion should move the second slice along its ~72 deg bisector; d=({dx_b:.2},{dy_b:.2}) ang={ang_b:.1}"
        );
        for i in [0, 2, 3] {
            let d = (b_c[i].0 - ctrl_c[i].0).hypot(b_c[i].1 - ctrl_c[i].1);
            assert!(
                d < 2.0,
                "unexploded slices should stay put on a point-B explosion; slice {i} moved {d:.2}"
            );
        }
    }

    #[test]
    fn pie_slice_fills_reuse_locked_solid_alpha_and_nofill() {
        let vals = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let with_plot = |xml: String| {
            xml.replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='FFFFCC'/></a:solidFill></c:spPr>",
            )
        };
        let src_over = |src: [u8; 3], dst: [u8; 3], cov: u8| -> [u8; 3] {
            let src_a = u16::from(cov);
            std::array::from_fn(|k| {
                ((u16::from(src[k]) * src_a + u16::from(dst[k]) * (255 - src_a) + 127) / 255) as u8
            })
        };
        let exact = |rgb: &[u8], w: u32, h: u32, color: [u8; 3]| {
            let mut n = 0usize;
            for i in 0..(w * h) as usize {
                if rgb[i * 3..i * 3 + 3] == color {
                    n += 1;
                }
            }
            n
        };

        let ser_solid = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr>"#,
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&ser_solid);
        assert!(
            exact(&rgb, w, h, [255, 0, 0]) >= 200,
            "series solidFill should paint every slice even when varyColors=1"
        );
        for accent in &FALLBACK_ACCENTS[..4] {
            assert!(
                exact(&rgb, w, h, *accent) < 40,
                "varyColors must not keep accent slices once the series has an explicit solid"
            );
        }

        let cov = ooxml_alpha_to_u8(50_000);
        assert_eq!(cov, 128);
        let plot = [255, 255, 204];
        let red_over_plot = src_over([255, 0, 0], plot, cov);
        assert_eq!(red_over_plot, [255, 127, 102]);
        let ser_alpha = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr>"#,
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&ser_alpha);
        assert!(
            exact(&rgb, w, h, red_over_plot) >= 200,
            "series alpha 50000 should use locked straight alpha over the plot fill"
        );
        assert_eq!(exact(&rgb, w, h, [255, 0, 0]), 0);

        let hidden = r#"<c:spPr><a:noFill/><a:extLst><a:ext uri="{909E8E84-426E-40DD-AFC4-6F175D3DCCD1}"><a14:hiddenFill xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"><a:solidFill><a:srgbClr val="156082"/></a:solidFill></a14:hiddenFill></a:ext></a:extLst></c:spPr>"#;
        let ser_nofill = render_pie(&with_plot(standard_pie_xml(
            &line_ser(0, "S1", &vals, hidden),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&ser_nofill);
        assert!(exact(&rgb, w, h, plot) >= 200, "series noFill shows the plot");
        assert_eq!(exact(&rgb, w, h, [255, 0, 0]), 0);
        assert_eq!(exact(&rgb, w, h, [0x15, 0x60, 0x82]), 0);
        for accent in &FALLBACK_ACCENTS[..4] {
            assert_eq!(
                exact(&rgb, w, h, *accent),
                0,
                "series noFill should not paint accent wedges or keys"
            );
        }

        let pt_nofill = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                &format!(r#"<c:dPt><c:idx val="0"/><c:bubble3D val="0"/>{hidden}</c:dPt>"#),
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&pt_nofill);
        assert_eq!(
            exact(&rgb, w, h, FALLBACK_ACCENTS[0]),
            0,
            "dPt noFill removes that slice and its legend key"
        );
        for accent in &FALLBACK_ACCENTS[1..4] {
            assert!(
                exact(&rgb, w, h, *accent) >= 80,
                "other slices stay on the default accent"
            );
        }

        let lime_over_plot = src_over([0, 255, 0], plot, cov);
        assert_eq!(lime_over_plot, [127, 255, 102]);
        let mixed = render_pie(&with_plot(standard_pie_xml(
            &line_ser(
                0,
                "S1",
                &vals,
                r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr>
          <c:dPt><c:idx val="1"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></c:spPr></c:dPt>
          <c:dPt><c:idx val="2"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="00FF00"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr></c:dPt>"#,
            ),
            "",
            "",
        )))
        .unwrap();
        let (w, h, rgb) = png_rgb(&mixed);
        assert!(exact(&rgb, w, h, [255, 0, 0]) >= 80, "unoverridden slices use the series solid");
        assert!(exact(&rgb, w, h, [0, 0, 255]) >= 80, "dPt solid overrides that slice");
        assert!(
            exact(&rgb, w, h, lime_over_plot) >= 80,
            "dPt alpha overrides that slice with locked straight alpha"
        );
    }

    #[test]
    fn unsupported_pie_inputs_stay_not_implemented() {
        let ser = line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], "");
        let base = standard_pie_xml(&ser, "", "");
        let two = format!(
            "{}{}",
            ser,
            line_ser(1, "S2", &[Some(1.0), Some(2.0), Some(3.0)], "")
        );
        let cases = [
            // Leftover: a val cache ptCount with no val pts and no category
            // pts is not sized from untrusted ptCount.
            standard_pie_xml(
                r#"<c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>S1</c:v></c:tx>
          <c:val><c:numLit><c:ptCount val="4"/></c:numLit></c:val>
        </c:ser>"#,
                r#"<c:dispBlanksAs val="gap"/>"#,
                "",
            ),
            pie_space(
                &format!(r#"<c:pieChart><c:varyColors val="1"/>{two}</c:pieChart>"#),
                "",
            ),
            standard_pie_xml(&ser, "", r#"<c:firstSliceAng val="361"/>"#),
            // Leftover: series explosion plus a dPt explosion is not uniquely locked.
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:explosion val="10"/><c:dPt><c:idx val="0"/><c:explosion val="20"/></c:dPt>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:dPt><c:idx val="0"/><c:bubble3D val="1"/><c:explosion val="20"/></c:dPt>"#,
                ),
                "",
                "",
            ),
            standard_pie_xml(
                &line_ser(
                    0,
                    "S1",
                    &[Some(1.0), Some(2.0)],
                    r#"<c:spPr><a:gradFill flip="none" rotWithShape="1"><a:gsLst><a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs></a:gsLst><a:lin ang="0" scaled="1"/><a:tileRect/></a:gradFill></c:spPr>"#,
                ),
                "",
                "",
            ),
            base.replacen(
                r#"</c:pieChart>"#,
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:pieChart>"#,
                1,
            ),
            pie_space(
                &format!(
                    r#"<c:pieChart><c:varyColors val="1"/>{ser}</c:pieChart>
       <c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>"#
                ),
                "",
            ),
            standard_pie_xml(&ser, "", "")
                .replace(r#"<c:legendPos val="r"/>"#, r#"<c:legendPos val="l"/>"#),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err =
                render_pie(xml).expect_err(&format!("pie case {i} should stay NotImplemented"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "pie case {i} should stay NotImplemented"
            );
        }
        // Omitted firstSliceAng and hidden labels stay supported; a column chart is unharmed.
        render_pie(
            &standard_pie_xml(&ser, r#"<c:dispBlanksAs val="gap"/>"#, "").replacen(
                r#"</c:pieChart>"#,
                r#"<c:dLbls><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showLegendKey val="0"/><c:showBubbleSize val="0"/></c:dLbls></c:pieChart>"#,
                1,
            ),
        )
        .unwrap();
        // Official F09 gold writes dispBlanksAs=zero. With a complete cache
        // zero and span are inert and must export the same image as gap.
        let gap = render_pie(&standard_pie_xml(
            &ser,
            r#"<c:dispBlanksAs val="gap"/>"#,
            "",
        ))
        .unwrap();
        let zero = render_pie(&standard_pie_xml(
            &ser,
            r#"<c:dispBlanksAs val="zero"/>"#,
            "",
        ))
        .unwrap();
        let span = render_pie(&standard_pie_xml(
            &ser,
            r#"<c:dispBlanksAs val="span"/>"#,
            "",
        ))
        .unwrap();
        assert_eq!(zero, gap, "complete-cache pie zero must match gap");
        assert_eq!(span, gap, "complete-cache pie span must match gap");
        let blank_ser = line_ser(0, "S1", &[Some(1.0), None, Some(3.0)], "");
        for mode in ["zero", "span"] {
            let xml = standard_pie_xml(
                &blank_ser,
                &format!(r#"<c:dispBlanksAs val="{mode}"/>"#),
                "",
            );
            let err = render_pie(&xml).expect_err(&format!(
                "pie dispBlanksAs={mode} with a blank should stay NotImplemented"
            ));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "pie dispBlanksAs={mode} with a blank should stay NotImplemented"
            );
        }
        render_line(&standard_line_xml(
            &line_ser(0, "S1", &[Some(1.0), Some(2.0), Some(3.0)], ""),
            "",
        ))
        .unwrap();
    }

    fn inject_explicit_log10(xml: &str) -> String {
        let scale =
            r#"<c:scaling><c:logBase val="10"/><c:min val="1"/><c:max val="1000"/></c:scaling>"#;
        if xml.contains(r#"<c:min val="-3"/>"#) {
            return xml.replace(
                "<c:scaling>\n          <c:min val=\"-3\"/>\n          <c:max val=\"5\"/>\n        </c:scaling>\n        <c:majorUnit val=\"1\"/>",
                scale,
            );
        }
        if xml.contains(
            r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling>"#,
        ) {
            return xml.replace(
                r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling>"#,
                &format!(r#"<c:valAx><c:axId val="2"/>{scale}"#),
            );
        }
        xml.replace(
            r#"<c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>"#,
            &format!(
                r#"<c:valAx>{scale}<c:majorGridlines/><c:crossBetween val="between"/></c:valAx>"#
            ),
        )
    }

    #[test]
    fn explicit_log_base_maps_equal_decades_for_column_and_line() {
        let plot = Rect {
            x: 0.0,
            y: 10.0,
            w: 100.0,
            h: 300.0,
        };
        let ticks = super::tick_values_log(1.0, 1000.0, 10.0);
        assert_eq!(ticks, vec![1.0, 10.0, 100.0, 1000.0]);
        let ys: Vec<i32> = ticks
            .iter()
            .map(|v| super::y_map(*v, 1.0, 1000.0, &plot, Some(10.0), false).round() as i32)
            .collect();
        assert_eq!(ys, vec![310, 210, 110, 10]);
        assert_eq!(
            super::tick_values_log(1.0, 32.0, 2.0),
            vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0]
        );

        let values = [1.0, 10.0, 100.0, 1000.0];
        let col = inject_explicit_log10(&mixed_column_xml(&values));
        let doc = crate::xml::parse(col.as_bytes()).unwrap();
        render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mut canvas = Canvas::new(300, 220);
        let bar_plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(
            &mut canvas,
            &chart,
            &bar_plot,
            1.0,
            1000.0,
            10.0,
            Some(10.0),
            1.0,
            false,
            None,
        );
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let mut counts = [0u32; 4];
        let slot = bar_plot.w / 4.0;
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                let i = (y as usize * canvas.w as usize + x as usize) * 3;
                if !near(&canvas.px[i..i + 3]) {
                    continue;
                }
                let ci = ((f64::from(x) - bar_plot.x) / slot).floor();
                if (0.0..4.0).contains(&ci) {
                    counts[ci as usize] += 1;
                }
            }
        }
        assert_eq!(counts[0], 0, "value==min must paint a zero-height bar");
        assert!(counts[1] > 0 && counts[2] > counts[1] && counts[3] > counts[2]);

        let line = inject_explicit_log10(&standard_line_xml(
            &line_ser(
                0,
                "S",
                &[Some(1.0), Some(10.0), Some(100.0), Some(1000.0)],
                "",
            ),
            "",
        ));
        render_line(&line).unwrap();

        let auto = col.replace(r#"<c:min val="1"/><c:max val="1000"/>"#, "");
        let auto_doc = crate::xml::parse(auto.as_bytes()).unwrap();
        let auto_chart = read_chart(
            &auto_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .expect("omitted log min/max with values >= 1 must export");
        assert_eq!(auto_chart.axis.min, None);
        render_rgb_chart(
            &auto_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("auto log10 1/10/100/1000 must render like explicit 1..1000");

        let below = inject_explicit_log10(&mixed_column_xml(&[0.5, 10.0, 100.0, 1000.0]));
        let below_doc = crate::xml::parse(below.as_bytes()).unwrap();
        render_rgb_chart(
            &below_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("positive values below the explicit log min must render a close-enough sliver");
        let zero = inject_explicit_log10(&mixed_column_xml(&[0.0, 10.0, 100.0, 1000.0]));
        let zero_doc = crate::xml::parse(zero.as_bytes()).unwrap();
        assert!(
            render_rgb_chart(
                &zero_doc.root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .is_err(),
            "non-positive log values stay NI"
        );

        let area = inject_explicit_log10(&standard_area_xml(
            &line_ser(
                0,
                "S",
                &[Some(1.0), Some(10.0), Some(100.0), Some(1000.0)],
                "",
            ),
            "",
        ));
        let err = render_area(&area).expect_err("area log scale stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);

        let bar = inject_explicit_log10(&bar_xml(
            &line_ser(
                0,
                "S",
                &[Some(1.0), Some(10.0), Some(100.0), Some(1000.0)],
                "",
            ),
            "",
            "maxMin",
            Some("bar"),
        ));
        let err = render_bar(&bar).expect_err("bar log scale stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_cat_crosses_at(xml: &str, value: &str) -> String {
        let xml = xml.replace(
            r#"<c:crosses val="autoZero"/>
      </c:catAx>"#,
            &format!(
                r#"<c:crossesAt val="{value}"/>
      </c:catAx>"#
            ),
        );
        let xml = if xml.contains(r#"<c:catAx/>"#) {
            xml.replace(
                r#"<c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>"#,
                &format!(
                    r#"<c:valAx><c:scaling><c:min val="-8"/><c:max val="8"/></c:scaling><c:majorUnit val="2"/><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx><c:crossesAt val="{value}"/></c:catAx>"#
                ),
            )
        } else {
            xml
        };
        // Compact bar fixtures keep autoZero on the same line as </c:catAx>.
        xml.replace(
            r#"<c:crosses val="autoZero"/></c:catAx>"#,
            &format!(r#"<c:crossesAt val="{value}"/></c:catAx>"#),
        )
    }

    fn mixed_column_explicit_pm8(values: &[f64]) -> String {
        mixed_column_xml(values)
            .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="-8"/>"#)
            .replace(r#"<c:max val="5"/>"#, r#"<c:max val="8"/>"#)
            .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#)
    }

    #[test]
    fn cat_axis_crosses_at_grows_columns_and_areas_from_the_crossing() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let xml = with_cat_crosses_at(&mixed_column_explicit_pm8(&values), "2");
        assert!(
            xml.contains(r#"<c:crossesAt val="2"/>"#),
            "fixture must carry catAx crossesAt"
        );
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(chart.axis.crosses_at, Some(2.0));
        let mut canvas = Canvas::new(300, 220);
        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        draw_bars(&mut canvas, &chart, &plot, -8.0, 8.0, 2.0, None, 2.0, false, None);
        let origin_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, 2.0);
        let zero_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, 0.0);
        assert_ne!(origin_y, zero_y);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let mut above_origin = 0u32;
        let mut between_origin_and_zero = 0u32;
        for y in 0..canvas.h {
            for x in 0..canvas.w {
                let i = (y as usize * canvas.w as usize + x as usize) * 3;
                if !near(&canvas.px[i..i + 3]) {
                    continue;
                }
                if (y as i32) < origin_y {
                    above_origin += 1;
                }
                if origin_y < y as i32 && (y as i32) < zero_y {
                    between_origin_and_zero += 1;
                }
            }
        }
        assert!(
            above_origin > 0,
            "values above the crossing must fill toward it"
        );
        assert_eq!(
            between_origin_and_zero, 0,
            "columns must not keep growing to zero once crossesAt is set"
        );

        render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        render_line(&with_cat_crosses_at(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "2",
        ))
        .unwrap();
        render_area(&with_cat_crosses_at(
            &standard_area_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "2",
        ))
        .unwrap();
        // Official F16-8 writes the Excel default midCat, not the between fixture.
        render_area(
            &with_cat_crosses_at(
                &standard_area_xml(
                    &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                    "",
                ),
                "2",
            )
            .replace(
                r#"crossBetween val="between""#,
                r#"crossBetween val="midCat""#,
            ),
        )
        .expect("standard area midCat plus catAx crossesAt must export");
        let bar = with_cat_crosses_at(
            &bar_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
                "maxMin",
                Some("bar"),
            ),
            "2",
        );
        assert!(
            bar.contains(r#"<c:crossesAt val="2"/>"#),
            "bar fixture must carry catAx crossesAt"
        );
        let err = render_bar(&bar).expect_err("bar crossesAt stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_cat_crosses(xml: &str, mode: &str) -> String {
        let xml = xml.replace(
            r#"<c:crosses val="autoZero"/>
      </c:catAx>"#,
            &format!(
                r#"<c:crosses val="{mode}"/>
      </c:catAx>"#
            ),
        );
        let xml = if xml.contains(r#"<c:catAx/>"#) {
            xml.replace(
                r#"<c:valAx><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx/>"#,
                &format!(
                    r#"<c:valAx><c:scaling><c:min val="-8"/><c:max val="8"/></c:scaling><c:majorUnit val="2"/><c:majorGridlines/><c:crossBetween val="between"/></c:valAx>
      <c:catAx><c:crosses val="{mode}"/></c:catAx>"#
                ),
            )
        } else {
            xml
        };
        xml.replace(
            r#"<c:crosses val="autoZero"/></c:catAx>"#,
            &format!(r#"<c:crosses val="{mode}"/></c:catAx>"#),
        )
    }

    fn first_accent_row(png: &[u8]) -> i32 {
        let (w, h, rgb) = png_rgb(png);
        let accent = FALLBACK_ACCENTS[0];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = (y * w as usize + x) * 3;
                if (rgb[i] as i16 - accent[0] as i16).abs() < 8
                    && (rgb[i + 1] as i16 - accent[1] as i16).abs() < 8
                    && (rgb[i + 2] as i16 - accent[2] as i16).abs() < 8
                {
                    return y as i32;
                }
            }
        }
        panic!("expected accent fill");
    }

    #[test]
    fn cat_axis_crosses_min_max_grows_from_the_bounds() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let min_xml = with_cat_crosses(&mixed_column_explicit_pm8(&values), "min");
        let max_xml = with_cat_crosses(&mixed_column_explicit_pm8(&values), "max");
        assert!(min_xml.contains(r#"<c:crosses val="min"/>"#));
        assert!(max_xml.contains(r#"<c:crosses val="max"/>"#));
        let min_doc = crate::xml::parse(min_xml.as_bytes()).unwrap();
        let max_doc = crate::xml::parse(max_xml.as_bytes()).unwrap();
        let min_chart = read_chart(
            &min_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let max_chart = read_chart(
            &max_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert_eq!(min_chart.axis.cat_crosses, super::axis::CatCrosses::Min);
        assert_eq!(max_chart.axis.cat_crosses, super::axis::CatCrosses::Max);

        let plot = Rect {
            x: 35.0,
            y: 16.0,
            w: 220.0,
            h: 188.0,
        };
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let min_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, -8.0);
        let max_y = crossing_axis_y(&plot, -8.0, 8.0, 2.0, 8.0);
        let mut min_canvas = Canvas::new(300, 220);
        draw_bars(
            &mut min_canvas,
            &min_chart,
            &plot,
            -8.0,
            8.0,
            2.0,
            None,
            -8.0,
            false,
            None,
        );
        let mut below_min = 0u32;
        let mut min_fill = 0u32;
        for y in 0..min_canvas.h {
            for x in 0..min_canvas.w {
                let i = (y as usize * min_canvas.w as usize + x as usize) * 3;
                if !near(&min_canvas.px[i..i + 3]) {
                    continue;
                }
                min_fill += 1;
                if (y as i32) > min_y {
                    below_min += 1;
                }
            }
        }
        assert!(min_fill > 0, "crosses=min columns must fill from the min");
        assert_eq!(below_min, 0, "crosses=min columns must not grow past min");

        let mut max_canvas = Canvas::new(300, 220);
        draw_bars(
            &mut max_canvas,
            &max_chart,
            &plot,
            -8.0,
            8.0,
            2.0,
            None,
            8.0,
            false,
            None,
        );
        let mut above_max = 0u32;
        let mut max_fill = 0u32;
        for y in 0..max_canvas.h {
            for x in 0..max_canvas.w {
                let i = (y as usize * max_canvas.w as usize + x as usize) * 3;
                if !near(&max_canvas.px[i..i + 3]) {
                    continue;
                }
                max_fill += 1;
                if (y as i32) < max_y {
                    above_max += 1;
                }
            }
        }
        assert!(max_fill > 0, "crosses=max columns must fill from the max");
        assert_eq!(above_max, 0, "crosses=max columns must not grow past max");

        let min_png = render_rgb_chart(
            &min_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let max_png = render_rgb_chart(
            &max_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let at_max = with_cat_crosses_at(&mixed_column_explicit_pm8(&values), "8");
        let at_doc = crate::xml::parse(at_max.as_bytes()).unwrap();
        let at_png = render_rgb_chart(
            &at_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let max_top = first_accent_row(&max_png);
        let at_top = first_accent_row(&at_png);
        assert!(
            max_top > at_top + 8,
            "official crosses=max keeps a top category-label gutter, unlike crossesAt=max; max={max_top} at={at_top}"
        );
        let _ = min_png;

        render_line(&with_cat_crosses(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "min",
        ))
        .unwrap();
        render_line(&with_cat_crosses(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "max",
        ))
        .unwrap();
        render_area(&with_cat_crosses(
            &standard_area_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "min",
        ))
        .unwrap();
        // Official F17-8 writes the Excel default midCat, not the between fixture.
        let area_mid_min = with_cat_crosses(
            &standard_area_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "min",
        )
        .replace(
            r#"crossBetween val="between""#,
            r#"crossBetween val="midCat""#,
        );
        render_area(&area_mid_min)
            .expect("standard area midCat plus catAx crosses=min must export");
        render_area(
            &with_cat_crosses(
                &standard_area_xml(
                    &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                    "",
                ),
                "max",
            )
            .replace(
                r#"crossBetween val="between""#,
                r#"crossBetween val="midCat""#,
            ),
        )
        .expect("standard area midCat plus catAx crosses=max must export under the same general rule");
        let bar = with_cat_crosses(
            &bar_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
                "maxMin",
                Some("bar"),
            ),
            "min",
        );
        assert!(bar.contains(r#"<c:crosses val="min"/>"#));
        let err = render_bar(&bar).expect_err("bar crosses=min stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_val_num_fmt(xml: &str, code: &str) -> String {
        let inject = format!(r#"<c:numFmt formatCode="{code}" sourceLinked="0"/>"#);
        let replaced = xml.replacen("</c:valAx>", &format!("{inject}</c:valAx>"), 1);
        assert!(
            replaced.contains(&inject),
            "fixture must carry valAx numFmt {code}"
        );
        replaced
    }

    fn mixed_column_thousands(values: &[f64]) -> String {
        mixed_column_xml(values)
            .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="-8000"/>"#)
            .replace(r#"<c:max val="5"/>"#, r#"<c:max val="8000"/>"#)
            .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2000"/>"#)
    }

    fn first_axis_x(png: &[u8]) -> u32 {
        let (w, h, rgb) = png_rgb(png);
        let y0 = h / 4;
        let y1 = (h * 3) / 4;
        let mut best_x = 0u32;
        let mut best_n = 0u32;
        for x in 0..w / 2 {
            let mut n = 0u32;
            for y in y0..y1 {
                let i = (y as usize * w as usize + x as usize) * 3;
                if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                    n += 1;
                }
            }
            if n > best_n {
                best_n = n;
                best_x = x;
            }
        }
        assert!(
            best_n > (y1 - y0) / 2,
            "expected a dark vertical value axis in the left half"
        );
        best_x
    }

    #[test]
    fn val_axis_locked_format_codes_match_official_tick_text() {
        use super::axis::TickFormat;
        let ticks = [-8.0, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0];
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::General)),
            ["-8", "-6", "-4", "-2", "0", "2", "4", "6", "8"]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Integer)),
            ["-8", "-6", "-4", "-2", "0", "2", "4", "6", "8"]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Fixed2)),
            [
                "-8.00", "-6.00", "-4.00", "-2.00", "0.00", "2.00", "4.00", "6.00",
                "8.00"
            ]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Percent)),
            [
                "-800%", "-600%", "-400%", "-200%", "0%", "200%", "400%", "600%",
                "800%"
            ]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::PercentFixed2)),
            [
                "-800.00%",
                "-600.00%",
                "-400.00%",
                "-200.00%",
                "0.00%",
                "200.00%",
                "400.00%",
                "600.00%",
                "800.00%"
            ]
        );
        assert_eq!(
            ticks.map(|v| fmt_tick_with(v, TickFormat::Scientific2)),
            [
                "-8.00E+00",
                "-6.00E+00",
                "-4.00E+00",
                "-2.00E+00",
                "0.00E+00",
                "2.00E+00",
                "4.00E+00",
                "6.00E+00",
                "8.00E+00"
            ]
        );
        let thousands = [-8000.0, -6000.0, -2000.0, 0.0, 2000.0, 8000.0];
        assert_eq!(
            thousands.map(|v| fmt_tick_with(v, TickFormat::Thousands)),
            ["-8,000", "-6,000", "-2,000", "0", "2,000", "8,000"]
        );
        assert_eq!(fmt_tick_with(-0.0, TickFormat::Fixed2), "0.00");
        assert_eq!(fmt_tick_with(-0.0, TickFormat::Percent), "0%");
        assert_eq!(fmt_tick_with(-0.0, TickFormat::Scientific2), "0.00E+00");
    }

    #[test]
    fn val_axis_non_general_format_codes_widen_labels_and_keep_unlocked_ni() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let general = mixed_column_explicit_pm8(&values);
        let general_doc = crate::xml::parse(general.as_bytes()).unwrap();
        let general_chart = read_chart(
            &general_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert_eq!(general_chart.axis.tick_format, super::axis::TickFormat::General);

        let fixed = with_val_num_fmt(&general, "0.00");
        let fixed_doc = crate::xml::parse(fixed.as_bytes()).unwrap();
        let fixed_chart = read_chart(
            &fixed_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert_eq!(fixed_chart.axis.tick_format, super::axis::TickFormat::Fixed2);

        let general_png = render_rgb_chart(
            &general_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let fixed_png = render_rgb_chart(
            &fixed_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let general_x = first_axis_x(&general_png);
        let fixed_x = first_axis_x(&fixed_png);
        assert!(
            fixed_x > general_x + 8,
            "0.00 tick labels must widen the left gutter; general={general_x} fixed={fixed_x}"
        );

        let integer = with_val_num_fmt(&general, "0");
        let integer_doc = crate::xml::parse(integer.as_bytes()).unwrap();
        let integer_png = render_rgb_chart(
            &integer_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let integer_x = first_axis_x(&integer_png);
        assert!(
            (integer_x as i32 - general_x as i32).abs() <= 1,
            "format 0 on integer ticks matches General gutter; general={general_x} integer={integer_x}"
        );

        for code in ["0%", "0.00%", "0.00E+00"] {
            render_rgb_chart(
                &crate::xml::parse(with_val_num_fmt(&general, code).as_bytes())
                    .unwrap()
                    .root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .unwrap();
        }
        let thousands = with_val_num_fmt(
            &mixed_column_thousands(&[-4000.0, -1000.0, 3000.0, 6000.0]),
            "#,##0",
        );
        render_rgb_chart(
            &crate::xml::parse(thousands.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        render_line(&with_val_num_fmt(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "0.00",
        ))
        .unwrap();

        let currency = with_val_num_fmt(&general, "$#,##0.00");
        let err = render_rgb_chart(
            &crate::xml::parse(currency.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect_err("currency formatCode stays NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_val_orientation(xml: &str, mode: &str) -> String {
        let start = xml.find("<c:valAx").expect("fixture must have valAx");
        let rel_end = xml[start..]
            .find("</c:valAx>")
            .expect("fixture must close valAx");
        let end = start + rel_end;
        let val = &xml[start..end];
        let inject = format!(r#"<c:orientation val="{mode}"/>"#);
        let new_val = if let Some((head, tail)) = val.split_once("<c:orientation val=\"") {
            let close = tail.find('"').expect("orientation close");
            format!("{}<c:orientation val=\"{}{}", head, mode, &tail[close..])
        } else if val.contains("<c:scaling>") {
            val.replacen("<c:scaling>", &format!("<c:scaling>{inject}"), 1)
        } else {
            let gt = val.find('>').expect("valAx open tag");
            format!(
                "{}<c:scaling>{inject}</c:scaling>{}",
                &val[..=gt],
                &val[gt + 1..]
            )
        };
        let out = format!("{}{}{}", &xml[..start], new_val, &xml[end..]);
        assert!(
            out[start..].contains(&inject),
            "fixture must carry valAx orientation {mode}"
        );
        out
    }

    #[test]
    fn val_axis_orientation_max_min_flips_columns_lines_and_bars() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let mixed = mixed_column_explicit_pm8(&values).replacen(
            "<c:ser>",
            r#"<c:ser><c:invertIfNegative val="0"/>"#,
            1,
        );
        let xml = with_val_orientation(&mixed, "maxMin");
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(
            &doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(chart.axis.reversed);
        let mixed_scale = super::axis::scale(&chart.series, chart.axis, 10).unwrap();
        assert!(!mixed_scale.category_labels_above());
        assert!(!mixed_scale.category_labels_below());

        let png = render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let minmax_doc = crate::xml::parse(mixed.as_bytes()).unwrap();
        let minmax_png = render_rgb_chart(
            &minmax_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let (_, _, minmax_rgb) = png_rgb(&minmax_png);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let centroid_y = |rgb: &[u8], x0: u32, x1: u32| {
            let mut sum = 0u64;
            let mut n = 0u64;
            for y in 0..h {
                for x in x0..x1 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        sum += u64::from(y);
                        n += 1;
                    }
                }
            }
            assert!(n > 0, "expected accent fill in x={x0}..{x1}");
            sum as f64 / n as f64
        };
        // Official F19-2: negative A/B grow upward (toward -8 at the top).
        // The first category (A=-4) occupies the left of the plot. Do not use
        // the longest dark row as the crossing: major grids are the same length
        // as the category axis and the top grid wins first.
        let mid = w / 2;
        let left_maxmin = centroid_y(&rgb, 0, mid);
        let right_maxmin = centroid_y(&rgb, mid, w);
        assert!(
            left_maxmin + 8.0 < right_maxmin,
            "official maxMin grows negative columns toward the top; left={left_maxmin} right={right_maxmin}"
        );
        let left_minmax = centroid_y(&minmax_rgb, 0, mid);
        let right_minmax = centroid_y(&minmax_rgb, mid, w);
        assert!(
            left_minmax > right_minmax + 8.0,
            "minMax control grows negative columns downward; left={left_minmax} right={right_minmax}"
        );

        let pos = with_val_orientation(
            &mixed_column_xml(&[2.0, 4.0, 6.0, 8.0])
                .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="0"/>"#)
                .replace(r#"<c:max val="5"/>"#, r#"<c:max val="10"/>"#)
                .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#),
            "maxMin",
        );
        let pos_doc = crate::xml::parse(pos.as_bytes()).unwrap();
        let pos_chart = read_chart(
            &pos_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(pos_chart.axis.reversed);
        let pos_scale = super::axis::scale(&pos_chart.series, pos_chart.axis, 10).unwrap();
        assert!(pos_scale.category_labels_above());
        assert!(!pos_scale.category_labels_below());
        let pos_png = render_rgb_chart(
            &pos_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let pos_minmax_xml = mixed_column_xml(&[2.0, 4.0, 6.0, 8.0])
            .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="0"/>"#)
            .replace(r#"<c:max val="5"/>"#, r#"<c:max val="10"/>"#)
            .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#);
        let pos_minmax_png = render_rgb_chart(
            &crate::xml::parse(pos_minmax_xml.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let dark_band = |png: &[u8], y0: u32, y1: u32| {
            let (w, _, rgb) = png_rgb(png);
            let mut n = 0u32;
            for y in y0..y1 {
                for x in 0..w {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let (_, ph, _) = png_rgb(&pos_png);
        let top = dark_band(&pos_png, 0, ph / 6);
        let bottom = dark_band(&pos_png, (ph * 5) / 6, ph);
        let top_ctrl = dark_band(&pos_minmax_png, 0, ph / 6);
        let bottom_ctrl = dark_band(&pos_minmax_png, (ph * 5) / 6, ph);
        assert!(
            top > bottom && top > top_ctrl,
            "positive maxMin keeps category labels in the top gutter; top={top} bottom={bottom} ctrl_top={top_ctrl} ctrl_bottom={bottom_ctrl}"
        );
        assert!(
            bottom_ctrl > bottom,
            "positive minMax keeps category labels in the bottom gutter; bottom={bottom} ctrl_bottom={bottom_ctrl}"
        );

        let neg = with_val_orientation(
            &mixed_column_xml(&[-8.0, -5.0, -3.0, -1.0])
                .replace(r#"<c:min val="-3"/>"#, r#"<c:min val="-10"/>"#)
                .replace(r#"<c:max val="5"/>"#, r#"<c:max val="0"/>"#)
                .replace(r#"<c:majorUnit val="1"/>"#, r#"<c:majorUnit val="2"/>"#),
            "maxMin",
        );
        let neg_doc = crate::xml::parse(neg.as_bytes()).unwrap();
        let neg_chart = read_chart(
            &neg_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let neg_scale = super::axis::scale(&neg_chart.series, neg_chart.axis, 10).unwrap();
        assert!(neg_scale.category_labels_below());
        assert!(!neg_scale.category_labels_above());
        render_rgb_chart(
            &neg_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();

        render_line(&with_val_orientation(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "maxMin",
        ))
        .unwrap();

        let bar_vals = [Some(2.0), Some(4.0), Some(6.0), Some(8.0)];
        let bar_base = bar_xml(&line_ser(0, "S", &bar_vals, ""), "", "minMax", Some("bar"));
        let bar_png = render_bar(&with_val_orientation(&bar_base, "maxMin")).unwrap();
        let minmax_bar = render_bar(&bar_base).unwrap();
        let (bw, bh, brgb) = png_rgb(&bar_png);
        let (_, _, mrgb) = png_rgb(&minmax_bar);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let centroid_x = |rgb: &[u8]| {
            let mut sum = 0u64;
            let mut n = 0u64;
            for y in 0..bh {
                for x in 0..bw {
                    let i = (y as usize * bw as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        sum += u64::from(x);
                        n += 1;
                    }
                }
            }
            assert!(n > 0, "expected horizontal-bar accent fill");
            sum as f64 / n as f64
        };
        let maxmin_x = centroid_x(&brgb);
        let minmax_x = centroid_x(&mrgb);
        // Positive bars grow from the visual zero. Official F19-7 puts max
        // on the left, so autoZero sits on the right and the fill centroid
        // stays right of a minMax control (whose zero is on the left).
        assert!(
            maxmin_x > minmax_x + 8.0,
            "official bar maxMin places max on the left so positives grow left from a right-hand zero; maxmin={maxmin_x} minmax={minmax_x}"
        );
        // Official F19-7 writes catAx axPos=r with valAx orientation=maxMin.
        render_bar(&with_val_orientation(
            &bar_base.replace(r#"<c:axPos val="l"/>"#, r#"<c:axPos val="r"/>"#),
            "maxMin",
        ))
        .unwrap();

        let log = with_val_orientation(
            &inject_explicit_log10(&mixed_column_xml(&[1.0, 10.0, 100.0, 1000.0])),
            "maxMin",
        );
        let log_doc = crate::xml::parse(log.as_bytes()).unwrap();
        assert!(
            read_chart(&log_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "log plus orientation=maxMin stays NI"
        );

        let labeled = xml.replacen(
            "</c:ser>",
            r#"<c:dLbls><c:showVal val="1"/></c:dLbls></c:ser>"#,
            1,
        );
        let labeled_doc = crate::xml::parse(labeled.as_bytes()).unwrap();
        let err = render_rgb_chart(
            &labeled_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect_err("visible value labels plus orientation=maxMin stay NI");
        assert_eq!(err.code(), crate::error::ErrorCode::NotImplemented);
    }

    fn with_val_delete(xml: &str) -> String {
        let start = xml.find("<c:valAx").expect("fixture must have valAx");
        let rel_end = xml[start..]
            .find("</c:valAx>")
            .expect("fixture must close valAx");
        let end = start + rel_end;
        let val = &xml[start..end];
        let new_val = if let Some((head, tail)) = val.split_once(r#"<c:delete val=""#) {
            let close = tail.find('"').expect("delete close");
            format!(r#"{}<c:delete val="1{}"#, head, &tail[close..])
        } else {
            let gt = val.find('>').expect("valAx open tag");
            format!(r#"{}<c:delete val="1"/>{}"#, &val[..=gt], &val[gt + 1..])
        };
        let out = format!("{}{}{}", &xml[..start], new_val, &xml[end..]);
        assert!(
            out[start..].contains(r#"<c:delete val="1"/>"#),
            "fixture must carry valAx delete=true"
        );
        out
    }

    #[test]
    fn val_axis_delete_hides_axis_line_and_releases_gutter() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let mixed = mixed_column_explicit_pm8(&values).replacen(
            "<c:ser>",
            r#"<c:ser><c:invertIfNegative val="0"/>"#,
            1,
        );
        let control_doc = crate::xml::parse(mixed.as_bytes()).unwrap();
        let delete_doc = crate::xml::parse(with_val_delete(&mixed).as_bytes()).unwrap();
        let control_chart = read_chart(
            &control_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let delete_chart = read_chart(
            &delete_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(!control_chart.axis.deleted);
        assert!(delete_chart.axis.deleted);

        let render = |root: &crate::xml::Element| {
            render_rgb_chart(
                root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .unwrap()
        };
        let control_png = render(&control_doc.root);
        let delete_png = render(&delete_doc.root);
        let (w, h, crgb) = png_rgb(&control_png);
        let (_, _, drgb) = png_rgb(&delete_png);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let first_accent_x = |rgb: &[u8]| {
            for x in 0..w {
                for y in 0..h {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        return x;
                    }
                }
            }
            panic!("expected accent fill");
        };
        let left_x = first_accent_x(&crgb);
        let del_x = first_accent_x(&drgb);
        assert!(
            del_x + 8 < left_x,
            "official delete=true releases the value-label gutter; delete={del_x} control={left_x}"
        );
        let dark_left = |rgb: &[u8]| {
            let mut n = 0u32;
            for y in h / 6..(h * 5) / 6 {
                for x in 0..w / 8 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let control_left = dark_left(&crgb);
        let delete_left = dark_left(&drgb);
        assert!(
            control_left > 20 && delete_left + 10 < control_left,
            "official delete=true omits value tick labels; control_left={control_left} delete_left={delete_left}"
        );

        render_line(&with_val_delete(&standard_line_xml(
            &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
            "",
        )))
        .unwrap();

        let bar_base = bar_xml(
            &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
            "",
            "minMax",
            Some("bar"),
        );
        let bar_ctrl = render_bar(&bar_base).unwrap();
        let bar_del = render_bar(&with_val_delete(&bar_base)).unwrap();
        let (bw, bh, brgb) = png_rgb(&bar_ctrl);
        let (_, _, bdrgb) = png_rgb(&bar_del);
        let bottom_dark = |rgb: &[u8]| {
            let mut n = 0u32;
            for y in (bh * 5) / 6..bh {
                for x in 0..bw {
                    let i = (y as usize * bw as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let bar_bottom = bottom_dark(&brgb);
        let bar_del_bottom = bottom_dark(&bdrgb);
        assert!(
            bar_bottom > bar_del_bottom + 10,
            "official bar delete=true releases the bottom value-label gutter; control={bar_bottom} delete={bar_del_bottom}"
        );

        let log = with_val_delete(&inject_explicit_log10(&mixed_column_xml(&[
            1.0, 10.0, 100.0, 1000.0,
        ])));
        let log_doc = crate::xml::parse(log.as_bytes()).unwrap();
        assert!(
            read_chart(&log_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "log plus valAx delete=true stays NI"
        );
    }

    fn with_val_tick_lbl_pos(xml: &str, pos: &str) -> String {
        let start = xml.find("<c:valAx").expect("fixture must have valAx");
        let rel_end = xml[start..]
            .find("</c:valAx>")
            .expect("fixture must close valAx");
        let end = start + rel_end;
        let val = &xml[start..end];
        let new_val = if let Some((head, tail)) = val.split_once(r#"<c:tickLblPos val=""#) {
            let close = tail.find('"').expect("tickLblPos close");
            format!(r#"{}<c:tickLblPos val="{}{}"#, head, pos, &tail[close..])
        } else {
            format!(r#"{}<c:tickLblPos val="{}"/>"#, val, pos)
        };
        let out = format!("{}{}{}", &xml[..start], new_val, &xml[end..]);
        assert!(
            out[start..end + new_val.len() - val.len()].contains(&format!(
                r#"<c:tickLblPos val="{pos}"/>"#
            )),
            "fixture must carry valAx tickLblPos={pos}"
        );
        out
    }

    #[test]
    fn val_axis_tick_lbl_pos_none_keeps_axis_line_and_releases_gutter() {
        let values = [-4.0, -1.0, 3.0, 6.0];
        let mixed = mixed_column_explicit_pm8(&values).replacen(
            "<c:ser>",
            r#"<c:ser><c:invertIfNegative val="0"/>"#,
            1,
        );
        let control_doc = crate::xml::parse(mixed.as_bytes()).unwrap();
        let none_xml = with_val_tick_lbl_pos(&mixed, "none");
        let none_doc = crate::xml::parse(none_xml.as_bytes()).unwrap();
        let delete_doc = crate::xml::parse(with_val_delete(&mixed).as_bytes()).unwrap();
        let none_chart = read_chart(
            &none_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(none_chart.axis.hide_tick_labels);
        assert!(!none_chart.axis.deleted);

        let render = |root: &crate::xml::Element| {
            render_rgb_chart(
                root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
            )
            .unwrap()
        };
        let control_png = render(&control_doc.root);
        let none_png = render(&none_doc.root);
        let delete_png = render(&delete_doc.root);
        let (w, h, crgb) = png_rgb(&control_png);
        let (_, _, nrgb) = png_rgb(&none_png);
        let (_, _, drgb) = png_rgb(&delete_png);
        let accent = FALLBACK_ACCENTS[0];
        let near = |px: &[u8]| {
            (px[0] as i16 - accent[0] as i16).abs() < 8
                && (px[1] as i16 - accent[1] as i16).abs() < 8
                && (px[2] as i16 - accent[2] as i16).abs() < 8
        };
        let first_accent_x = |rgb: &[u8]| {
            for x in 0..w {
                for y in 0..h {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if near(&rgb[i..i + 3]) {
                        return x;
                    }
                }
            }
            panic!("expected accent fill");
        };
        let left_x = first_accent_x(&crgb);
        let none_x = first_accent_x(&nrgb);
        let del_x = first_accent_x(&drgb);
        assert!(
            none_x + 8 < left_x,
            "official tickLblPos=none releases the value-label gutter; none={none_x} control={left_x}"
        );
        assert!(
            (none_x as i32 - del_x as i32).abs() <= 2,
            "tickLblPos=none and delete=true release the same gutter; none={none_x} delete={del_x}"
        );
        // The leftover value axis is the tallest vertical ink in the left
        // quarter (first_accent_x is the first column, not the axis).
        // Labels would sit left of that axis, beyond TICK_OUT=3.
        let y0 = h / 6;
        let y1 = (h * 5) / 6;
        let tallest_vline = |rgb: &[u8]| {
            let mut best_x = 0u32;
            let mut best = 0u32;
            for x in 0..w / 4 {
                let mut n = 0u32;
                for y in y0..y1 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
                if n > best {
                    best = n;
                    best_x = x;
                }
            }
            (best_x, best)
        };
        let (none_axis_x, none_axis) = tallest_vline(&nrgb);
        let (_, delete_axis) = tallest_vline(&drgb);
        assert!(
            none_axis > delete_axis + 20,
            "official tickLblPos=none keeps the value axis line; none={none_axis}@{none_axis_x} delete={delete_axis}"
        );
        let left_of_ticks = |rgb: &[u8], axis_x: u32| {
            let mut n = 0u32;
            let lim = axis_x.saturating_sub(4);
            for y in y0..y1 {
                for x in 0..lim {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let none_outside = left_of_ticks(&nrgb, none_axis_x);
        let control_axis_x = tallest_vline(&crgb).0;
        let control_outside = left_of_ticks(&crgb, control_axis_x);
        assert!(
            control_outside > 20 && none_outside < 5,
            "official tickLblPos=none omits value tick labels left of the axis; control_outside={control_outside} none_outside={none_outside} axis={none_axis_x}"
        );

        render_line(&with_val_tick_lbl_pos(
            &standard_line_xml(
                &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
                "",
            ),
            "none",
        ))
        .unwrap();

        let bar_base = bar_xml(
            &line_ser(0, "S", &[Some(-4.0), Some(-1.0), Some(3.0), Some(6.0)], ""),
            "",
            "minMax",
            Some("bar"),
        );
        let bar_none_xml = with_val_tick_lbl_pos(&bar_base, "none");
        let bar_none_doc = crate::xml::parse(bar_none_xml.as_bytes()).unwrap();
        let bar_none_chart = read_chart(
            &bar_none_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        assert!(bar_none_chart.axis.hide_tick_labels);
        assert!(!bar_none_chart.axis.deleted);
        render_bar(&bar_none_xml).unwrap();

        let log = with_val_tick_lbl_pos(
            &inject_explicit_log10(&mixed_column_xml(&[1.0, 10.0, 100.0, 1000.0])),
            "none",
        );
        let log_doc = crate::xml::parse(log.as_bytes()).unwrap();
        assert!(
            read_chart(&log_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "log plus tickLblPos=none stays NI"
        );
    }

    fn secondary_pair_xml(kind: &str, s1: &[f64], s2: &[f64], auto: bool) -> String {
        let n = s1.len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let nums = |vals: &[f64]| {
            vals.iter()
                .enumerate()
                .map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#))
                .collect::<String>()
        };
        let scale_p = if auto {
            String::new()
        } else if s1.iter().any(|v| *v < 0.0) {
            r#"<c:min val="-8"/><c:max val="8"/>"#.into()
        } else {
            r#"<c:min val="0"/><c:max val="10"/>"#.into()
        };
        let major_p = if auto {
            String::new()
        } else if s1.iter().any(|v| *v < 0.0) {
            r#"<c:majorUnit val="2"/>"#.into()
        } else {
            r#"<c:majorUnit val="2"/>"#.into()
        };
        let scale_s = if auto {
            String::new()
        } else {
            r#"<c:min val="0"/><c:max val="100"/>"#.into()
        };
        let major_s = if auto {
            String::new()
        } else {
            r#"<c:majorUnit val="20"/>"#.into()
        };
        let (w1, w2) = match kind {
            "line" => (
                format!(
                    r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:axId val="1"/><c:axId val="2"/></c:lineChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:axId val="3"/><c:axId val="4"/></c:lineChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            "bar" => (
                format!(
                    r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="3"/><c:axId val="4"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            "area" => (
                format!(
                    r#"<c:areaChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:axId val="1"/><c:axId val="2"/></c:areaChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:areaChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:axId val="3"/><c:axId val="4"/></c:areaChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            "combo" => (
                format!(
                    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:axId val="3"/><c:axId val="4"/></c:lineChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
            _ => (
                format!(
                    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n1 = nums(s1),
                ),
                format!(
                    r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser><c:gapWidth val="150"/><c:axId val="3"/><c:axId val="4"/></c:barChart>"#,
                    n = n,
                    cats = cats,
                    n2 = nums(s2),
                ),
            ),
        };
        let (cat_pos, val_p, val_s, between) = if kind == "bar" {
            ("l", "b", "t", "between")
        } else if kind == "area" {
            ("b", "l", "r", "midCat")
        } else {
            ("b", "l", "r", "between")
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      {w1}{w2}
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="{cat_pos}"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/>{scale_p}</c:scaling>{major_p}<c:delete val="0"/><c:axPos val="{val_p}"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="{between}"/></c:valAx>
      <c:valAx><c:axId val="4"/><c:scaling><c:orientation val="minMax"/>{scale_s}</c:scaling>{major_s}<c:delete val="0"/><c:axPos val="{val_s}"/><c:crossAx val="3"/><c:crosses val="max"/><c:crossBetween val="{between}"/></c:valAx>
      <c:catAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="1"/><c:axPos val="{cat_pos}"/><c:crossAx val="4"/></c:catAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn two_series_primary_xml(s1: &[f64], s2: &[f64]) -> String {
        let n = s1.len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let nums = |vals: &[f64]| {
            vals.iter()
                .enumerate()
                .map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#))
                .collect::<String>()
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="col"/><c:grouping val="clustered"/>
        <c:ser><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n1}</c:numLit></c:val></c:ser>
        <c:ser><c:order val="1"/><c:tx><c:v>S2</c:v></c:tx><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{n}"/>{n2}</c:numLit></c:val></c:ser>
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:min val="0"/><c:max val="100"/></c:scaling><c:majorUnit val="20"/><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            n = n,
            cats = cats,
            n1 = nums(s1),
            n2 = nums(s2),
        )
    }

    fn accent_x_runs(rgb: &[u8], w: u32, h: u32, accent: [u8; 3]) -> Vec<(u32, u32)> {
        let mut xs = Vec::new();
        for x in 0..w {
            let mut n = 0u32;
            for y in 0..h {
                let i = (y as usize * w as usize + x as usize) * 3;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 16) {
                    n += 1;
                }
            }
            if n >= 3 {
                xs.push(x);
            }
        }
        let mut runs = Vec::new();
        if xs.is_empty() {
            return runs;
        }
        let mut start = xs[0];
        let mut prev = xs[0];
        for x in xs.into_iter().skip(1) {
            if x <= prev + 2 {
                prev = x;
            } else {
                // drop slim legend swatches
                if prev - start >= 8 {
                    runs.push((start, prev));
                }
                start = x;
                prev = x;
            }
        }
        if prev - start >= 8 {
            runs.push((start, prev));
        }
        runs
    }

    #[test]
    fn secondary_val_axis_maps_own_scale_and_overlaps_same_type_columns() {
        let s1 = [2.0, 4.0, 6.0, 8.0];
        let s2 = [20.0, 40.0, 80.0, 60.0];
        let xml = secondary_pair_xml("col", &s1, &s2, false);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert!(chart.axis_secondary.is_some());
        assert_eq!(chart.axis.min, Some(0.0));
        assert_eq!(chart.axis.max, Some(10.0));
        let sec = chart.axis_secondary.unwrap();
        assert_eq!(sec.min, Some(0.0));
        assert_eq!(sec.max, Some(100.0));
        assert!(!chart.series[0].on_secondary);
        assert!(chart.series[1].on_secondary);

        let png = render_rgb_chart(
            &doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a0 = FALLBACK_ACCENTS[0];
        let a1 = FALLBACK_ACCENTS[1];
        let r0 = accent_x_runs(&rgb, w, h, a0);
        let r1 = accent_x_runs(&rgb, w, h, a1);
        // Official same-type secondary columns overlap in the same slots.
        // Secondary is painted on top, so S1 is only visible in D (8/10 vs 60/100).
        assert_eq!(r1.len(), 4, "secondary series must occupy four category slots, got {r1:?}");
        assert_eq!(
            r0.len(),
            1,
            "primary S1 should only stick out above S2 in category D, got {r0:?}"
        );
        assert!(
            r1.iter().any(|(s, e)| r0[0].0 <= *e && r0[0].1 >= *s),
            "official secondary columns overlap the primary slot; primary={r0:?} secondary={r1:?}"
        );
        let ctrl_doc = crate::xml::parse(two_series_primary_xml(&s1, &s2).as_bytes()).unwrap();
        let ctrl = read_chart(&ctrl_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert!(ctrl.axis_secondary.is_none());
        let ctrl_png = render_rgb_chart(
            &ctrl_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (cw, ch, crgb) = png_rgb(&ctrl_png);
        let c0 = accent_x_runs(&crgb, cw, ch, a0);
        let c1 = accent_x_runs(&crgb, cw, ch, a1);
        assert_eq!(c0.len(), 4);
        assert_eq!(c1.len(), 4);
        // Control clusters sit side by side, so matching-index runs should not fully coincide.
        for ((a0s, a0e), (a1s, a1e)) in c0.iter().zip(&c1) {
            assert!(
                *a0e < *a1s || *a1e < *a0s,
                "one-axis two-series columns stay clustered side by side; {c0:?} {c1:?}"
            );
        }
        let secondary_w: i32 = r1.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        let cluster_w: i32 = c0.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        assert!(
            secondary_w > cluster_w,
            "each secondary-axis wrapper is a one-series cluster, so bars are wider than a two-series cluster; secondary_w={secondary_w} control_w={cluster_w}"
        );
        // Right-side tick labels: more ink in the right gutter than the one-axis control.
        let ink = |rgb: &[u8], w: u32, h: u32| {
            let mut n = 0u32;
            for y in h / 6..(h * 5) / 6 {
                for x in (w * 5) / 8..(w * 7) / 8 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    if rgb[i] < 40 && rgb[i + 1] < 40 && rgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let right = ink(&rgb, w, h);
        let ctrl_right = ink(&crgb, cw, ch);
        assert!(
            right > ctrl_right + 10,
            "official secondary valAx places tick labels on the right; secondary={right} control={ctrl_right}"
        );

        render_line(&secondary_pair_xml(
            "line",
            &s1,
            &s2,
            false,
        ))
        .unwrap();

        // Official F21-5 / F21-6 / F21-7 / F21-8 Chart.Export gold exists, so
        // these must produce an image (t215u). Close-enough composition of
        // the already-locked pairing; residuals stay on the review page.
        let bar_doc = crate::xml::parse(secondary_pair_xml("bar", &s1, &s2, false).as_bytes()).unwrap();
        let bar_chart = read_chart(&bar_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("clustered bar secondary valAx must export");
        assert!(bar_chart.axis_secondary.is_some());
        assert!(!bar_chart.series[0].on_secondary);
        assert!(bar_chart.series[1].on_secondary);
        let bar_png = render_rgb_chart(
            &bar_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("clustered bar secondary valAx must produce an image");
        let (bw, bh, brgb) = png_rgb(&bar_png);
        let accent_count = |rgb: &[u8], w: u32, h: u32, accent: [u8; 3]| {
            let mut n = 0u32;
            for i in (0..(w * h) as usize).map(|i| i * 3) {
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 16) {
                    n += 1;
                }
            }
            n
        };
        assert!(
            accent_count(&brgb, bw, bh, a0) > 50 && accent_count(&brgb, bw, bh, a1) > 50,
            "bar secondary must paint both series"
        );
        let top_ink = {
            let mut n = 0u32;
            for y in 0..bh / 8 {
                for x in 0..bw {
                    let i = (y as usize * bw as usize + x as usize) * 3;
                    if brgb[i] < 40 && brgb[i + 1] < 40 && brgb[i + 2] < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        assert!(
            top_ink > 20,
            "official bar secondary valAx places tick labels on the top; top_ink={top_ink}"
        );

        let auto_doc = crate::xml::parse(secondary_pair_xml("col", &s1, &s2, true).as_bytes()).unwrap();
        let auto_chart = read_chart(&auto_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("omitted min/max on a secondary pair must export");
        assert!(auto_chart.axis.min.is_none() && auto_chart.axis.max.is_none());
        assert!(auto_chart
            .axis_secondary
            .as_ref()
            .is_some_and(|spec| spec.min.is_none() && spec.max.is_none()));
        let auto_png = render_rgb_chart(
            &auto_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("omitted min/max secondary pair must produce an image");
        let (aw, ah, argb) = png_rgb(&auto_png);
        let auto_r1 = accent_x_runs(&argb, aw, ah, a1);
        assert_eq!(
            auto_r1.len(),
            4,
            "auto secondary columns should occupy four category slots, got {auto_r1:?}"
        );

        let area_doc = crate::xml::parse(secondary_pair_xml("area", &s1, &s2, false).as_bytes()).unwrap();
        let area_chart = read_chart(&area_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("standard area secondary valAx must export");
        assert!(area_chart.area_midcat);
        assert!(area_chart.axis_secondary.is_some());
        render_rgb_chart(
            &area_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("standard area secondary valAx must produce an image");

        let combo_doc = crate::xml::parse(secondary_pair_xml("combo", &s1, &s2, false).as_bytes()).unwrap();
        let combo_chart = read_chart(&combo_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("official column+line secondary combo must export");
        assert!(matches!(combo_chart.series[0].plot_kind, PlotKind::ClusteredColumn));
        assert!(matches!(combo_chart.series[1].plot_kind, PlotKind::StandardLine));
        assert!(!combo_chart.series[0].on_secondary);
        assert!(combo_chart.series[1].on_secondary);
        let combo_png = render_rgb_chart(
            &combo_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("official column+line secondary combo must produce an image");
        let (cow, coh, corgb) = png_rgb(&combo_png);
        let combo_r0 = accent_x_runs(&corgb, cow, coh, a0);
        assert_eq!(
            combo_r0.len(),
            4,
            "combo primary must still be four columns, got {combo_r0:?}"
        );
        assert!(
            accent_count(&corgb, cow, coh, a1) > 20,
            "combo secondary line must be painted, not omitted"
        );
        assert!(
            combo_chart.series[1].line_visible
                || !matches!(combo_chart.series[1].marker, MarkerKind::None),
            "combo secondary must keep the line series style rather than a column fill"
        );
        let combo_right = ink(&corgb, cow, coh);
        assert!(
            combo_right > 10,
            "combo secondary valAx still places tick labels on the right; ink={combo_right}"
        );
    }

    fn stacked_xml(
        dir: &str,
        grouping: &str,
        series: &[&[Option<f64>]],
        auto: bool,
        invert: bool,
        labels: bool,
    ) -> String {
        let n = series[0].len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let mut sers = String::new();
        for (si, vals) in series.iter().enumerate() {
            let pts = vals
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.map(|x| format!(r#"<c:pt idx="{i}"><c:v>{x}</c:v></c:pt>"#))
                })
                .collect::<String>();
            let present = vals.iter().filter(|v| v.is_some()).count();
            let inv = if invert { "1" } else { "0" };
            let d_lbls = if labels {
                r#"<c:dLbls><c:dLblPos val="outEnd"/><c:showVal val="1"/></c:dLbls>"#
            } else {
                ""
            };
            sers.push_str(&format!(
                r#"<c:ser><c:idx val="{si}"/><c:order val="{si}"/><c:tx><c:v>S{si}</c:v></c:tx><c:invertIfNegative val="{inv}"/><c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{present}"/>{pts}</c:numLit></c:val>{d_lbls}</c:ser>"#
            ));
        }
        let (vmin, vmax, major, numfmt) = if grouping == "percentStacked" {
            (
                "0",
                "1",
                "0.2",
                r#"<c:numFmt formatCode="0%" sourceLinked="1"/>"#,
            )
        } else {
            ("0", "80", "20", "")
        };
        let scaling = if auto {
            r#"<c:scaling><c:orientation val="minMax"/></c:scaling>"#.to_string()
        } else {
            format!(
                r#"<c:scaling><c:orientation val="minMax"/><c:min val="{vmin}"/><c:max val="{vmax}"/></c:scaling>"#
            )
        };
        let major_el = if auto {
            String::new()
        } else {
            format!(r#"<c:majorUnit val="{major}"/>"#)
        };
        let (cat_pos, val_pos) = if dir == "bar" { ("l", "b") } else { ("b", "l") };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:barChart>
        <c:barDir val="{dir}"/><c:grouping val="{grouping}"/><c:varyColors val="0"/>
        {sers}
        <c:gapWidth val="150"/><c:overlap val="100"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="{cat_pos}"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/>{scaling}{major_el}<c:delete val="0"/><c:axPos val="{val_pos}"/><c:majorGridlines/>{numfmt}<c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn clustered_three_xml(series: &[&[Option<f64>]]) -> String {
        stacked_xml("col", "stacked", series, false, false, false).replacen(
            r#"grouping val="stacked""#,
            r#"grouping val="clustered""#,
            1,
        ).replacen(r#"<c:overlap val="100"/>"#, "", 1)
    }

    #[test]
    fn stacked_and_percent_stacked_bars_accumulate_in_series_order() {
        let s1 = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let s2 = [Some(20.0), Some(10.0), Some(20.0), Some(10.0)];
        let s3 = [Some(20.0), Some(30.0), Some(10.0), Some(20.0)];
        let series: [&[Option<f64>]; 3] = [&s1, &s2, &s3];
        let xml = stacked_xml("col", "stacked", &series, false, false, false);
        let doc = crate::xml::parse(xml.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(chart.stacking, Stacking::Stacked);
        assert_eq!(chart.overlap, 100.0);

        let png = render_rgb_chart(
            &doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (w, h, rgb) = png_rgb(&png);
        let a0 = FALLBACK_ACCENTS[0];
        let a1 = FALLBACK_ACCENTS[1];
        let a2 = FALLBACK_ACCENTS[2];
        let r0 = accent_x_runs(&rgb, w, h, a0);
        let r1 = accent_x_runs(&rgb, w, h, a1);
        let r2 = accent_x_runs(&rgb, w, h, a2);
        assert_eq!(r0.len(), 4, "S1 four stacked slots, got {r0:?}");
        assert_eq!(r1.len(), 4, "S2 four stacked slots, got {r1:?}");
        assert_eq!(r2.len(), 4, "S3 four stacked slots, got {r2:?}");
        for i in 0..4 {
            let overlap01 = r0[i].0 <= r1[i].1 && r1[i].0 <= r0[i].1;
            let overlap12 = r1[i].0 <= r2[i].1 && r2[i].0 <= r1[i].1;
            assert!(
                overlap01 && overlap12,
                "official stacked columns share category slots; {r0:?} {r1:?} {r2:?}"
            );
        }

        let ctrl_doc = crate::xml::parse(clustered_three_xml(&series).as_bytes()).unwrap();
        let ctrl = read_chart(&ctrl_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(ctrl.stacking, Stacking::Clustered);
        let ctrl_png = render_rgb_chart(
            &ctrl_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (cw, ch, crgb) = png_rgb(&ctrl_png);
        let c0 = accent_x_runs(&crgb, cw, ch, a0);
        assert_eq!(c0.len(), 4);
        let stacked_w: i32 = r0.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        let cluster_w: i32 = c0.iter().map(|(s, e)| (*e - *s) as i32 + 1).sum();
        assert!(
            stacked_w > cluster_w,
            "overlap=100 stacked bars are one-series wide; stacked={stacked_w} clustered={cluster_w}"
        );

        // S1 sits on the zero baseline; S3 is farther from it (on top of the stack).
        let mid_x = (r0[0].0 + r0[0].1) / 2;
        let ys = |accent: [u8; 3]| {
            (0..h)
                .filter(|&y| {
                    let i = (y as usize * w as usize + mid_x as usize) * 3;
                    near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 16)
                })
                .collect::<Vec<_>>()
        };
        let y1 = ys(a0);
        let y3 = ys(a2);
        assert!(!y1.is_empty() && !y3.is_empty());
        let mean = |v: &Vec<u32>| v.iter().sum::<u32>() as f64 / v.len() as f64;
        assert!(
            mean(&y1) > mean(&y3),
            "S1 is at the base (higher y), S3 on top; S1={:?} S3={:?}",
            y1,
            y3
        );

        let pct_xml = stacked_xml("col", "percentStacked", &series, false, false, false);
        let pct_doc = crate::xml::parse(pct_xml.as_bytes()).unwrap();
        let pct = read_chart(&pct_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(pct.stacking, Stacking::PercentStacked);
        assert_eq!(pct.axis.min, Some(0.0));
        assert_eq!(pct.axis.max, Some(1.0));
        assert_eq!(pct.axis.tick_format, super::axis::TickFormat::Percent);
        let pct_png = render_rgb_chart(
            &pct_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (pw, ph, prgb) = png_rgb(&pct_png);
        let tops: Vec<u32> = (0..4)
            .map(|i| {
                let pr = accent_x_runs(&prgb, pw, ph, a0);
                let mid = (pr[i].0 + pr[i].1) / 2;
                (0..ph)
                    .filter(|&y| {
                        let idx = (y as usize * pw as usize + mid as usize) * 3;
                        near([prgb[idx], prgb[idx + 1], prgb[idx + 2]], a0, 16)
                            || near([prgb[idx], prgb[idx + 1], prgb[idx + 2]], a1, 16)
                            || near([prgb[idx], prgb[idx + 1], prgb[idx + 2]], a2, 16)
                    })
                    .min()
                    .unwrap()
            })
            .collect();
        let span = tops.iter().max().unwrap() - tops.iter().min().unwrap();
        assert!(
            span <= 3,
            "percentStacked all-positive stacks reach the same 100% top; tops={tops:?}"
        );

        render_rgb_chart(
            &crate::xml::parse(stacked_xml("bar", "stacked", &series, false, false, false).as_bytes())
                .unwrap()
                .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        render_rgb_chart(
            &crate::xml::parse(
                stacked_xml("bar", "percentStacked", &series, false, false, false).as_bytes(),
            )
            .unwrap()
            .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();

        // Mixed signs: positives stack up from zero, negatives down, independently.
        let m1 = [Some(10.0), Some(20.0), Some(-30.0), Some(40.0)];
        let m2 = [Some(20.0), Some(-10.0), Some(20.0), Some(-10.0)];
        let m3 = [Some(-20.0), Some(30.0), Some(10.0), Some(20.0)];
        let mixed: [&[Option<f64>]; 3] = [&m1, &m2, &m3];
        let mix_xml = stacked_xml("col", "stacked", &mixed, false, false, false)
            .replace(r#"min val="0""#, r#"min val="-40""#);
        let mix_doc = crate::xml::parse(mix_xml.as_bytes()).unwrap();
        read_chart(&mix_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mix_png = render_rgb_chart(
            &mix_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (mw, mh, mrgb) = png_rgb(&mix_png);
        let mr0 = accent_x_runs(&mrgb, mw, mh, a0);
        assert_eq!(mr0.len(), 4, "mixed S1 still occupies four categories, {mr0:?}");

        // Blank S2 in category B: S3 packs onto S1 (no reserved hole).
        let b2 = [Some(20.0), None, Some(20.0), Some(10.0)];
        let blank: [&[Option<f64>]; 3] = [&s1, &b2, &s3];
        let blank_png = render_rgb_chart(
            &crate::xml::parse(stacked_xml("col", "stacked", &blank, false, false, false).as_bytes())
                .unwrap()
                .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (bw, bh, brgb) = png_rgb(&blank_png);
        let br1 = accent_x_runs(&brgb, bw, bh, a1);
        assert_eq!(br1.len(), 3, "missing S2 in B is not drawn, got {br1:?}");

        // Official F22-6 Chart.Export gold exists, so omitted min/max/major
        // on stacked columns must produce an image. Auto bounds reuse
        // scale_values on stacked extents (official 0-80 / 10), not raw
        // point values, and are close-enough rather than a unique lock.
        let auto_doc = crate::xml::parse(stacked_xml("col", "stacked", &series, true, false, false).as_bytes())
            .unwrap();
        let auto_chart = read_chart(&auto_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("omitted min/max on stacked must export");
        assert!(auto_chart.axis.min.is_none() && auto_chart.axis.max.is_none());
        let auto_png = render_rgb_chart(
            &auto_doc.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("omitted min/max stacked must produce an image");
        let (aw, ah, argb) = png_rgb(&auto_png);
        let auto_r0 = accent_x_runs(&argb, aw, ah, a0);
        let auto_r1 = accent_x_runs(&argb, aw, ah, a1);
        let auto_r2 = accent_x_runs(&argb, aw, ah, a2);
        assert_eq!(auto_r0.len(), 4, "auto stacked S1 four slots, got {auto_r0:?}");
        assert_eq!(auto_r1.len(), 4, "auto stacked S2 four slots, got {auto_r1:?}");
        assert_eq!(auto_r2.len(), 4, "auto stacked S3 four slots, got {auto_r2:?}");
        let h_grids = {
            let mut n = 0u32;
            for y in 0..ah {
                let mut ink = 0u32;
                for x in aw / 8..(aw * 3) / 4 {
                    let i = (y as usize * aw as usize + x as usize) * 3;
                    if argb[i] < 40 && argb[i + 1] < 40 && argb[i + 2] < 40 {
                        ink += 1;
                    }
                }
                if ink >= aw / 5 {
                    n += 1;
                }
            }
            n
        };
        // Official auto is 0-80 major 10 (8 intervals). Explicit 0-80 / 20
        // has fewer grids. Raw point max 40 would not use an 80 range.
        assert!(
            h_grids >= 8,
            "stacked auto should use stacked extents (0-80 / 10), grids={h_grids}"
        );
        let inv = crate::xml::parse(stacked_xml("col", "stacked", &series, false, true, false).as_bytes())
            .unwrap();
        assert!(
            read_chart(&inv.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "invertIfNegative=1 on stacked stays NI"
        );
        let labs = crate::xml::parse(stacked_xml("col", "stacked", &series, false, false, true).as_bytes())
            .unwrap();
        assert!(
            read_chart(&labs.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "value labels plus stacked stay NI"
        );
        let pct_mixed = crate::xml::parse(stacked_xml("col", "percentStacked", &mixed, false, false, false).as_bytes())
            .unwrap();
        assert!(
            read_chart(&pct_mixed.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "percentStacked mixed signs stay NI"
        );
        let no_overlap = stacked_xml("col", "stacked", &series, false, false, false)
            .replacen(r#"<c:overlap val="100"/>"#, "", 1);
        let no_over_doc = crate::xml::parse(no_overlap.as_bytes()).unwrap();
        assert!(
            read_chart(&no_over_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked without official overlap=100 stays NI"
        );
    }

    fn line_area_stacked_xml(
        kind: &str,
        grouping: &str,
        series: &[&[Option<f64>]],
        auto: bool,
        min: &str,
        max: &str,
        midcat: bool,
        marker_none: bool,
    ) -> String {
        let n = series[0].len();
        let cats = (0..n)
            .map(|i| format!(r#"<c:pt idx="{i}"><c:v>{}</c:v></c:pt>"#, (b'A' + i as u8) as char))
            .collect::<String>();
        let mut sers = String::new();
        for (si, vals) in series.iter().enumerate() {
            let pts = vals
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.map(|x| format!(r#"<c:pt idx="{i}"><c:v>{x}</c:v></c:pt>"#))
                })
                .collect::<String>();
            let present = vals.iter().filter(|v| v.is_some()).count();
            let marker = if kind == "lineChart" && marker_none {
                r#"<c:marker><c:symbol val="none"/></c:marker>"#
            } else {
                ""
            };
            let smooth = if kind == "lineChart" {
                r#"<c:smooth val="0"/>"#
            } else {
                ""
            };
            sers.push_str(&format!(
                r#"<c:ser><c:idx val="{si}"/><c:order val="{si}"/><c:tx><c:v>S{si}</c:v></c:tx>{marker}{smooth}<c:cat><c:strLit><c:ptCount val="{n}"/>{cats}</c:strLit></c:cat><c:val><c:numLit><c:ptCount val="{present}"/>{pts}</c:numLit></c:val></c:ser>"#
            ));
        }
        let (major, numfmt) = if grouping == "percentStacked" {
            ("0.2", r#"<c:numFmt formatCode="0%" sourceLinked="1"/>"#)
        } else {
            ("20", "")
        };
        let scaling = if auto {
            r#"<c:scaling><c:orientation val="minMax"/></c:scaling>"#.to_string()
        } else {
            format!(
                r#"<c:scaling><c:orientation val="minMax"/><c:min val="{min}"/><c:max val="{max}"/></c:scaling>"#
            )
        };
        let major_el = if auto {
            String::new()
        } else {
            format!(r#"<c:majorUnit val="{major}"/>"#)
        };
        let cross = if midcat { "midCat" } else { "between" };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart>
    <c:plotArea>
      <c:{kind}>
        <c:grouping val="{grouping}"/><c:varyColors val="0"/>
        {sers}
        <c:axId val="1"/><c:axId val="2"/>
      </c:{kind}>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/>{scaling}{major_el}<c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/>{numfmt}<c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="{cross}"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn accent_pixels(rgb: &[u8], w: u32, h: u32, accent: [u8; 3]) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = (y as usize * w as usize + x as usize) * 3;
                if near([rgb[i], rgb[i + 1], rgb[i + 2]], accent, 20) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    fn polylines_match(a: &[(u32, u32)], b: &[(u32, u32)]) -> bool {
        if a.len() < 40 || b.len() < 40 {
            return false;
        }
        let close = |pts: &[(u32, u32)], other: &[(u32, u32)]| {
            let mut hit = 0usize;
            for &(x, y) in pts {
                if other.iter().any(|&(ox, oy)| x.abs_diff(ox) <= 2 && y.abs_diff(oy) <= 2) {
                    hit += 1;
                }
            }
            hit * 100 / pts.len() >= 80
        };
        close(a, b) && close(b, a)
    }

    fn area_column_bands(rgb: &[u8], w: u32, h: u32, x: u32, accents: [[u8; 3]; 3]) -> Vec<(usize, u32, u32)> {
        let mut bands = Vec::new();
        let mut cur: Option<usize> = None;
        let mut y0 = 0u32;
        for y in 0..h {
            let i = (y as usize * w as usize + x as usize) * 3;
            let px = [rgb[i], rgb[i + 1], rgb[i + 2]];
            let hit = accents.iter().position(|a| near(px, *a, 24));
            if hit != cur {
                if let Some(idx) = cur {
                    bands.push((idx, y0, y.saturating_sub(1)));
                }
                cur = hit;
                y0 = y;
            }
        }
        if let Some(idx) = cur {
            bands.push((idx, y0, h.saturating_sub(1)));
        }
        bands
    }

    fn leftmost_area_column(rgb: &[u8], w: u32, h: u32, accents: [[u8; 3]; 3]) -> u32 {
        for x in 0..w {
            let mut seen = [false; 3];
            for y in 0..h {
                let i = (y as usize * w as usize + x as usize) * 3;
                let px = [rgb[i], rgb[i + 1], rgb[i + 2]];
                if let Some(idx) = accents.iter().position(|a| near(px, *a, 24)) {
                    seen[idx] = true;
                }
            }
            if seen.iter().filter(|v| **v).count() >= 2 {
                return x;
            }
        }
        panic!("no stacked area column with at least two series");
    }

    #[test]
    fn stacked_and_percent_stacked_lines_and_areas_use_algebraic_cumulative() {
        let s1 = [Some(10.0), Some(20.0), Some(30.0), Some(40.0)];
        let s2 = [Some(20.0), Some(10.0), Some(20.0), Some(10.0)];
        let s3 = [Some(20.0), Some(30.0), Some(10.0), Some(20.0)];
        let series: [&[Option<f64>]; 3] = [&s1, &s2, &s3];
        let a0 = FALLBACK_ACCENTS[0];
        let a1 = FALLBACK_ACCENTS[1];
        let a2 = FALLBACK_ACCENTS[2];

        let dummy = Series {
            values: s1.to_vec(),
            ..Series::default()
        };
        let dummy2 = Series {
            values: s2.to_vec(),
            ..Series::default()
        };
        let dummy3 = Series {
            values: s3.to_vec(),
            ..Series::default()
        };
        let group = [&dummy, &dummy2, &dummy3];
        assert_eq!(stack_cumulative(&group, 0, 0, false), Some((0.0, 10.0)));
        assert_eq!(stack_cumulative(&group, 0, 1, false), Some((10.0, 30.0)));
        assert_eq!(stack_cumulative(&group, 0, 2, false), Some((30.0, 50.0)));
        assert_eq!(stack_cumulative(&group, 3, 2, false), Some((50.0, 70.0)));
        let p0 = stack_cumulative(&group, 0, 0, true).unwrap();
        let p2 = stack_cumulative(&group, 0, 2, true).unwrap();
        assert!((p0.0 - 0.0).abs() < 1e-12 && (p0.1 - 0.2).abs() < 1e-12);
        assert!((p2.0 - 0.6).abs() < 1e-12 && (p2.1 - 1.0).abs() < 1e-12);

        let m1v = [Some(10.0), Some(20.0), Some(-30.0), Some(40.0)];
        let m2v = [Some(20.0), Some(-10.0), Some(20.0), Some(-10.0)];
        let m3v = [Some(-20.0), Some(30.0), Some(10.0), Some(20.0)];
        let mix1 = Series {
            values: m1v.to_vec(),
            ..Series::default()
        };
        let mix2 = Series {
            values: m2v.to_vec(),
            ..Series::default()
        };
        let mix3 = Series {
            values: m3v.to_vec(),
            ..Series::default()
        };
        let mixed_group = [&mix1, &mix2, &mix3];
        // Algebraic, not F22 pos/neg packing: cat A S3 ends at 10, not -20.
        assert_eq!(stack_cumulative(&mixed_group, 0, 2, false), Some((30.0, 10.0)));
        assert_eq!(stack_cumulative(&mixed_group, 2, 2, false), Some((-10.0, 0.0)));
        assert_eq!(stack_cumulative(&mixed_group, 3, 2, false), Some((30.0, 50.0)));

        let line_xml = line_area_stacked_xml("lineChart", "stacked", &series, false, "0", "80", false, true);
        let line_doc = crate::xml::parse(line_xml.as_bytes()).unwrap();
        let line = read_chart(&line_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(line.stacking, Stacking::Stacked);
        assert!(matches!(line.kind, PlotKind::StandardLine));
        let line_png = render_rgb_chart(
            &line_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (lw, lh, lrgb) = png_rgb(&line_png);

        let s2_end = [Some(30.0), Some(30.0), Some(50.0), Some(50.0)];
        let s3_end = [Some(50.0), Some(60.0), Some(60.0), Some(70.0)];
        let ends: [&[Option<f64>]; 3] = [&s1, &s2_end, &s3_end];
        let ctrl_xml = line_area_stacked_xml("lineChart", "standard", &ends, false, "0", "80", false, true);
        let ctrl_doc = crate::xml::parse(ctrl_xml.as_bytes()).unwrap();
        let ctrl = read_chart(&ctrl_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(ctrl.stacking, Stacking::Clustered);
        let ctrl_png = render_rgb_chart(
            &ctrl_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (_, _, crgb) = png_rgb(&ctrl_png);
        for (i, accent) in [a0, a1, a2].into_iter().enumerate() {
            let got = accent_pixels(&lrgb, lw, lh, accent);
            let exp = accent_pixels(&crgb, lw, lh, accent);
            assert!(
                polylines_match(&got, &exp),
                "stacked line S{i} should match a standard line of algebraic ends; stacked={} control={}",
                got.len(),
                exp.len()
            );
        }

        let pct_xml = line_area_stacked_xml(
            "lineChart",
            "percentStacked",
            &series,
            false,
            "0",
            "1",
            false,
            true,
        );
        let pct_doc = crate::xml::parse(pct_xml.as_bytes()).unwrap();
        let pct = read_chart(&pct_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(pct.stacking, Stacking::PercentStacked);
        assert_eq!(pct.axis.tick_format, super::axis::TickFormat::Percent);
        let pct_png = render_rgb_chart(
            &pct_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (pw, ph, prgb) = png_rgb(&pct_png);
        let p1 = [Some(0.2), Some(20.0 / 60.0), Some(0.5), Some(40.0 / 70.0)];
        let p2 = [Some(0.6), Some(0.5), Some(50.0 / 60.0), Some(50.0 / 70.0)];
        let p3 = [Some(1.0), Some(1.0), Some(1.0), Some(1.0)];
        let pct_ends: [&[Option<f64>]; 3] = [&p1, &p2, &p3];
        let pct_ctrl_xml = line_area_stacked_xml(
            "lineChart",
            "standard",
            &pct_ends,
            false,
            "0",
            "1",
            false,
            true,
        );
        let pct_ctrl_png = render_rgb_chart(
            &crate::xml::parse(pct_ctrl_xml.as_bytes()).unwrap().root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (_, _, pcrgb) = png_rgb(&pct_ctrl_png);
        let got3 = accent_pixels(&prgb, pw, ph, a2);
        let exp3 = accent_pixels(&pcrgb, pw, ph, a2);
        assert!(
            polylines_match(&got3, &exp3),
            "percentStacked line S3 should sit on the 100% cumulative, stacked={} control={}",
            got3.len(),
            exp3.len()
        );

        let area_xml = line_area_stacked_xml("areaChart", "stacked", &series, false, "0", "80", true, false);
        let area_doc = crate::xml::parse(area_xml.as_bytes()).unwrap();
        let area = read_chart(&area_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(area.stacking, Stacking::Stacked);
        assert!(matches!(area.kind, PlotKind::StandardArea));
        let area_png = render_rgb_chart(
            &area_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (aw, ah, argb) = png_rgb(&area_png);
        let x0 = leftmost_area_column(&argb, aw, ah, [a0, a1, a2]);
        let bands = area_column_bands(&argb, aw, ah, x0, [a0, a1, a2]);
        let stacked_bands: Vec<_> = bands.into_iter().filter(|(_idx, y0, y1)| *y1 > *y0 + 1).collect();
        assert!(
            stacked_bands.len() >= 3,
            "stacked area cat A should show S3/S2/S1 bands, got {stacked_bands:?} x={x0}"
        );
        assert_eq!(stacked_bands[0].0, 2, "top band is S3, {stacked_bands:?}");
        assert_eq!(stacked_bands[1].0, 1, "middle band is S2, {stacked_bands:?}");
        assert_eq!(stacked_bands[2].0, 0, "bottom band is S1, {stacked_bands:?}");
        let h1 = stacked_bands[2].2 - stacked_bands[2].1 + 1;
        let h2 = stacked_bands[1].2 - stacked_bands[1].1 + 1;
        let h3 = stacked_bands[0].2 - stacked_bands[0].1 + 1;
        assert!(
            (h2 as i32 - h3 as i32).abs() <= 3 && (h1 * 2 + 3) >= h2 && h1 + 3 >= h2 / 2,
            "area cat A heights should be ~10/20/20, got {h1},{h2},{h3}"
        );

        let area_pct_xml = line_area_stacked_xml(
            "areaChart",
            "percentStacked",
            &series,
            false,
            "0",
            "1",
            true,
            false,
        );
        let area_pct_doc = crate::xml::parse(area_pct_xml.as_bytes()).unwrap();
        let area_pct =
            read_chart(&area_pct_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        assert_eq!(area_pct.stacking, Stacking::PercentStacked);
        let area_pct_png = render_rgb_chart(
            &area_pct_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (apw, aph, aprgb) = png_rgb(&area_pct_png);
        let apx = leftmost_area_column(&aprgb, apw, aph, [a0, a1, a2]);
        let ap_bands = area_column_bands(&aprgb, apw, aph, apx, [a0, a1, a2]);
        let ap_stack: Vec<_> = ap_bands.into_iter().filter(|(_, y0, y1)| *y1 > *y0 + 1).collect();
        assert!(ap_stack.len() >= 3, "percent area bands {ap_stack:?}");
        assert_eq!(ap_stack[0].0, 2);
        assert_eq!(ap_stack[ap_stack.len() - 1].0, 0);

        let mixed: [&[Option<f64>]; 3] = [&m1v, &m2v, &m3v];
        let mix_xml = line_area_stacked_xml("lineChart", "stacked", &mixed, false, "-40", "80", false, true);
        let mix_doc = crate::xml::parse(mix_xml.as_bytes()).unwrap();
        read_chart(&mix_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).unwrap();
        let mix_png = render_rgb_chart(
            &mix_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (mw, mh, mrgb) = png_rgb(&mix_png);
        let mix_s1 = [Some(10.0), Some(20.0), Some(-30.0), Some(40.0)];
        let mix_s2 = [Some(30.0), Some(10.0), Some(-10.0), Some(30.0)];
        let mix_s3 = [Some(10.0), Some(40.0), Some(0.0), Some(50.0)];
        let mix_ends: [&[Option<f64>]; 3] = [&mix_s1, &mix_s2, &mix_s3];
        let mix_ctrl_png = render_rgb_chart(
            &crate::xml::parse(
                line_area_stacked_xml("lineChart", "standard", &mix_ends, false, "-40", "80", false, true)
                    .as_bytes(),
            )
            .unwrap()
            .root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .unwrap();
        let (_, _, mcrgb) = png_rgb(&mix_ctrl_png);
        let got_m3 = accent_pixels(&mrgb, mw, mh, a2);
        let exp_m3 = accent_pixels(&mcrgb, mw, mh, a2);
        assert!(
            polylines_match(&got_m3, &exp_m3),
            "mixed stacked line is algebraic (not F22 pos/neg pack); stacked={} control={}",
            got_m3.len(),
            exp_m3.len()
        );

        let mix_area_xml =
            line_area_stacked_xml("areaChart", "stacked", &mixed, false, "-40", "80", true, false);
        let mix_area_doc = crate::xml::parse(mix_area_xml.as_bytes()).unwrap();
        read_chart(
            &mix_area_doc.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .unwrap();
        let mix_area_png = render_rgb_chart(
            &mix_area_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "",
        )
        .unwrap();
        let (maw, mah, margb) = png_rgb(&mix_area_png);
        let xa = leftmost_area_column(&margb, maw, mah, [a0, a1, a2]);
        let mix_bands = area_column_bands(&margb, maw, mah, xa, [a0, a1, a2]);
        let mix_stack: Vec<_> = mix_bands.into_iter().filter(|(_, y0, y1)| *y1 > *y0 + 1).collect();
        assert!(
            mix_stack.iter().any(|(idx, _, _)| *idx == 0)
                && mix_stack.iter().any(|(idx, _, _)| *idx == 2),
            "mixed area cat A shows S1 and S3, got {mix_stack:?}"
        );
        assert!(
            !mix_stack.iter().any(|(idx, _, _)| *idx == 1),
            "mixed area cat A S3 covers the S2 band, got {mix_stack:?}"
        );

        // Official F23-7 Chart.Export gold exists, so omitted min/max/major
        // on a stacked line must produce an image. Auto bounds reuse
        // scale_values on algebraic cumulative extents (official 0-80 / 10),
        // not raw point values, and are close-enough rather than a unique
        // lock. The same general rule is allowed on stacked area; official
        // gold in this folder is the line case only.
        let auto = crate::xml::parse(
            line_area_stacked_xml("lineChart", "stacked", &series, true, "0", "80", false, true)
                .as_bytes(),
        )
        .unwrap();
        let auto_chart = read_chart(&auto.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("omitted min/max on stacked line must export");
        assert!(auto_chart.axis.min.is_none() && auto_chart.axis.max.is_none());
        let auto_png = render_rgb_chart(
            &auto.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("omitted min/max stacked line must produce an image");
        let (aw, ah, argb) = png_rgb(&auto_png);
        let auto_pts = accent_pixels(&argb, aw, ah, a2);
        assert!(
            !auto_pts.is_empty(),
            "auto stacked line should draw S3 ink"
        );
        let h_grids = {
            let mut n = 0u32;
            for y in 0..ah {
                let mut ink = 0u32;
                for x in aw / 8..(aw * 3) / 4 {
                    let i = (y as usize * aw as usize + x as usize) * 3;
                    if argb[i] < 40 && argb[i + 1] < 40 && argb[i + 2] < 40 {
                        ink += 1;
                    }
                }
                if ink >= aw / 5 {
                    n += 1;
                }
            }
            n
        };
        // Official auto is 0-80 major 10 (8 intervals). Explicit 0-80 / 20
        // has fewer grids. Raw point max 40 would not use an 80 range.
        assert!(
            h_grids >= 8,
            "stacked line auto should use cumulative extents (0-80 / 10), grids={h_grids}"
        );
        let auto_area = crate::xml::parse(
            line_area_stacked_xml("areaChart", "stacked", &series, true, "0", "80", true, false)
                .as_bytes(),
        )
        .unwrap();
        read_chart(
            &auto_area.root,
            FALLBACK_ACCENTS,
            None,
            "",
            RenderDpi::DEFAULT,
        )
        .expect("omitted min/max on stacked area must export");
        render_rgb_chart(
            &auto_area.root,
            424,
            265,
            FALLBACK_ACCENTS,
            None,
            ChartHost::Excel,
            None,
            "",
        )
        .expect("omitted min/max stacked area must produce an image");
        let between_area = crate::xml::parse(
            line_area_stacked_xml("areaChart", "stacked", &series, false, "0", "80", false, false)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&between_area.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked area with crossBetween=between stays NI"
        );
        let marked = crate::xml::parse(
            line_area_stacked_xml("lineChart", "stacked", &series, false, "0", "80", false, false)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&marked.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked line without official marker=none stays NI"
        );
        let blank_s2 = [Some(20.0), None, Some(20.0), Some(10.0)];
        let blank: [&[Option<f64>]; 3] = [&s1, &blank_s2, &s3];
        let blank_doc = crate::xml::parse(
            line_area_stacked_xml("lineChart", "stacked", &blank, false, "0", "80", false, true)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&blank_doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "stacked line with a missing cache point stays NI"
        );
        let pct_mixed = crate::xml::parse(
            line_area_stacked_xml("lineChart", "percentStacked", &mixed, false, "0", "1", false, true)
                .as_bytes(),
        )
        .unwrap();
        assert!(
            read_chart(&pct_mixed.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT).is_err(),
            "percentStacked mixed signs on a line stay NI"
        );
    }

    fn titled_legend_xml(def_rpr: &str) -> String {
        let title = format!(
            "<c:title><c:tx><c:rich xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><a:bodyPr/><a:lstStyle/><a:p><a:pPr>{def}</a:pPr><a:r><a:rPr lang=\"en-US\"/><a:t>Legend foundation</a:t></a:r></a:p></c:rich></c:tx><c:overlay val=\"0\"/></c:title>",
            def = def_rpr
        );
        LONG_LEGEND.replace("<c:chart>", &format!("<c:chart>{title}"))
    }

    fn read_titled(def_rpr: &str) -> Result<ColumnChart, Error> {
        let doc = crate::xml::parse(titled_legend_xml(def_rpr).as_bytes()).unwrap();
        read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
    }

    #[test]
    fn omitted_title_typography_stays_legacy_and_does_not_invent_a_point_size() {
        let chart = read_titled("<a:defRPr/>").expect("empty defRPr remains renderable");
        assert_eq!(chart.title.as_deref(), Some("Legend foundation"));
        assert_eq!(chart.title_typo, None, "do not invent 10pt or 18pt when sz is omitted");
        // Language-only run properties are not typography either.
        let chart = read_titled("<a:defRPr></a:defRPr>").expect("empty defRPr element");
        assert_eq!(chart.title_typo, None);
    }

    #[test]
    fn explicit_title_sz_is_hundredths_of_a_point() {
        let typo = read_titled("<a:defRPr sz='1800' b='1'/>")
            .expect("explicit sz is supported")
            .title_typo
            .expect("sz must lock typography");
        assert_eq!(typo.pt, 18.0);
        assert!(typo.bold);
        assert!(!typo.italic);
        assert_eq!(typo.name, None);
        let ten = read_titled("<a:defRPr sz='1000'/>")
            .unwrap()
            .title_typo
            .unwrap();
        assert_eq!(ten.pt, 10.0);
        assert!(!ten.bold, "omitted b on an explicit sz is not assumed bold");
    }

    #[test]
    fn explicit_title_typeface_parses_with_size() {
        let typo = read_titled("<a:defRPr sz='1400' i='1'><a:latin typeface='Arial'/></a:defRPr>")
            .expect("named typeface with sz")
            .title_typo
            .unwrap();
        assert_eq!(typo.pt, 14.0);
        assert!(typo.italic);
        assert!(!typo.bold);
        assert_eq!(typo.name.as_deref(), Some("Arial"));
    }

    #[test]
    fn unsupported_or_incomplete_title_typography_stays_ni() {
        for def in [
            "<a:defRPr sz='1800' u='sng'/>",
            "<a:defRPr sz='NaN'/>",
            "<a:defRPr sz='0'/>",
            "<a:defRPr sz='50000'/>",
            "<a:defRPr b='1'/>",
            "<a:defRPr><a:latin typeface='Arial'/></a:defRPr>",
            "<a:defRPr sz='1800'><a:latin typeface='+mj-lt'/></a:defRPr>",
        ] {
            assert!(
                read_titled(def).is_err(),
                "must not silently drop typography semantics: {def}"
            );
        }
    }

    #[test]
    fn explicit_title_sz_changes_rendered_title_metrics_at_export_dpi() {
        let small_xml = titled_legend_xml("<a:defRPr sz='1000' b='1'/>");
        let large_xml = titled_legend_xml("<a:defRPr sz='1800' b='1'/>");
        let small_doc = crate::xml::parse(small_xml.as_bytes()).unwrap();
        let large_doc = crate::xml::parse(large_xml.as_bytes()).unwrap();
        let dpi = RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap();
        let small = render_chart_at(
            &small_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi,
        )
        .unwrap();
        let large = render_chart_at(
            &large_doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "", dpi,
        )
        .unwrap();
        assert_ne!(small, large, "10pt and 18pt titles must not share a raster");
        assert!(large.len() >= 8 && small.len() >= 8);
    }

    fn two_paragraph_title_xml() -> String {
        let title = concat!(
            "<c:title><c:tx><c:rich xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
            "<a:bodyPr/><a:lstStyle/>",
            "<a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:rPr lang=\"en-US\"/><a:t>Line one</a:t></a:r></a:p>",
            "<a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:rPr lang=\"en-US\"/><a:t>Line two</a:t></a:r></a:p>",
            "</c:rich></c:tx><c:overlay val=\"0\"/></c:title>",
        );
        LONG_LEGEND.replace("<c:chart>", &format!("<c:chart>{title}"))
    }

    #[test]
    fn title_paragraphs_and_breaks_become_hard_newlines() {
        let doc = crate::xml::parse(two_paragraph_title_xml().as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("multi-paragraph titles are renderable hard breaks");
        assert_eq!(chart.title.as_deref(), Some("Line one\nLine two"));
        assert_eq!(chart.title_typo, None);

        let br = titled_legend_xml("<a:defRPr/>").replace(
            "<a:t>Legend foundation</a:t>",
            "<a:t>Legend</a:t></a:r><a:br/><a:r><a:rPr lang=\"en-US\"/><a:t>foundation</a:t>",
        );
        let doc = crate::xml::parse(br.as_bytes()).unwrap();
        let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
            .expect("a:br is a hard title break");
        assert_eq!(chart.title.as_deref(), Some("Legend\nfoundation"));
    }

    #[test]
    fn title_wrap_keeps_short_lines_and_splits_when_wider_than_the_box() {
        let short = wrap_title_lines("Legend foundation", None, None, 13.0, 400.0);
        assert_eq!(short, vec!["Legend foundation".to_string()]);
        let hard = wrap_title_lines("Alpha\nBeta", None, None, 13.0, 400.0);
        assert_eq!(hard, vec!["Alpha".to_string(), "Beta".to_string()]);
        let long = "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM";
        let wrapped = wrap_title_lines(long, None, None, 20.0, 80.0);
        assert!(
            wrapped.len() >= 2,
            "a string wider than the title box must wrap: {wrapped:?}"
        );
        assert!(wrapped.iter().all(|line| !line.is_empty()));
        assert_eq!(wrapped.join(""), long);

        let y1 = titled_plot_y(
            RenderDpi::DEFAULT,
            ChartHost::Excel,
            None,
            None,
            None,
            13.0,
            1,
        );
        let y2 = titled_plot_y(
            RenderDpi::DEFAULT,
            ChartHost::Excel,
            None,
            None,
            None,
            13.0,
            2,
        );
        assert!(y2 > y1, "wrapped titles must reserve a taller plot offset");
        let title_top = RenderDpi::DEFAULT.scale_f(4.0);
        let band = text_height(None, 13.0).ceil() as i32;
        assert_eq!(
            y1,
            title_top as i32 + 2 * band,
            "one-line titles keep the previous plot offset"
        );
    }

    fn scheme_with_sys(pairs: &[(&str, [u8; 3])]) -> ThemeScheme {
        let mut scheme = theme_scheme(None, FALLBACK_ACCENTS);
        for (name, rgb) in pairs {
            scheme.sys.insert(*name, *rgb);
        }
        scheme
    }

    #[test]
    fn chart_frame_sysclr_uses_caller_palette_and_ignores_last_clr() {
        let scheme = scheme_with_sys(&[
            ("window", [255, 255, 255]),
            ("windowText", [0, 0, 0]),
        ]);
        let window = crate::xml::parse(b"<sysClr val='window' lastClr='00FF00'/>").unwrap();
        assert_eq!(
            resolve_frame_solid(&window.root, &scheme).unwrap(),
            FrameFill::Solid([255, 255, 255]),
            "live window is white; lastClr green is not current"
        );
        let text = crate::xml::parse(b"<sysClr val='windowText' lastClr='FF0000'/>").unwrap();
        assert_eq!(
            resolve_frame_solid(&text.root, &scheme).unwrap(),
            FrameFill::Solid([0, 0, 0]),
            "live windowText is black; lastClr red is not current"
        );
        // lastClr may coincide with the live color; the palette still wins.
        let coinc = crate::xml::parse(b"<sysClr val='windowText' lastClr='000000'/>").unwrap();
        assert_eq!(
            resolve_frame_solid(&coinc.root, &scheme).unwrap(),
            FrameFill::Solid([0, 0, 0])
        );
        let empty = theme_scheme(None, FALLBACK_ACCENTS);
        assert!(
            resolve_frame_solid(&window.root, &empty).is_err(),
            "without a palette, lastClr must not become a silent current color"
        );
        assert!(resolve_frame_solid(&text.root, &empty).is_err());
        let mismatch = crate::xml::parse(b"<sysClr val='Window' lastClr='FFFFFF'/>").unwrap();
        assert!(
            resolve_frame_solid(&mismatch.root, &scheme).is_err(),
            "val matching is exact; do not fold case or invent aliases"
        );
        let missing = crate::xml::parse(b"<sysClr lastClr='FFFFFF'/>").unwrap();
        assert!(resolve_frame_solid(&missing.root, &scheme).is_err());
        let unknown = crate::xml::parse(b"<sysClr val='captionText' lastClr='000000'/>").unwrap();
        assert!(
            resolve_frame_solid(&unknown.root, &scheme).is_err(),
            "unmapped names stay NI even when lastClr looks usable"
        );
        for xml in [
            "<sysClr val='window' lastClr='FFFFFF'><alpha val='50000'/></sysClr>",
            "<sysClr val='window' lastClr='FFFFFF'><tint val='50000'/></sysClr>",
            "<sysClr val='windowText' lastClr='FF0000'><red val='50000'/></sysClr>",
        ] {
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(
                resolve_frame_solid(&doc.root, &scheme).is_err(),
                "sysClr transforms were not in the official matrix: {xml}"
            );
        }
    }

    #[test]
    fn chart_and_plot_sysclr_rasters_match_palette_srgb_not_last_clr() {
        let mut palette = SystemColorPalette::default();
        palette
            .insert("window", [255, 255, 255])
            .insert("windowText", [0, 0, 0]);
        let dpi =
            RenderDpi::from_request(serde_json::json!({"dpi":106}).as_object().unwrap()).unwrap();
        let render = |fill: &str, sys: Option<&SystemColorPalette>, host| {
            let xml = LONG_LEGEND.replace("<c:chart>", &format!("{fill}<c:chart>"));
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            render_chart_with_sys_palette(
                &doc.root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                host,
                None,
                "",
                dpi,
                None,
                sys,
            )
        };
        let window_sys = "<c:spPr><a:solidFill><a:sysClr val='window' lastClr='00FF00'/></a:solidFill></c:spPr>";
        let window_srgb = "<c:spPr><a:solidFill><a:srgbClr val='FFFFFF'/></a:solidFill></c:spPr>";
        let text_sys = "<c:spPr><a:solidFill><a:sysClr val='windowText' lastClr='FF0000'/></a:solidFill></c:spPr>";
        let text_srgb = "<c:spPr><a:solidFill><a:srgbClr val='000000'/></a:solidFill></c:spPr>";
        let green_srgb = "<c:spPr><a:solidFill><a:srgbClr val='00FF00'/></a:solidFill></c:spPr>";
        let red_srgb = "<c:spPr><a:solidFill><a:srgbClr val='FF0000'/></a:solidFill></c:spPr>";
        assert!(
            render(window_sys, None, ChartHost::Excel).is_err(),
            "export without a palette must stay NI"
        );
        assert_eq!(
            render(window_sys, Some(&palette), ChartHost::Excel).unwrap(),
            render(window_srgb, None, ChartHost::Excel).unwrap(),
            "window lastClr green must paint palette white"
        );
        assert_eq!(
            render(text_sys, Some(&palette), ChartHost::Excel).unwrap(),
            render(text_srgb, None, ChartHost::Excel).unwrap(),
            "windowText lastClr red must paint palette black"
        );
        assert_ne!(
            render(window_sys, Some(&palette), ChartHost::Excel).unwrap(),
            render(green_srgb, None, ChartHost::Excel).unwrap()
        );
        assert_ne!(
            render(text_sys, Some(&palette), ChartHost::Excel).unwrap(),
            render(red_srgb, None, ChartHost::Excel).unwrap()
        );
        // Plot-area sysClr uses the same palette policy.
        let plot_sys = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:sysClr val='windowText' lastClr='FF0000'/></a:solidFill></c:spPr>",
            );
        let plot_srgb = LONG_LEGEND
            .replace(
                "<c:chart>",
                "<c:spPr><a:solidFill><a:srgbClr val='EAF7F7'/></a:solidFill></c:spPr><c:chart>",
            )
            .replace(
                "<c:plotArea>",
                "<c:plotArea><c:spPr><a:solidFill><a:srgbClr val='000000'/></a:solidFill></c:spPr>",
            );
        let plot_doc = crate::xml::parse(plot_sys.as_bytes()).unwrap();
        let plot_ref = crate::xml::parse(plot_srgb.as_bytes()).unwrap();
        let plot_png = |root: &Element, sys: Option<&SystemColorPalette>| {
            render_chart_with_sys_palette(
                root,
                424,
                265,
                FALLBACK_ACCENTS,
                None,
                ChartHost::Excel,
                None,
                "",
                dpi,
                None,
                sys,
            )
        };
        assert!(plot_png(&plot_doc.root, None).is_err());
        assert_eq!(
            plot_png(&plot_doc.root, Some(&palette)).unwrap(),
            plot_png(&plot_ref.root, None).unwrap()
        );
        // Slide host still has no chart-area frame, even with a palette.
        assert_eq!(
            render(window_sys, Some(&palette), ChartHost::Slide).unwrap(),
            render("", None, ChartHost::Slide).unwrap()
        );
    }

    #[test]
    fn theme_clrscheme_sysclr_lastclr_snapshot_is_unchanged() {
        // dk1/lt1 in the theme still use serialized lastClr, which is a
        // different path from live chart-frame sysClr.
        let theme = br#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme name="x"><a:dk1><a:sysClr val="windowText" lastClr="112233"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="AABBCC"/></a:lt1></a:clrScheme></a:themeElements></a:theme>"#;
        let scheme = theme_scheme(Some(theme), FALLBACK_ACCENTS);
        assert_eq!(scheme.dk1, Some([0x11, 0x22, 0x33]));
        assert_eq!(scheme.lt1, Some([0xAA, 0xBB, 0xCC]));
        assert!(scheme.sys.get("windowText").is_none());
        assert!(scheme.sys.get("window").is_none());
    }


    #[test]
    fn color_transform_working_space_matches_official_t187_probe_rgb() {
        // Official Chart.Export t187-frame-color-transforms-20261126, 8 srgbClr
        // bases, CLEAN_PUBLIC_OFFICE_EXPORT, probe (5,5). Competing models were
        // frozen before the session. Pure 2.2, encoded 255-c, and linear-IEC
        // gray were rejected by these pixels.
        let cases: [(&str, [u8; 3], [u8; 3], [u8; 3], [u8; 3], [u8; 3], [u8; 3]); 8] = [
            ("010101", [1, 1, 1], [1, 1, 1], [255, 255, 255], [13, 13, 13], [0, 0, 0], [1, 1, 1]),
            ("0A0A0A", [10, 10, 10], [10, 10, 10], [255, 255, 255], [56, 56, 56], [1, 1, 1], [10, 10, 10]),
            ("808080", [128, 128, 128], [128, 128, 128], [229, 229, 229], [188, 188, 188], [55, 55, 55], [128, 128, 128]),
            ("A0A0A0", [160, 160, 160], [160, 160, 160], [211, 211, 211], [208, 208, 208], [90, 90, 90], [160, 160, 160]),
            ("C0C0C0", [192, 192, 192], [192, 192, 192], [183, 183, 183], [225, 225, 225], [134, 134, 134], [192, 192, 192]),
            ("FF0000", [255, 0, 0], [0, 255, 255], [0, 255, 255], [255, 0, 0], [255, 0, 0], [54, 54, 54]),
            ("00FF00", [0, 255, 0], [255, 0, 255], [255, 0, 255], [0, 255, 0], [0, 255, 0], [182, 182, 182]),
            ("C04020", [192, 64, 32], [32, 160, 192], [183, 249, 253], [225, 137, 99], [134, 13, 4], [89, 89, 89]),
        ];
        for (base, none, comp, inv, gamma, inv_gamma, gray) in cases {
            let rgb = none;
            assert_eq!(rgb, none, "{base} control");
            assert_eq!(frame_comp(rgb), comp, "{base} comp");
            assert_eq!(frame_inv(rgb), inv, "{base} inv");
            assert_eq!(frame_gamma(rgb), gamma, "{base} gamma");
            assert_eq!(frame_inv_gamma(rgb), inv_gamma, "{base} invGamma");
            assert_eq!(frame_gray(rgb), gray, "{base} gray");
            // Reject the models the official matrix uniquely ruled out.
            if base == "808080" {
                assert_ne!(frame_inv(rgb), [127, 127, 127], "inv is not encoded 255-c");
                assert_ne!(frame_gamma(rgb), [186, 186, 186], "gamma is not pure 2.2");
                assert_ne!(frame_inv_gamma(rgb), [56, 56, 56], "invGamma is not pure 2.2");
            }
            if base == "FF0000" {
                assert_ne!(frame_gray(rgb), [76, 76, 76], "gray is not encoded Rec.601");
                assert_ne!(frame_gray(rgb), [53, 53, 53], "gray is not 54/183/19 >> 8");
                let lin = srgb_to_linear(255);
                let linear_709 = linear_to_srgb(0.2126 * lin);
                assert_ne!(frame_gray(rgb), [linear_709, linear_709, linear_709], "gray is not linear-IEC Rec.709");
            }
            if base == "C04020" {
                assert_ne!(frame_comp(rgb), [63, 191, 223], "comp is not encoded 255-c");
                assert_ne!(frame_comp(rgb), frame_inv(rgb), "comp is not inv");
            }
        }
    }

    #[test]
    fn color_transform_working_space_resolve_and_gates() {
        let scheme = theme_scheme(None, FALLBACK_ACCENTS);
        let ok = |xml: &str, rgb: [u8; 3]| {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(resolve_frame_solid(&d.root, &scheme).unwrap(), FrameFill::Solid(rgb), "{xml}");
        };
        let ni = |xml: &str| {
            let d = crate::xml::parse(xml.as_bytes()).unwrap();
            assert!(resolve_frame_solid(&d.root, &scheme).is_err(), "{xml}");
        };
        ok("<srgbClr val='C04020'><comp/></srgbClr>", [32, 160, 192]);
        ok("<srgbClr val='808080'><inv/></srgbClr>", [229, 229, 229]);
        ok("<srgbClr val='010101'><gamma/></srgbClr>", [13, 13, 13]);
        ok("<srgbClr val='C0C0C0'><invGamma/></srgbClr>", [134, 134, 134]);
        ok("<srgbClr val='FF0000'><gray/></srgbClr>", [54, 54, 54]);
        ok("<a:srgbClr xmlns:a='http://schemas.openxmlformats.org/drawingml/2006/main' val='00FF00'><a:gray/></a:srgbClr>", [182, 182, 182]);
        ok("<srgbClr val='C04020'><inv/><comp/></srgbClr>", [253, 187, 183]);
        ok("<srgbClr val='C04020'><comp/><inv/></srgbClr>", [253, 211, 183]);
        // Other bases, non-empty leaves, length-3 chains, and schemeClr stay NI.
        // No silent lastClr / scheme widening.
        for xml in [
            "<schemeClr val='accent1'><comp/></schemeClr>",
            "<sysClr val='window' lastClr='FFFFFF'><inv/></sysClr>",
            "<prstClr val='red'><gamma/></prstClr>",
            "<hslClr hue='0' sat='100000' lum='50000'><gray/></hslClr>",
            "<scrgbClr r='50000' g='50000' b='50000'><invGamma/></scrgbClr>",
            "<srgbClr val='808080'><comp val='1'/></srgbClr>",
            "<srgbClr val='808080'><inv><alpha val='50000'/></inv></srgbClr>",
            "<srgbClr val='808080'><gamma/><alpha val='100000'/></srgbClr>",
            "<srgbClr val='808080'><inv/><gamma/><gray/></srgbClr>",
            "<schemeClr val='accent1'><inv/><gamma/></schemeClr>",
        ] {
            ni(xml);
        }
    }

    #[test]
    fn color_transform_working_space_chart_frame_matches_official_rgb_reference() {
        // Compare against a solid srgbClr of the official probe RGB. Do not
        // use official 425 by 266 coordinates on the library 424 by 265 raster,
        // and do not compensate that size residual.
        let cases: [(&str, &str, &str); 8] = [
            ("comp", "C04020", "20A0C0"),
            ("inv", "808080", "E5E5E5"),
            ("gamma", "010101", "0D0D0D"),
            ("invGamma", "C0C0C0", "868686"),
            ("gray", "FF0000", "363636"),
            ("comp", "010101", "010101"),
            ("gamma", "C04020", "E18963"),
            ("gray", "C04020", "595959"),
        ];
        for (transform, base, official) in cases {
            let fill = format!("<c:spPr><a:solidFill><a:srgbClr val='{base}'><a:{transform}/></a:srgbClr></a:solidFill></c:spPr>");
            let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{official}'/></a:solidFill></c:spPr>");
            let make = |sp: &str| LONG_LEGEND.replace("<c:chart>", &format!("{sp}<c:chart>"));
            let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
            let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
            let render = |root: &Element| render_chart(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "").unwrap();
            assert_eq!(render(&doc.root), render(&reference.root), "{transform} {base}");
            let slide = crate::xml::parse(LONG_LEGEND.as_bytes()).unwrap();
            let with = render_chart(&doc.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Slide, None, "").unwrap();
            let without = render_chart(&slide.root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Slide, None, "").unwrap();
            assert_eq!(with, without, "slide host must not grow a chart-area frame for {transform}");
        }
    }
    #[test]
    fn srgb_transform_pairs_match_official_t189_xml_order() {
        // Official Chart.Export t189-srgb-transform-chains-20261126, 50 cases,
        // CLEAN_PUBLIC_OFFICE_EXPORT, probe (5,5). Competing models (XML order
        // of the already-locked helpers, reverse order, identity) were frozen
        // before the session. Reverse lost 17 order-discriminating pixels.
        // Five C04020 pairs are recorded as integer-roundtrip residuals below;
        // they are not a reason to pick reverse order or invent a fudge.
        let exact: [(&str, [&str; 2], [u8; 3]); 35] = [
            ("808080", ["inv", "gamma"], [243, 243, 243]),
            ("808080", ["gamma", "inv"], [187, 187, 187]),
            ("808080", ["gamma", "invGamma"], [128, 128, 128]),
            ("808080", ["invGamma", "gamma"], [128, 128, 128]),
            ("808080", ["inv", "comp"], [229, 229, 229]),
            ("808080", ["comp", "inv"], [229, 229, 229]),
            ("808080", ["inv", "gray"], [229, 229, 229]),
            ("808080", ["gray", "inv"], [229, 229, 229]),
            ("808080", ["comp", "gray"], [128, 128, 128]),
            ("808080", ["gray", "comp"], [128, 128, 128]),
            ("808080", ["gamma", "gray"], [188, 188, 188]),
            ("808080", ["gray", "gamma"], [188, 188, 188]),
            ("808080", ["inv", "inv"], [128, 128, 128]),
            ("808080", ["gamma", "gamma"], [223, 223, 223]),
            ("808080", ["comp", "comp"], [128, 128, 128]),
            ("808080", ["gray", "gray"], [128, 128, 128]),
            ("808080", ["invGamma", "inv"], [251, 251, 251]),
            ("808080", ["inv", "invGamma"], [200, 200, 200]),
            ("808080", ["comp", "gamma"], [188, 188, 188]),
            ("808080", ["gamma", "comp"], [188, 188, 188]),
            ("C04020", ["inv", "gamma"], [220, 252, 254]),
            ("C04020", ["gamma", "invGamma"], [192, 64, 32]),
            ("C04020", ["inv", "comp"], [253, 187, 183]),
            ("C04020", ["comp", "inv"], [253, 211, 183]),
            ("C04020", ["inv", "gray"], [235, 235, 235]),
            ("C04020", ["gray", "inv"], [243, 243, 243]),
            ("C04020", ["comp", "gray"], [135, 135, 135]),
            ("C04020", ["gray", "comp"], [89, 89, 89]),
            ("C04020", ["gamma", "gray"], [153, 153, 153]),
            ("C04020", ["gray", "gamma"], [159, 159, 159]),
            ("C04020", ["comp", "comp"], [192, 64, 32]),
            ("C04020", ["gray", "gray"], [89, 89, 89]),
            ("C04020", ["invGamma", "inv"], [226, 255, 255]),
            ("C04020", ["comp", "gamma"], [99, 208, 225]),
            ("C04020", ["gamma", "comp"], [99, 187, 225]),
        ];
        let scheme = theme_scheme(None, FALLBACK_ACCENTS);
        for (base, ops, official) in exact {
            let rgb = [
                u8::from_str_radix(&base[0..2], 16).unwrap(),
                u8::from_str_radix(&base[2..4], 16).unwrap(),
                u8::from_str_radix(&base[4..6], 16).unwrap(),
            ];
            let composed = apply_locked_srgb_transforms(rgb, &ops).unwrap();
            assert_eq!(composed, official, "xml-order {base} {ops:?}");
            let xml = format!(
                "<srgbClr val='{base}'><{a}/><{b}/></srgbClr>",
                a = ops[0],
                b = ops[1]
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(
                resolve_frame_solid(&doc.root, &scheme).unwrap(),
                FrameFill::Solid(official),
                "{xml}"
            );
            let reversed = apply_locked_srgb_transforms(rgb, &[ops[1], ops[0]]).unwrap();
            if reversed != composed {
                assert_ne!(reversed, official, "reverse must not win when order differs: {base} {ops:?}");
            }
        }
    }

    #[test]
    fn srgb_transform_pairs_record_t189_integer_roundtrip_residuals() {
        // These five official (5,5) pixels are closer to XML-order than to
        // reverse (or equal on commuting pairs), but they differ from
        // per-step rounded locked helpers by 1..4. Do not fudge, do not pick
        // reverse, and do not lower the exact official standard on the 35
        // exact cases. Library output is the composed locked helpers.
        let residuals: [(&str, [&str; 2], [u8; 3], [u8; 3], [u8; 3], u8); 5] = [
            ("C04020", ["gamma", "inv"], [136, 224, 240], [136, 225, 240], [220, 252, 254], 1),
            ("C04020", ["invGamma", "gamma"], [192, 64, 32], [192, 64, 34], [192, 64, 32], 2),
            ("C04020", ["inv", "inv"], [192, 64, 32], [192, 65, 36], [192, 65, 36], 4),
            ("C04020", ["gamma", "gamma"], [241, 194, 168], [241, 194, 167], [241, 194, 167], 1),
            ("C04020", ["inv", "invGamma"], [121, 242, 251], [121, 242, 250], [226, 255, 255], 1),
        ];
        let scheme = theme_scheme(None, FALLBACK_ACCENTS);
        for (base, ops, official, library, reverse, delta) in residuals {
            let rgb = [0xC0, 0x40, 0x20];
            let composed = apply_locked_srgb_transforms(rgb, &ops).unwrap();
            assert_eq!(composed, library, "library composition {ops:?}");
            let xml = format!(
                "<srgbClr val='{base}'><{a}/><{b}/></srgbClr>",
                a = ops[0],
                b = ops[1]
            );
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            assert_eq!(
                resolve_frame_solid(&doc.root, &scheme).unwrap(),
                FrameFill::Solid(library),
                "{xml}"
            );
            let d = official
                .iter()
                .zip(library.iter())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert_eq!(d, delta, "recorded residual {ops:?}");
            assert_ne!(library, official, "do not silently treat residual as exact {ops:?}");
            if reverse != library && official != reverse {
                let d_rev = official
                    .iter()
                    .zip(reverse.iter())
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert!(d < d_rev, "xml-order remains closer than reverse for {ops:?}");
            }
        }
    }

    #[test]
    fn srgb_transform_pairs_chart_frame_matches_official_rgb_reference() {
        // Exact official pairs only. Residual cases are not used as a
        // reference color. Do not sample official 425 by 266 coordinates on
        // the library 424 by 265 raster.
        let cases: [(&str, &str, &str, &str); 6] = [
            ("808080", "inv", "gamma", "F3F3F3"),
            ("808080", "gamma", "inv", "BBBBBB"),
            ("C04020", "inv", "comp", "FDBBB7"),
            ("C04020", "comp", "inv", "FDD3B7"),
            ("C04020", "comp", "gray", "878787"),
            ("C04020", "gray", "comp", "595959"),
        ];
        for (base, first, second, official) in cases {
            let fill = format!(
                "<c:spPr><a:solidFill><a:srgbClr val='{base}'><a:{first}/><a:{second}/></a:srgbClr></a:solidFill></c:spPr>"
            );
            let reference = format!("<c:spPr><a:solidFill><a:srgbClr val='{official}'/></a:solidFill></c:spPr>");
            let make = |sp: &str| LONG_LEGEND.replace("<c:chart>", &format!("{sp}<c:chart>"));
            let doc = crate::xml::parse(make(&fill).as_bytes()).unwrap();
            let reference = crate::xml::parse(make(&reference).as_bytes()).unwrap();
            let render = |root: &Element| render_chart(root, 424, 265, FALLBACK_ACCENTS, None, ChartHost::Excel, None, "").unwrap();
            assert_eq!(render(&doc.root), render(&reference.root), "{first}+{second} {base}");
        }
    }

    fn official_like_3d_xml(kind: &str, extra_wrapper: &str, extra_chart: &str) -> String {
        let rot_y = if kind == "pie3DChart" { "0" } else { "20" };
        let series = if kind == "pie3DChart" {
            r#"<c:ser>
          <c:idx val="0"/><c:order val="0"/><c:tx><c:v>F24S1</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="4"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt><c:pt idx="2"><c:v>C</c:v></c:pt><c:pt idx="3"><c:v>D</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="4"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>30</c:v></c:pt><c:pt idx="3"><c:v>40</c:v></c:pt></c:numLit></c:val>
        </c:ser>"#
                .to_string()
        } else {
            let one = |order: usize, name: &str, a: i32, b: i32, c: i32, d: i32| {
                format!(
                    r#"<c:ser>
          <c:idx val="{order}"/><c:order val="{order}"/><c:tx><c:v>{name}</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="4"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt><c:pt idx="2"><c:v>C</c:v></c:pt><c:pt idx="3"><c:v>D</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="4"/><c:pt idx="0"><c:v>{a}</c:v></c:pt><c:pt idx="1"><c:v>{b}</c:v></c:pt><c:pt idx="2"><c:v>{c}</c:v></c:pt><c:pt idx="3"><c:v>{d}</c:v></c:pt></c:numLit></c:val>
        </c:ser>"#
                )
            };
            format!(
                "{}{}{}",
                one(0, "F24S1", 10, 20, 30, 40),
                one(1, "F24S2", 20, 10, 20, 10),
                one(2, "F24S3", 20, 30, 10, 20)
            )
        };
        let head = match kind {
            "bar3DChart" => r#"<c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>"#,
            "line3DChart" => r#"<c:grouping val="standard"/><c:varyColors val="0"/>"#,
            "area3DChart" => r#"<c:grouping val="standard"/><c:varyColors val="0"/>"#,
            "pie3DChart" => r#"<c:varyColors val="1"/>"#,
            // Omitted wireframe is the official-like exportable case (same as val="0").
            "surface3DChart" => "",
            _ => "",
        };
        let axes = if kind == "pie3DChart" {
            String::new()
        } else {
            let mid = if matches!(kind, "area3DChart" | "surface3DChart") {
                "midCat"
            } else {
                "between"
            };
            format!(
                r#"<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="{mid}"/><c:majorUnit val="20"/></c:valAx>
      <c:serAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/></c:serAx>"#
            )
        };
        let ax_ids = if kind == "pie3DChart" {
            ""
        } else {
            r#"<c:axId val="1"/><c:axId val="2"/><c:axId val="3"/>"#
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    {extra_chart}
    <c:view3D><c:rotX val="15"/><c:rotY val="{rot_y}"/><c:rAngAx val="0"/></c:view3D>
    <c:plotArea>
      <c:{kind}>
        {head}
        {series}
        {extra_wrapper}
        {ax_ids}
      </c:{kind}>
      {axes}
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn count_nonwhite_rgb(rgb: &[u8]) -> usize {
        rgb.chunks_exact(3)
            .filter(|p| *p != [255, 255, 255])
            .count()
    }

    #[test]
    fn official_like_3d_charts_export_and_paint_ink() {
        // Official F24 Chart.Export gold exists for these five wrappers.
        // Close-enough projection must produce an image with real ink.
        // Residual pixels vs official gold stay on the review page; this
        // is not a unique official-matrix lock and does not claim 0-diff.
        for kind in [
            "bar3DChart",
            "line3DChart",
            "area3DChart",
            "pie3DChart",
            "surface3DChart",
        ] {
            // Official pie3D gold writes hidden dLbls plus showLeaderLines=1.
            let pie_dlbls = r#"<c:dLbls><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/><c:showLeaderLines val="1"/></c:dLbls>"#;
            let extra = if kind == "pie3DChart" { pie_dlbls } else { "" };
            let xml = official_like_3d_xml(kind, extra, "");
            let png = render_line(&xml).unwrap_or_else(|err| {
                panic!("{kind} must export; got {:?} {}", err.code(), err)
            });
            let (_w, _h, rgb) = png_rgb(&png);
            let ink = count_nonwhite_rgb(&rgb);
            assert!(ink >= 80, "{kind} must paint ink, got {ink}");
        }
    }

    #[test]
    fn leftover_3d_variants_without_official_gold_stay_not_implemented() {
        let cases = [
            official_like_3d_xml("bar3DChart", "", "")
                .replace(r#"barDir val="col""#, r#"barDir val="bar""#),
            official_like_3d_xml("bar3DChart", "", "").replace(
                r#"grouping val="clustered""#,
                r#"grouping val="stacked""#,
            ),
            official_like_3d_xml("bar3DChart", r#"<c:shape val="cone"/>"#, ""),
            official_like_3d_xml("line3DChart", "", "").replace(
                r#"grouping val="standard""#,
                r#"grouping val="stacked""#,
            ),
            official_like_3d_xml("area3DChart", "", "").replace(
                r#"grouping val="standard""#,
                r#"grouping val="percentStacked""#,
            ),
            official_like_3d_xml("surface3DChart", r#"<c:wireframe val="1"/>"#, ""),
            official_like_3d_xml(
                "bar3DChart",
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls>"#,
                "",
            ),
            official_like_3d_xml(
                "pie3DChart",
                r#"<c:dLbls><c:showVal val="1"/></c:dLbls>"#,
                "",
            ),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_line(xml).expect_err(&format!("leftover 3D case {i} should stay NI"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "leftover 3D case {i} should stay NotImplemented"
            );
        }
    }

    fn hidden_dlbls() -> &'static str {
        r#"<c:dLbls><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/><c:showLeaderLines val="1"/></c:dLbls>"#
    }

    fn cats_ab() -> &'static str {
        r#"<c:cat><c:strLit><c:ptCount val="4"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt><c:pt idx="2"><c:v>C</c:v></c:pt><c:pt idx="3"><c:v>D</c:v></c:pt></c:strLit></c:cat>"#
    }

    fn val_pts(vals: &str) -> String {
        let parts: Vec<&str> = vals.split(',').collect();
        let mut pts = String::new();
        for (i, v) in parts.iter().enumerate() {
            pts.push_str(&format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#));
        }
        format!(
            r#"<c:val><c:numLit><c:ptCount val="{}"/>{pts}</c:numLit></c:val>"#,
            parts.len()
        )
    }

    fn f25_ser(order: usize, name: &str, vals: &str, extra: &str) -> String {
        format!(
            r#"<c:ser>
          <c:idx val="{order}"/><c:order val="{order}"/><c:tx><c:v>{name}</c:v></c:tx>
          {cats}
          {val}
          {extra}
        </c:ser>"#,
            cats = cats_ab(),
            val = val_pts(vals),
        )
    }

    fn official_like_doughnut_xml() -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:plotArea>
      <c:doughnutChart>
        <c:varyColors val="1"/>
        {ser}
        {dlbls}
        <c:firstSliceAng val="0"/>
        <c:holeSize val="50"/>
      </c:doughnutChart>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            ser = f25_ser(0, "F25S1", "10,20,30,40", ""),
            dlbls = hidden_dlbls(),
        )
    }

    fn official_like_ofpie_xml() -> String {
        let cats = r#"<c:cat><c:strLit><c:ptCount val="7"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>B</c:v></c:pt><c:pt idx="2"><c:v>C</c:v></c:pt><c:pt idx="3"><c:v>D</c:v></c:pt><c:pt idx="4"><c:v>E</c:v></c:pt><c:pt idx="5"><c:v>F</c:v></c:pt><c:pt idx="6"><c:v>G</c:v></c:pt></c:strLit></c:cat>"#;
        let val = val_pts("40,25,15,10,8,6,5");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:plotArea>
      <c:ofPieChart>
        <c:ofPieType val="pie"/>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx>
          {cats}
          {val}
        </c:ser>
        {dlbls}
        <c:gapWidth val="150"/>
        <c:secondPieSize val="75"/>
        <c:serLines/>
      </c:ofPieChart>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            dlbls = hidden_dlbls(),
        )
    }

    fn official_like_radar_xml() -> String {
        let marker = r#"<c:marker></c:marker>"#;
        let series = format!(
            "{}{}{}",
            f25_ser(0, "F25S1", "10,20,30,40", marker),
            f25_ser(1, "F25S2", "20,10,20,10", marker),
            f25_ser(2, "F25S3", "20,30,10,20", marker),
        );
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:plotArea>
      <c:radarChart>
        <c:radarStyle val="marker"/>
        <c:varyColors val="0"/>
        {series}
        {dlbls}
        <c:axId val="1"/><c:axId val="2"/>
      </c:radarChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/><c:majorUnit val="20"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            dlbls = hidden_dlbls(),
        )
    }

    fn official_like_bubble_xml() -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:plotArea>
      <c:bubbleChart>
        <c:varyColors val="0"/>
        <c:ser>
          <c:idx val="0"/><c:order val="0"/><c:tx><c:v>S1</c:v></c:tx>
          <c:xVal><c:strLit><c:ptCount val="5"/><c:pt idx="0"><c:v>X</c:v></c:pt><c:pt idx="1"><c:v>10</c:v></c:pt><c:pt idx="2"><c:v>20</c:v></c:pt><c:pt idx="3"><c:v>30</c:v></c:pt><c:pt idx="4"><c:v>40</c:v></c:pt></c:strLit></c:xVal>
          <c:yVal><c:numLit><c:ptCount val="5"/><c:pt idx="0"><c:v>0</c:v></c:pt><c:pt idx="1"><c:v>10</c:v></c:pt><c:pt idx="2"><c:v>20</c:v></c:pt><c:pt idx="3"><c:v>30</c:v></c:pt><c:pt idx="4"><c:v>40</c:v></c:pt></c:numLit></c:yVal>
          <c:bubbleSize><c:numLit><c:ptCount val="5"/><c:pt idx="0"><c:v>0</c:v></c:pt><c:pt idx="1"><c:v>10</c:v></c:pt><c:pt idx="2"><c:v>20</c:v></c:pt><c:pt idx="3"><c:v>15</c:v></c:pt><c:pt idx="4"><c:v>25</c:v></c:pt></c:numLit></c:bubbleSize>
          <c:bubble3D val="0"/>
        </c:ser>
        <c:bubbleScale val="100"/>
        <c:showNegBubbles val="0"/>
        <c:axId val="1"/><c:axId val="2"/>
      </c:bubbleChart>
      <c:valAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/><c:max val="50"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/><c:crosses val="autoZero"/><c:crossBetween val="midCat"/><c:majorUnit val="10"/></c:valAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="50"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="midCat"/><c:majorUnit val="10"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
            .to_string()
    }

    fn official_like_stock_xml() -> String {
        let cat = r#"<c:cat><c:numLit><c:ptCount val="4"/><c:pt idx="0"><c:v>46023</c:v></c:pt><c:pt idx="1"><c:v>46024</c:v></c:pt><c:pt idx="2"><c:v>46025</c:v></c:pt><c:pt idx="3"><c:v>46026</c:v></c:pt></c:numLit></c:cat>"#;
        let ser = |order: usize, name: &str, vals: &str, extra: &str| {
            format!(
                r#"<c:ser>
          <c:idx val="{order}"/><c:order val="{order}"/><c:tx><c:v>{name}</c:v></c:tx>
          {cat}
          {val}
          {extra}
        </c:ser>"#,
                val = val_pts(vals),
            )
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:plotArea>
      <c:stockChart>
        {high}{low}{close}
        <c:hiLowLines/>
        {dlbls}
        <c:axId val="1"/><c:axId val="2"/>
      </c:stockChart>
      <c:dateAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:numFmt formatCode="yyyy\-mm\-dd" sourceLinked="1"/><c:crossAx val="2"/><c:crosses val="autoZero"/><c:baseTimeUnit val="days"/></c:dateAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/><c:majorUnit val="20"/></c:valAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
            high = ser(0, "High", "40,45,50,48", r#"<c:marker><c:symbol val="none"/></c:marker>"#),
            low = ser(1, "Low", "20,25,30,28", r#"<c:marker><c:symbol val="none"/></c:marker>"#),
            close = ser(2, "Close", "30,35,40,36", r#"<c:spPr><a:ln><a:solidFill><a:srgbClr val="196B24"/></a:solidFill></a:ln></c:spPr><c:marker><c:symbol val="circle"/><c:size val="5"/><c:spPr><a:solidFill><a:srgbClr val="196B24"/></a:solidFill></c:spPr></c:marker>"#),
            dlbls = hidden_dlbls(),
        )
    }

    fn official_like_surface_top_xml() -> String {
        let series = format!(
            "{}{}{}",
            f25_ser(0, "F25S1", "10,20,30,40", ""),
            f25_ser(1, "F25S2", "20,10,20,10", ""),
            f25_ser(2, "F25S3", "20,30,10,20", ""),
        );
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:view3D><c:rotX val="90"/><c:rotY val="0"/><c:rAngAx val="0"/><c:perspective val="0"/></c:view3D>
    <c:plotArea>
      <c:surfaceChart>
        {series}
        <c:axId val="1"/><c:axId val="2"/><c:axId val="3"/>
      </c:surfaceChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="none"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="midCat"/><c:majorUnit val="20"/></c:valAx>
      <c:serAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/></c:serAx>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    #[test]
    fn excel_serial_formats_official_stock_dates() {
        assert_eq!(format_excel_date(46023.0), "2026-01-01");
        assert_eq!(format_excel_date(46024.0), "2026-01-02");
        assert_eq!(format_excel_date(46025.0), "2026-01-03");
        assert_eq!(format_excel_date(46026.0), "2026-01-04");
    }

    #[test]
    fn official_like_f25_other_types_export_and_paint_ink() {
        // Official F25 Chart.Export gold exists for these six wrappers.
        // Close-enough painters must produce an image with real ink.
        // Residual pixels vs official gold stay on the review page; this
        // is not a unique official-matrix lock and does not claim 0-diff.
        let cases = [
            ("doughnutChart", official_like_doughnut_xml()),
            ("ofPieChart", official_like_ofpie_xml()),
            ("radarChart", official_like_radar_xml()),
            ("bubbleChart", official_like_bubble_xml()),
            ("stockChart", official_like_stock_xml()),
            ("surfaceChart", official_like_surface_top_xml()),
        ];
        for (kind, xml) in cases {
            let png = render_line(&xml).unwrap_or_else(|err| {
                panic!("{kind} must export; got {:?} {}", err.code(), err)
            });
            let (_w, _h, rgb) = png_rgb(&png);
            let ink = count_nonwhite_rgb(&rgb);
            assert!(ink >= 80, "{kind} must paint ink, got {ink}");
        }
    }

    #[test]
    fn leftover_f25_variants_without_official_gold_stay_not_implemented() {
        let cases = [
            official_like_doughnut_xml().replace(r#"holeSize val="50""#, r#"holeSize val="5""#),
            official_like_ofpie_xml().replace(r#"ofPieType val="pie""#, r#"ofPieType val="bar""#),
            official_like_ofpie_xml().replace(
                r#"<c:gapWidth val="150"/>"#,
                r#"<c:splitType val="cust"/><c:gapWidth val="150"/>"#,
            ),
            official_like_radar_xml()
                .replace(r#"radarStyle val="marker""#, r#"radarStyle val="filled""#),
            official_like_radar_xml()
                .replace(r#"radarStyle val="marker""#, r#"radarStyle val="standard""#),
            official_like_bubble_xml().replace(r#"bubble3D val="0""#, r#"bubble3D val="1""#),
            official_like_bubble_xml()
                .replace(r#"showNegBubbles val="0""#, r#"showNegBubbles val="1""#),
            official_like_stock_xml().replace(
                r#"<c:hiLowLines/>"#,
                r#"<c:hiLowLines/><c:upDownBars><c:gapWidth val="150"/></c:upDownBars>"#,
            ),
            official_like_surface_top_xml().replace(
                r#"<c:surfaceChart>"#,
                r#"<c:surfaceChart><c:wireframe val="1"/>"#,
            ),
            official_like_surface_top_xml().replace(r#"rotX val="90""#, r#"rotX val="15""#),
        ];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_line(xml).expect_err(&format!("leftover F25 case {i} should stay NI"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "leftover F25 case {i} should stay NotImplemented"
            );
        }
    }

    fn f26_ser(order: usize, name: &str, vals: &str, extra: &str) -> String {
        format!(
            r#"<c:ser>
          <c:idx val="{order}"/><c:order val="{order}"/><c:tx><c:v>{name}</c:v></c:tx>
          {cats}
          {val}
          {extra}
        </c:ser>"#,
            cats = cats_ab(),
            val = val_pts(vals),
        )
    }

    fn f26_line_marker() -> &'static str {
        r#"<c:marker><c:symbol val="none"/></c:marker>"#
    }

    fn f26_shared_axes() -> &'static str {
        r#"<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:tickLblPos val="nextTo"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/><c:majorUnit val="20"/></c:valAx>"#
    }

    fn official_like_f26_space(title: &str, plot: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <c:chart>
    <c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>{title}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>
    <c:plotArea>
      {plot}
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#
        )
    }

    fn official_like_f26_col_line_primary_xml() -> String {
        let plot = format!(
            r#"<c:barChart>
        <c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>
        {s1}
        {dlbls}
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:lineChart>
        <c:grouping val="standard"/><c:varyColors val="0"/>
        {s2}
        {dlbls}
        <c:smooth val="0"/><c:axId val="1"/><c:axId val="2"/>
      </c:lineChart>
      {axes}"#,
            s1 = f26_ser(0, "F26S1", "10,20,30,40", ""),
            s2 = f26_ser(1, "F26S2", "20,10,20,10", f26_line_marker()),
            dlbls = hidden_dlbls(),
            axes = f26_shared_axes(),
        );
        official_like_f26_space("F26-1", &plot)
    }

    fn official_like_f26_col_col_line_xml() -> String {
        let plot = format!(
            r#"<c:barChart>
        <c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>
        {s1}{s2}
        {dlbls}
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:lineChart>
        <c:grouping val="standard"/><c:varyColors val="0"/>
        {s3}
        {dlbls}
        <c:smooth val="0"/><c:axId val="1"/><c:axId val="2"/>
      </c:lineChart>
      {axes}"#,
            s1 = f26_ser(0, "F26S1", "10,20,30,40", ""),
            s2 = f26_ser(1, "F26S2", "20,10,20,10", ""),
            s3 = f26_ser(2, "F26S3", "20,30,10,20", f26_line_marker()),
            dlbls = hidden_dlbls(),
            axes = f26_shared_axes(),
        );
        official_like_f26_space("F26-2", &plot)
    }

    fn official_like_f26_col_line_secondary_xml() -> String {
        let plot = format!(
            r#"<c:barChart>
        <c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>
        {s1}
        {dlbls}
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:lineChart>
        <c:grouping val="standard"/><c:varyColors val="0"/>
        {s2}
        {dlbls}
        <c:smooth val="0"/><c:axId val="3"/><c:axId val="4"/>
      </c:lineChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/><c:majorUnit val="20"/></c:valAx>
      <c:valAx><c:axId val="4"/><c:scaling><c:orientation val="minMax"/><c:max val="50"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="r"/><c:crossAx val="3"/><c:crosses val="max"/><c:crossBetween val="between"/><c:majorUnit val="10"/></c:valAx>
      <c:catAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="1"/><c:axPos val="b"/><c:crossAx val="4"/></c:catAx>"#,
            s1 = f26_ser(0, "F26S1", "10,20,30,40", ""),
            s2 = f26_ser(1, "F26S2", "20,10,20,10", f26_line_marker()),
            dlbls = hidden_dlbls(),
        );
        official_like_f26_space("F26-3", &plot)
    }

    fn official_like_f26_col_area_primary_xml() -> String {
        let plot = format!(
            r#"<c:areaChart>
        <c:grouping val="standard"/><c:varyColors val="0"/>
        {s2}
        {dlbls}
        <c:axId val="1"/><c:axId val="2"/>
      </c:areaChart>
      <c:barChart>
        <c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>
        {s1}
        {dlbls}
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      {axes}"#,
            s1 = f26_ser(0, "F26S1", "10,20,30,40", ""),
            s2 = f26_ser(1, "F26S2", "20,10,20,10", ""),
            dlbls = hidden_dlbls(),
            axes = f26_shared_axes(),
        );
        official_like_f26_space("F26-4", &plot)
    }

    fn official_like_f26_bar_line_mixed_xml() -> String {
        let plot = format!(
            r#"<c:barChart>
        <c:barDir val="bar"/><c:grouping val="clustered"/><c:varyColors val="0"/>
        {s1}
        {dlbls}
        <c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/>
      </c:barChart>
      <c:lineChart>
        <c:grouping val="standard"/><c:varyColors val="0"/>
        {s2}
        {dlbls}
        <c:smooth val="0"/><c:axId val="3"/><c:axId val="4"/>
      </c:lineChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorGridlines/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/><c:majorUnit val="20"/></c:valAx>
      <c:valAx><c:axId val="4"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="r"/><c:crossAx val="3"/><c:crosses val="max"/><c:crossBetween val="between"/></c:valAx>
      <c:catAx><c:axId val="3"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="1"/><c:axPos val="b"/><c:crossAx val="4"/></c:catAx>"#,
            s1 = f26_ser(0, "F26S1", "10,20,30,40", ""),
            s2 = f26_ser(1, "F26S2", "20,10,20,10", f26_line_marker()),
            dlbls = hidden_dlbls(),
        );
        official_like_f26_space("F26-5", &plot)
    }

    #[test]
    fn official_like_f26_combo_charts_export_and_paint_ink() {
        // Official F26 Chart.Export gold exists for these combo encodings.
        // Close-enough painters must produce an image with real ink.
        // Residual pixels vs official gold stay on the review page; this
        // is not a unique official-matrix lock, not a general F26 combo
        // lock, and does not claim 0-diff. F26-3 reuses the already
        // landed F21-8 column+line secondary path.
        let cases = [
            ("col-line-primary", official_like_f26_col_line_primary_xml()),
            ("col-col-line", official_like_f26_col_col_line_xml()),
            ("col-line-secondary", official_like_f26_col_line_secondary_xml()),
            ("col-area-primary", official_like_f26_col_area_primary_xml()),
            ("bar-line-primary", official_like_f26_bar_line_mixed_xml()),
        ];
        for (kind, xml) in cases {
            let doc = crate::xml::parse(xml.as_bytes()).unwrap();
            let chart = read_chart(&doc.root, FALLBACK_ACCENTS, None, "", RenderDpi::DEFAULT)
                .unwrap_or_else(|err| panic!("{kind} must read; got {:?} {}", err.code(), err));
            match kind {
                "col-line-primary" => {
                    assert_eq!(chart.series.len(), 2);
                    assert!(matches!(chart.series[0].plot_kind, PlotKind::ClusteredColumn));
                    assert!(matches!(chart.series[1].plot_kind, PlotKind::StandardLine));
                    assert!(!chart.series[0].on_secondary && !chart.series[1].on_secondary);
                    assert!(!chart.secondary_line_on_right);
                }
                "col-col-line" => {
                    assert_eq!(chart.series.len(), 3);
                    assert!(matches!(chart.series[0].plot_kind, PlotKind::ClusteredColumn));
                    assert!(matches!(chart.series[1].plot_kind, PlotKind::ClusteredColumn));
                    assert!(matches!(chart.series[2].plot_kind, PlotKind::StandardLine));
                }
                "col-line-secondary" => {
                    assert!(matches!(chart.series[0].plot_kind, PlotKind::ClusteredColumn));
                    assert!(matches!(chart.series[1].plot_kind, PlotKind::StandardLine));
                    assert!(!chart.series[0].on_secondary && chart.series[1].on_secondary);
                    assert!(!chart.secondary_line_on_right);
                }
                "col-area-primary" => {
                    assert!(matches!(chart.series[0].plot_kind, PlotKind::ClusteredColumn));
                    assert!(matches!(chart.series[1].plot_kind, PlotKind::StandardArea));
                    assert!(!chart.area_midcat);
                }
                "bar-line-primary" => {
                    assert!(matches!(chart.kind, PlotKind::ClusteredBar));
                    assert!(matches!(chart.series[0].plot_kind, PlotKind::ClusteredBar));
                    assert!(matches!(chart.series[1].plot_kind, PlotKind::StandardLine));
                    assert!(!chart.series[0].on_secondary && chart.series[1].on_secondary);
                    assert!(chart.secondary_line_on_right);
                }
                _ => {}
            }
            let png = render_line(&xml).unwrap_or_else(|err| {
                panic!("{kind} must export; got {:?} {}", err.code(), err)
            });
            let (_w, _h, rgb) = png_rgb(&png);
            let ink = count_nonwhite_rgb(&rgb);
            assert!(ink >= 80, "{kind} must paint ink, got {ink}");
        }
    }

    #[test]
    fn leftover_f26_variants_without_official_gold_stay_not_implemented() {
        let three_col = official_like_f26_col_col_line_xml().replace(
            &f26_ser(1, "F26S2", "20,10,20,10", ""),
            &format!(
                "{}{}",
                f26_ser(1, "F26S2", "20,10,20,10", ""),
                f26_ser(3, "F26S4", "15,15,15,15", ""),
            ),
        );
        let line_area = official_like_f26_space(
            "NI",
            &format!(
                r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>{s1}{dlbls}<c:smooth val="0"/><c:axId val="1"/><c:axId val="2"/></c:lineChart>
      <c:areaChart><c:grouping val="standard"/><c:varyColors val="0"/>{s2}{dlbls}<c:axId val="1"/><c:axId val="2"/></c:areaChart>
      {axes}"#,
                s1 = f26_ser(0, "F26S1", "10,20,30,40", f26_line_marker()),
                s2 = f26_ser(1, "F26S2", "20,10,20,10", ""),
                dlbls = hidden_dlbls(),
                axes = f26_shared_axes(),
            ),
        );
        let stacked = official_like_f26_col_line_primary_xml().replace(
            r#"<c:grouping val="clustered"/>"#,
            r#"<c:grouping val="stacked"/>"#,
        );
        let smooth = official_like_f26_col_line_primary_xml().replace(
            r#"<c:smooth val="0"/>"#,
            r#"<c:smooth val="1"/>"#,
        );
        let marked = official_like_f26_col_line_primary_xml().replace(
            r#"<c:marker><c:symbol val="none"/></c:marker>"#,
            r#"<c:marker><c:symbol val="circle"/></c:marker>"#,
        );
        let three_wrappers = official_like_f26_col_line_primary_xml().replace(
            r#"</c:lineChart>"#,
            &format!(
                r#"</c:lineChart>
      <c:areaChart><c:grouping val="standard"/><c:varyColors val="0"/>{}{}<c:axId val="1"/><c:axId val="2"/></c:areaChart>"#,
                f26_ser(2, "F26S3", "20,30,10,20", ""),
                hidden_dlbls(),
            ),
        );
        let bar_area = official_like_f26_space(
            "NI",
            &format!(
                r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/><c:varyColors val="0"/>{s1}{dlbls}<c:gapWidth val="150"/><c:axId val="1"/><c:axId val="2"/></c:barChart>
      <c:areaChart><c:grouping val="standard"/><c:varyColors val="0"/>{s2}{dlbls}<c:axId val="1"/><c:axId val="2"/></c:areaChart>
      <c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="2"/><c:crosses val="autoZero"/></c:catAx>
      <c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/><c:max val="80"/><c:min val="0"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="1"/><c:crosses val="autoZero"/><c:crossBetween val="between"/><c:majorUnit val="20"/></c:valAx>"#,
                s1 = f26_ser(0, "F26S1", "10,20,30,40", ""),
                s2 = f26_ser(1, "F26S2", "20,10,20,10", ""),
                dlbls = hidden_dlbls(),
            ),
        );
        let cases = [three_col, line_area, stacked, smooth, marked, three_wrappers, bar_area];
        for (i, xml) in cases.iter().enumerate() {
            let err = render_line(xml).expect_err(&format!("leftover F26 case {i} should stay NI"));
            assert_eq!(
                err.code(),
                crate::error::ErrorCode::NotImplemented,
                "leftover F26 case {i} should stay NotImplemented"
            );
        }
    }
