/**
 * OpenXML (.pptx) Packaging and Extraction Engine
 * Generates ISO/IEC 29500 / ECMA-376 compliant Microsoft PowerPoint .pptx files in pure Rust.
 * Compatible with Microsoft PowerPoint, LibreOffice Impress, Google Slides, and Apple Keynote.
 */

use crate::api::json_helper::{escape_json, parse_json, JsonValue};
use crate::docx::DocxEngine;
use std::collections::BTreeMap;

pub struct PptxEngine;

impl PptxEngine {
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Extract a clean 6-character uppercase hex color (e.g. "0F172A") from HEX or CSS gradient
    fn extract_hex_color(input: &str, default_hex: &str) -> String {
        if let Some(hash_idx) = input.find('#') {
            let after = &input[hash_idx + 1..];
            let hex: String = after
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if hex.len() == 6 {
                return hex.to_uppercase();
            } else if hex.len() == 3 {
                let mut expanded = String::with_capacity(6);
                for c in hex.chars() {
                    expanded.push(c);
                    expanded.push(c);
                }
                return expanded.to_uppercase();
            }
        }
        default_hex.to_uppercase()
    }

    /// Build a complete, genuine ISO/IEC 29500 OpenXML (.pptx) ZIP archive
    pub fn create_pptx_package(json_or_text: &str) -> Vec<u8> {
        let mut title = String::from("DeckCraft Presentation");
        let mut slides_xml_list: Vec<String> = Vec::new();

        if let Some(json) = parse_json(json_or_text) {
            if let Some(t) = json.get_str("title") {
                if !t.trim().is_empty() {
                    title = t.trim().to_string();
                }
            }
            if let Some(JsonValue::Array(slides)) = json.get("slides") {
                for slide in slides {
                    slides_xml_list.push(Self::build_slide_xml(slide));
                }
            }
        }

        if slides_xml_list.is_empty() {
            // Ensure at least 1 valid slide exists
            let fallback_slide = JsonValue::Null;
            slides_xml_list.push(Self::build_slide_xml(&fallback_slide));
        }

        let slide_count = slides_xml_list.len();
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

        // 1. [Content_Types].xml
        let mut ct_slides = String::new();
        for i in 1..=slide_count {
            ct_slides.push_str(&format!(
                r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
                i
            ));
        }
        let content_types = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="json" ContentType="application/json"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/ppt/presProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presProps+xml"/>
  <Override PartName="/ppt/viewProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml"/>
  <Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
  {}
</Types>"#,
            ct_slides
        );
        entries.push(("[Content_Types].xml".to_string(), content_types.into_bytes()));

        // 2. _rels/.rels
        let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;
        entries.push(("_rels/.rels".to_string(), root_rels.as_bytes().to_vec()));

        // 3. docProps/core.xml & docProps/app.xml
        let core_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>{}</dc:title>
  <dc:creator>BijiOS DeckCraft</dc:creator>
  <cp:lastModifiedBy>BijiOS DeckCraft</cp:lastModifiedBy>
</cp:coreProperties>"#,
            Self::escape_xml(&title)
        );
        entries.push(("docProps/core.xml".to_string(), core_xml.into_bytes()));

        let app_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
  <Application>BijiOS DeckCraft</Application>
  <Slides>{}</Slides>
  <PresentationFormat>Widescreen</PresentationFormat>
</Properties>"#,
            slide_count
        );
        entries.push(("docProps/app.xml".to_string(), app_xml.into_bytes()));

        // Store DeckCraft round-trip JSON inside docProps/deckcraft.json
        if json_or_text.trim().starts_with('{') {
            entries.push(("docProps/deckcraft.json".to_string(), json_or_text.as_bytes().to_vec()));
        }

        // 4. ppt/presentation.xml & ppt/_rels/presentation.xml.rels
        let mut sld_id_lst = String::new();
        let mut pres_slide_rels = String::new();
        for i in 1..=slide_count {
            let rid = i + 4; // rId1..rId4 reserved for master, presProps, viewProps, tableStyles
            sld_id_lst.push_str(&format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 255 + i, rid));
            pres_slide_rels.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                rid, i
            ));
        }

        // 16:9 Widescreen standard: 12192000 x 6858000 EMU
        let presentation_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" saveSubsetFonts="1">
  <p:sldMasterIdLst>
    <p:sldMasterId id="2147483648" r:id="rId1"/>
  </p:sldMasterIdLst>
  <p:sldIdLst>
    {}
  </p:sldIdLst>
  <p:sldSz cx="12192000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#,
            sld_id_lst
        );
        entries.push(("ppt/presentation.xml".to_string(), presentation_xml.into_bytes()));

        let presentation_rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps" Target="presProps.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/viewProps" Target="viewProps.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles" Target="tableStyles.xml"/>
  {}
