use std::io::Cursor;

use crate::xlsx::RenderDpi;

use super::{render_chart_at, theme_accents, ChartHost};

fn lock_soft() -> Option<crate::xlsx::chart_gdi::SoftFontGuard> {
    #[cfg(windows)]
    {
        Some(crate::xlsx::chart_gdi::force_soft_fonts())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn long_legend() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
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
</c:chartSpace>"#
}

fn styled_legend_chart() -> String {
    let title = concat!(
        r#"<c:title><c:tx><c:rich xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
        r#"<a:bodyPr/><a:lstStyle/><a:p><a:pPr>"#,
        r#"<a:defRPr sz="1800" b="1" i="1"><a:latin typeface="Calibri"/></a:defRPr>"#,
        r#"</a:pPr><a:r><a:rPr lang="en-US"/><a:t>Sales Title</a:t></a:r></a:p></c:rich></c:tx>"#,
        r#"<c:overlay val="0"/></c:title>"#,
    );
    let legend_tx = concat!(
        r#"<c:txPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">"#,
        r#"<a:bodyPr/><a:lstStyle/><a:p><a:pPr>"#,
        r#"<a:defRPr sz="1400" b="1" i="1"><a:solidFill><a:srgbClr val="000000"/></a:solidFill>"#,
        r#"<a:latin typeface="微软雅黑"/></a:defRPr></a:pPr></a:p></c:txPr>"#,
    );
    long_legend()
        .replace("<c:chart>", &format!("<c:chart>{title}"))
        .replace("</c:legend>", &format!("{legend_tx}</c:legend>"))
}

fn decode_rgb(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).unwrap();
    let raw = &buf[..frame.buffer_size()];
    let rgb = match frame.color_type {
        png::ColorType::Rgb => raw.to_vec(),
        png::ColorType::Rgba => raw
            .chunks(4)
            .flat_map(|p| p[..3].iter().copied())
            .collect(),
        _ => panic!("unexpected color type"),
    };
    (frame.width, frame.height, rgb)
}

fn dark_pixels(rgb: &[u8]) -> usize {
    rgb.chunks(3).filter(|p| p.iter().any(|c| *c < 200)).count()
}

#[test]
fn soft_legend_and_title_named_bold_italic_paint() {
    let _guard = lock_soft();
    let doc = crate::xml::parse(styled_legend_chart().as_bytes()).unwrap();
    let png = render_chart_at(
        &doc.root,
        424,
        265,
        theme_accents(None),
        None,
        ChartHost::Excel,
        None,
        "",
        RenderDpi::DEFAULT,
    )
    .unwrap_or_else(|err| {
        panic!("styled legend/title should render on the software font path, got {err}");
    });
    let (w, h, rgb) = decode_rgb(&png);
    assert_eq!((w, h), (424, 265));
    let ink = dark_pixels(&rgb);
    assert!(ink > 80, "legend and title should paint text, ink={ink}");
}