</Relationships>"#,
            pres_slide_rels
        );
        entries.push(("ppt/_rels/presentation.xml.rels".to_string(), presentation_rels.into_bytes()));

        // 5. Slide Master, Slide Layout & Theme
        let slide_master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
    </p:spTree>
  </p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:sldLayoutIdLst>
    <p:sldLayoutId id="2147483649" r:id="rId1"/>
  </p:sldLayoutIdLst>
</p:sldMaster>"#;
        entries.push(("ppt/slideMasters/slideMaster1.xml".to_string(), slide_master_xml.as_bytes().to_vec()));

        let slide_master_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#;
        entries.push(("ppt/slideMasters/_rels/slideMaster1.xml.rels".to_string(), slide_master_rels.as_bytes().to_vec()));

        let slide_layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
  <p:cSld name="Blank">
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#;
        entries.push(("ppt/slideLayouts/slideLayout1.xml".to_string(), slide_layout_xml.as_bytes().to_vec()));

        let slide_layout_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;
        entries.push(("ppt/slideLayouts/_rels/slideLayout1.xml.rels".to_string(), slide_layout_rels.as_bytes().to_vec()));

        let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="BijiOS Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="1E293B"/></a:dk2>
      <a:lt2><a:srgbClr val="F8FAFC"/></a:lt2>
      <a:accent1><a:srgbClr val="D24726"/></a:accent1>
      <a:accent2><a:srgbClr val="2563EB"/></a:accent2>
      <a:accent3><a:srgbClr val="10B981"/></a:accent3>
      <a:accent4><a:srgbClr val="F59E0B"/></a:accent4>
      <a:accent5><a:srgbClr val="8B5CF6"/></a:accent5>
      <a:accent6><a:srgbClr val="06B6D4"/></a:accent6>
      <a:hlink><a:srgbClr val="007AFF"/></a:hlink>
      <a:folHlink><a:srgbClr val="6366F1"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="25400"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;
        entries.push(("ppt/theme/theme1.xml".to_string(), theme_xml.as_bytes().to_vec()));

        // 6. Presentation auxiliary props
        let pres_props = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentationPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;
        let view_props = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:viewPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:normalViewPr><p:restoredLeft sz="15620"/><p:restoredTop sz="94660"/></p:normalViewPr></p:viewPr>"#;
        let table_styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"/>"#;
        entries.push(("ppt/presProps.xml".to_string(), pres_props.as_bytes().to_vec()));
        entries.push(("ppt/viewProps.xml".to_string(), view_props.as_bytes().to_vec()));
        entries.push(("ppt/tableStyles.xml".to_string(), table_styles.as_bytes().to_vec()));

        // 7. Individual slides & their rels
        let slide_rel_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

        for (idx, s_xml) in slides_xml_list.into_iter().enumerate() {
            let num = idx + 1;
            entries.push((format!("ppt/slides/slide{}.xml", num), s_xml.into_bytes()));
            entries.push((
                format!("ppt/slides/_rels/slide{}.xml.rels", num),
                slide_rel_xml.as_bytes().to_vec(),
            ));
        }

        let refs: Vec<(&str, &[u8])> = entries.iter().map(|(k, v)| (k.as_str(), v.as_slice())).collect();
        DocxEngine::build_zip(&refs)
    }

    fn build_slide_xml(slide: &JsonValue) -> String {
        let bg_raw = slide.get_str("background").unwrap_or("#ffffff");
        let bg_hex = Self::extract_hex_color(bg_raw, "FFFFFF");

        let mut shapes_xml = String::new();
        let mut shape_id = 2u32;

        if let Some(JsonValue::Array(elements)) = slide.get("elements") {
            for el in elements {
                let el_type = el.get_str("type").unwrap_or("text");
                let x = (el.get_number("x").unwrap_or(60.0).max(0.0) * 15240.0) as i64;
                let y = (el.get_number("y").unwrap_or(60.0).max(0.0) * 15240.0) as i64;
                let w = (el.get_number("w").unwrap_or(560.0).max(40.0) * 15240.0) as i64;
                let h = (el.get_number("h").unwrap_or(80.0).max(30.0) * 15240.0) as i64;
                let color_hex = Self::extract_hex_color(el.get_str("color").unwrap_or("#0f172a"), "0F172A");
                let bg_fill_hex = Self::extract_hex_color(el.get_str("bg").unwrap_or("#3b82f6"), "3B82F6");
                let font_size = (el.get_number("size").unwrap_or(if el_type == "title" { 40.0 } else { 18.0 }) * 100.0) as u32;
                let is_bold = el.get_bool("bold").unwrap_or(el_type == "title" || el_type == "heading");
                let is_italic = el.get_bool("italic").unwrap_or(false);
                let align = match el.get_str("align").unwrap_or("left") {
                    "center" => "ctr",
                    "right" => "r",
                    _ => "l",
                };

                if el_type == "table" {
                    shapes_xml.push_str(&Self::build_table_xml(shape_id, x, y, w, h, &color_hex, el));
                } else {
                    let (prst, fill_xml, text_content, text_color) = match el_type {
                        "circle" => (
                            "ellipse",
                            format!(r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#, bg_fill_hex),
                            el.get_str("text").unwrap_or("").to_string(),
                            color_hex.clone(),
                        ),
                        "rect" | "badge" => (
                            "roundRect",
                            format!(r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#, bg_fill_hex),
                            el.get_str("text").unwrap_or("").to_string(),
                            color_hex.clone(),
                        ),
                        "arrow" => (
                            "rightArrow",
                            format!(r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#, color_hex),
                            String::new(),
                            "FFFFFF".to_string(),
                        ),
                        "card" => {
                            let c_title = el.get_str("title").unwrap_or("Card");
                            let c_desc = el.get_str("desc").unwrap_or("");
                            (
                                "roundRect",
                                r#"<a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill><a:ln w="19050"><a:solidFill><a:srgbClr val="CBD5E1"/></a:solidFill></a:ln>"#.to_string(),
                                format!("{}\n{}", c_title, c_desc),
                                "111827".to_string(),
                            )
                        }
                        "chart" => {
                            let c_title = el.get_str("title").unwrap_or("Chart");
                            (
                                "roundRect",
                                r#"<a:solidFill><a:srgbClr val="F8FAFC"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="CBD5E1"/></a:solidFill></a:ln>"#.to_string(),
                                format!("[Chart] {}", c_title),
                                "1E293B".to_string(),
                            )
                        }
                        _ => (
                            "rect",
                            "<a:noFill/>".to_string(),
                            el.get_str("text").unwrap_or("").to_string(),
                            color_hex.clone(),
                        ),
                    };

                    let mut paras = String::new();
                    for line in text_content.split('\n') {
                        paras.push_str(&format!(
                            r#"<a:p><a:pPr algn="{}"/><a:r><a:rPr lang="en-US" sz="{}" b="{}" i="{}" dirty="0"><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:rPr><a:t>{}</a:t></a:r></a:p>"#,
                            align,
                            font_size,
                            if is_bold { "1" } else { "0" },
                            if is_italic { "1" } else { "0" },
                            text_color,
                            Self::escape_xml(line)
                        ));
                    }
                    if paras.is_empty() {
                        paras.push_str("<a:p><a:endParaRPr lang=\"en-US\"/></a:p>");
                    }

                    shapes_xml.push_str(&format!(
                        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="{}" name="Shape {}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
  <p:spPr>
    <a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm>
    <a:prstGeom prst="{}"><a:avLst/></a:prstGeom>
    {}
  </p:spPr>
  <p:txBody>
    <a:bodyPr wrap="square" rtlCol="0"/>
    <a:lstStyle/>
    {}
  </p:txBody>
</p:sp>"#,
                        shape_id, shape_id, x, y, w, h, prst, fill_xml, paras
                    ));
                }
                shape_id += 1;
            }
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:solidFill><a:srgbClr val="{}"/></a:solidFill>
        <a:effectLst/>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
      {}
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#,
            bg_hex, shapes_xml
        )
    }

    fn build_table_xml(id: u32, x: i64, y: i64, w: i64, h: i64, header_hex: &str, el: &JsonValue) -> String {
        let mut rows_vec: Vec<Vec<String>> = Vec::new();
        if let Some(JsonValue::Array(rows)) = el.get("rows") {
            for r in rows {
                if let JsonValue::Array(cols) = r {
                    rows_vec.push(cols.iter().map(|c| c.as_str().unwrap_or("").to_string()).collect());
                }
            }
        }
        if rows_vec.is_empty() {
            rows_vec.push(vec!["Col 1".into(), "Col 2".into(), "Col 3".into()]);
            rows_vec.push(vec!["Data 1".into(), "Data 2".into(), "Data 3".into()]);
        }

        let col_count = rows_vec[0].len().max(1);
        let col_w = w / (col_count as i64);
        let row_h = (h / (rows_vec.len() as i64)).max(370840);

        let mut grid_cols = String::new();
        for _ in 0..col_count {
            grid_cols.push_str(&format!(r#"<a:gridCol w="{}"/>"#, col_w));
        }

        let mut tr_xml = String::new();
        for (r_idx, r) in rows_vec.iter().enumerate() {
            let mut tc_xml = String::new();
            for cell in r {
                let (fill, txt_clr, bold) = if r_idx == 0 {
                    (header_hex, "FFFFFF", "1")
                } else {
                    ("FFFFFF", "1E293B", "0")
                };
                tc_xml.push_str(&format!(
                    r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="1400" b="{}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:rPr><a:t>{}</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:tcPr></a:tc>"#,
                    bold,
                    txt_clr,
                    Self::escape_xml(cell),
                    fill
                ));
            }
            tr_xml.push_str(&format!(r#"<a:tr h="{}">{}</a:tr>"#, row_h, tc_xml));
        }

        format!(
            r#"<p:graphicFrame>
  <p:nvGraphicFramePr><p:cNvPr id="{}" name="Table {}"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></p:xfrm>
  <a:graphic>
    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
      <a:tbl>
        <a:tblPr firstRow="1" bandRow="1"/>
        <a:tblGrid>{}</a:tblGrid>
        {}
      </a:tbl>
    </a:graphicData>
  </a:graphic>
</p:graphicFrame>"#,
            id, id, x, y, w, h, grid_cols, tr_xml
        )
    }

    /// Extract DeckCraft JSON from a genuine `.pptx` ZIP archive
    pub fn extract_to_json(pptx_bytes: &[u8]) -> Option<String> {
        let mut cursor = 0;
        let mut slide_xml_map: BTreeMap<String, String> = BTreeMap::new();

        while cursor + 30 <= pptx_bytes.len() {
            if &pptx_bytes[cursor..cursor + 4] != b"PK\x03\x04" {
                break;
            }
            let compression = u16::from_le_bytes([pptx_bytes[cursor + 8], pptx_bytes[cursor + 9]]);
            let comp_size = u32::from_le_bytes([
                pptx_bytes[cursor + 18],
                pptx_bytes[cursor + 19],
                pptx_bytes[cursor + 20],
                pptx_bytes[cursor + 21],
            ]) as usize;
            let name_len = u16::from_le_bytes([pptx_bytes[cursor + 26], pptx_bytes[cursor + 27]]) as usize;
            let extra_len = u16::from_le_bytes([pptx_bytes[cursor + 28], pptx_bytes[cursor + 29]]) as usize;

            let header_size = 30 + name_len + extra_len;
            if cursor + header_size + comp_size > pptx_bytes.len() {
                break;
            }

            let name_str = String::from_utf8_lossy(&pptx_bytes[cursor + 30..cursor + 30 + name_len]).to_string();
            let data_bytes = &pptx_bytes[cursor + header_size..cursor + header_size + comp_size];

            if compression == 0 {
                let content_str = String::from_utf8_lossy(data_bytes).to_string();
                if name_str == "docProps/deckcraft.json" && content_str.trim().starts_with('{') {
                    return Some(content_str);
                }
                if name_str.starts_with("ppt/slides/slide") && name_str.ends_with(".xml") {
                    slide_xml_map.insert(name_str, content_str);
                }
            }

            cursor += header_size + comp_size;
        }

        if slide_xml_map.is_empty() {
            return None;
        }

        // Fallback: extract text runs from external PowerPoint XML slides
        let mut slides_json = Vec::new();
        for (idx, (_, xml)) in slide_xml_map.iter().enumerate() {
            let mut texts = Vec::new();
            let mut rem = xml.as_str();
            while let Some(s) = rem.find("<a:t>") {
                let after = &rem[s + 5..];
                if let Some(e) = after.find("</a:t>") {
                    let t = after[..e].trim();
                    if !t.is_empty() {
                        texts.push(t.to_string());
                    }
                    rem = &after[e + 6..];
                } else {
                    break;
                }
            }
            let title = texts.first().cloned().unwrap_or_else(|| format!("Slide {}", idx + 1));
            let body = if texts.len() > 1 { texts[1..].join("\n") } else { String::new() };
            slides_json.push(format!(
                r##"{{"id":"slide_{}","background":"#ffffff","transition":"fade","elements":[{{"type":"title","text":"{}","x":60,"y":60,"w":680,"h":60,"size":36,"color":"#0f172a","bold":true}},{{"type":"text","text":"{}","x":60,"y":140,"w":680,"h":240,"size":18,"color":"#334155"}}]}}"##,
                idx + 1,
                escape_json(&title),
                escape_json(&body)
            ));
        }

        Some(format!(r#"{{"title":"Imported Presentation.pptx","slides":[{}]}}"#, slides_json.join(",")))
    }
}
