/**
 * OpenXML (.xlsx) Packaging and Extraction Engine
 * Generates ISO/IEC 29500 / ECMA-376 compliant Microsoft Excel .xlsx files in pure Rust.
 * Compatible with Microsoft Excel, Google Sheets, Apple Numbers, and LibreOffice Calc.
 */

use std::collections::BTreeMap;
use crate::api::json_helper::{parse_json, JsonValue};
use crate::docx::DocxEngine;

pub struct XlsxEngine;

impl XlsxEngine {
    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Converts column letter (e.g. "A", "Z", "AA") to 1-based index (1, 26, 27)
    pub fn col_to_num(col: &str) -> u32 {
        let mut num = 0u32;
        for b in col.to_uppercase().bytes() {
            if b >= b'A' && b <= b'Z' {
                num = num * 26 + (b - b'A' + 1) as u32;
            }
        }
        num
    }

    /// Parses a cell coordinate like "B4" into (col_str, row_num, col_num)
    pub fn parse_coord(coord: &str) -> Option<(String, u32, u32)> {
        let trimmed = coord.trim().to_uppercase();
        let letter_count = trimmed.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        if letter_count == 0 {
            return None;
        }
        let col_str = trimmed[..letter_count].to_string();
        let row_num = trimmed[letter_count..].parse::<u32>().ok()?;
        let col_num = Self::col_to_num(&col_str);
        Some((col_str, row_num, col_num))
    }

    /// Builds a full, genuine ISO/IEC 29500 OpenXML (.xlsx) ZIP archive
    pub fn create_xlsx_package(json_or_text: &str) -> Vec<u8> {
        let mut sheets_data: Vec<(String, BTreeMap<u32, BTreeMap<u32, (String, String, bool, Option<String>)>>)> = Vec::new();

        // 1. Parse JSON or CSV input
        if let Some(json) = parse_json(json_or_text) {
            if let Some(JsonValue::Array(sheets)) = json.get("sheets") {
                for s in sheets {
                    let sheet_name = s.get_str("name").unwrap_or("Sheet1").to_string();
                    let mut rows_map: BTreeMap<u32, BTreeMap<u32, (String, String, bool, Option<String>)>> = BTreeMap::new();

                    if let Some(JsonValue::Object(cells)) = s.get("cells") {
                        for (coord, cell_val) in cells {
                            if let Some((col_str, row_num, col_num)) = Self::parse_coord(coord) {
                                let (val_str, formula_opt, is_bold) = match cell_val {
                                    JsonValue::Object(c_obj) => {
                                        let v = c_obj.get("value").map(|val| match val {
                                            JsonValue::String(s) => s.clone(),
                                            JsonValue::Number(n) => n.to_string(),
                                            JsonValue::Bool(b) => b.to_string(),
                                            _ => String::new(),
                                        }).unwrap_or_default();
                                        let f = c_obj.get("formula").and_then(|formula| formula.as_str()).map(|formula| {
                                            if formula.starts_with('=') {
                                                formula[1..].to_string()
                                            } else {
                                                formula.to_string()
                                            }
                                        });
                                        let bold = c_obj.get("bold").and_then(|b| b.as_bool()).unwrap_or(false);
                                        (v, f, bold)
                                    }
                                    JsonValue::String(s) => (s.clone(), None, false),
                                    JsonValue::Number(n) => (n.to_string(), None, false),
                                    _ => (String::new(), None, false),
                                };

                                if !val_str.is_empty() || formula_opt.is_some() {
                                    rows_map.entry(row_num)
                                        .or_default()
                                        .insert(col_num, (col_str, val_str, is_bold, formula_opt));
                                }
                            }
                        }
                    }

                    sheets_data.push((sheet_name, rows_map));
                }
            }
        }

        // Fallback for CSV or raw text
        if sheets_data.is_empty() {
            let mut rows_map: BTreeMap<u32, BTreeMap<u32, (String, String, bool, Option<String>)>> = BTreeMap::new();
            for (r_idx, line) in json_or_text.lines().enumerate() {
                let row_num = (r_idx + 1) as u32;
                let delimiter = if line.contains('\t') { '\t' } else { ',' };
                for (c_idx, val) in line.split(delimiter).enumerate() {
                    let col_num = (c_idx + 1) as u32;
                    let col_str = if col_num <= 26 {
                        ((b'A' + (col_num - 1) as u8) as char).to_string()
                    } else {
                        format!("A{}", ((b'A' + (col_num - 27) as u8) as char))
                    };
                    let val_trimmed = val.trim().to_string();
                    if !val_trimmed.is_empty() {
                        rows_map.entry(row_num).or_default().insert(col_num, (col_str, val_trimmed, false, None));
                    }
                }
            }
            sheets_data.push(("Sheet1".to_string(), rows_map));
        }

        // 2. Generate XML for each worksheet
        let mut sheet_xmls = Vec::new();
        for (sheet_idx, (_name, rows_map)) in sheets_data.iter().enumerate() {
            let mut xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetViews>
    <sheetView tabSelected="1" workbookViewId="0"/>
  </sheetViews>
  <sheetFormatPr defaultRowHeight="20"/>
  <sheetData>
"#
            );

            for (row_num, cols) in rows_map {
                xml.push_str(&format!(r#"    <row r="{}">"#, row_num));
                xml.push('\n');

                for (_col_num, (col_str, val_str, is_bold, formula_opt)) in cols {
                    let cell_ref = format!("{}{}", col_str, row_num);
                    let style_attr = if *is_bold { r#" s="1""# } else { "" };

                    if let Some(ref formula) = formula_opt {
                        let esc_formula = Self::escape_xml(formula);
                        if let Ok(num) = val_str.parse::<f64>() {
                            xml.push_str(&format!(
                                r#"      <c r="{}"{}><f>{}</f><v>{}</v></c>"#,
                                cell_ref, style_attr, esc_formula, num
                            ));
                        } else if !val_str.is_empty() {
                            xml.push_str(&format!(
                                r#"      <c r="{}"{} t="inlineStr"><f>{}</f><is><t>{}</t></is></c>"#,
                                cell_ref, style_attr, esc_formula, Self::escape_xml(val_str)
                            ));
                        } else {
                            xml.push_str(&format!(
                                r#"      <c r="{}"{}><f>{}</f></c>"#,
                                cell_ref, style_attr, esc_formula
                            ));
                        }
                    } else if let Ok(num) = val_str.parse::<f64>() {
                        xml.push_str(&format!(
                            r#"      <c r="{}"{}><v>{}</v></c>"#,
                            cell_ref, style_attr, num
                        ));
                    } else {
                        xml.push_str(&format!(
                            r#"      <c r="{}"{} t="inlineStr"><is><t>{}</t></is></c>"#,
                            cell_ref, style_attr, Self::escape_xml(val_str)
                        ));
                    }
                    xml.push('\n');
                }

                xml.push_str("    </row>\n");
            }

            xml.push_str("  </sheetData>\n</worksheet>");
            sheet_xmls.push((sheet_idx + 1, xml));
        }

        // 3. Generate workbook.xml
        let mut workbook_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
"#
        );
        for (idx, (name, _)) in sheets_data.iter().enumerate() {
            workbook_xml.push_str(&format!(
                r#"    <sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
                Self::escape_xml(name),
                idx + 1,
                idx + 1
            ));
            workbook_xml.push('\n');
        }
        workbook_xml.push_str("  </sheets>\n</workbook>");

        // 4. Generate [Content_Types].xml
        let mut content_types_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
"#
        );
        for (idx, _) in sheets_data.iter().enumerate() {
            content_types_xml.push_str(&format!(
                r#"  <Override PartName="/xl/worksheets/sheet{}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
                idx + 1
            ));
            content_types_xml.push('\n');
        }
        content_types_xml.push_str("</Types>");

        // 5. Generate xl/_rels/workbook.xml.rels
        let mut workbook_rels_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#
        );
        for (idx, _) in sheets_data.iter().enumerate() {
            workbook_rels_xml.push_str(&format!(
                r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{}.xml"/>"#,
                idx + 1,
                idx + 1
            ));
            workbook_rels_xml.push('\n');
        }
        workbook_rels_xml.push_str(&format!(
            r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#,
            sheets_data.len() + 1
        ));

        // 6. Global _rels/.rels
        let root_rels_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

        // 7. xl/styles.xml (Standard Fonts, Fills, Borders)
        let styles_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="2">
    <font><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/></font>
    <font><b/><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/></font>
  </fonts>
  <fills count="2">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
  </fills>
  <borders count="1">
    <border><left/><right/><top/><bottom/><diagonal/></border>
  </borders>
  <cellStyleXfs count="1">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
  </cellStyleXfs>
  <cellXfs count="2">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/>
  </cellXfs>
  <cellStyles count="1">
    <cellStyle name="Normal" xfId="0" builtinId="0"/>
  </cellStyles>
</styleSheet>"#;

        // 8. Assemble all parts into ZIP archive
        let mut entries: Vec<(&str, &[u8])> = Vec::new();
        entries.push(("[Content_Types].xml", content_types_xml.as_bytes()));
        entries.push(("_rels/.rels", root_rels_xml.as_bytes()));
        entries.push(("xl/workbook.xml", workbook_xml.as_bytes()));
        entries.push(("xl/_rels/workbook.xml.rels", workbook_rels_xml.as_bytes()));
        entries.push(("xl/styles.xml", styles_xml.as_bytes()));

        for (sheet_num, xml) in &sheet_xmls {
            let path = format!("xl/worksheets/sheet{}.xml", sheet_num);
            // Leak path string so its slice lives for the zip build scope
            let leaked_path: &'static str = Box::leak(path.into_boxed_str());
            entries.push((leaked_path, xml.as_bytes()));
        }

        DocxEngine::build_zip(&entries)
    }

    /// Extracts spreadsheet data from a standard binary .xlsx (ZIP) into GridCraft JSON format
    pub fn extract_to_json(xlsx_bytes: &[u8]) -> Option<String> {
        // Simple ZIP local header reader
        let mut cursor = 0;
        let mut sheet_xml_map: BTreeMap<String, String> = BTreeMap::new();
        let mut shared_strings: Vec<String> = Vec::new();

        while cursor + 30 <= xlsx_bytes.len() {
            // Check Local File Header signature 0x04034B50
            if &xlsx_bytes[cursor..cursor + 4] != b"PK\x03\x04" {
                break;
            }

            let compression = u16::from_le_bytes([xlsx_bytes[cursor + 8], xlsx_bytes[cursor + 9]]);
            let comp_size = u32::from_le_bytes([xlsx_bytes[cursor + 18], xlsx_bytes[cursor + 19], xlsx_bytes[cursor + 20], xlsx_bytes[cursor + 21]]) as usize;
            let name_len = u16::from_le_bytes([xlsx_bytes[cursor + 26], xlsx_bytes[cursor + 27]]) as usize;
            let extra_len = u16::from_le_bytes([xlsx_bytes[cursor + 28], xlsx_bytes[cursor + 29]]) as usize;

            let header_size = 30 + name_len + extra_len;
            if cursor + header_size + comp_size > xlsx_bytes.len() {
                break;
            }

            let name_bytes = &xlsx_bytes[cursor + 30..cursor + 30 + name_len];
            let name_str = String::from_utf8_lossy(name_bytes).to_string();

            let data_bytes = &xlsx_bytes[cursor + header_size..cursor + header_size + comp_size];

            // If stored uncompressed (compression == 0)
            if compression == 0 {
                let content_str = String::from_utf8_lossy(data_bytes).to_string();
                if name_str.starts_with("xl/worksheets/") && name_str.ends_with(".xml") {
                    sheet_xml_map.insert(name_str, content_str);
                } else if name_str == "xl/sharedStrings.xml" {
                    // Extract <t> strings
                    let mut rem = content_str.as_str();
                    while let Some(start) = rem.find("<t>") {
                        let after = &rem[start + 3..];
                        if let Some(end) = after.find("</t>") {
                            shared_strings.push(after[..end].to_string());
                            rem = &after[end + 4..];
                        } else {
                            break;
                        }
                    }
                }
            }

            cursor += header_size + comp_size;
        }

        if sheet_xml_map.is_empty() {
            return None;
        }

        let mut sheets_json_parts = Vec::new();
        for (sheet_path, xml) in sheet_xml_map {
            let sheet_name = sheet_path.replace("xl/worksheets/", "").replace(".xml", "");
            let mut cells_parts = Vec::new();

            // Simple XML regex-less cell tokenizer: <c r="A1"...><v>...</v></c>
            let mut rem = xml.as_str();
            while let Some(c_start) = rem.find("<c ") {
                let cell_block = &rem[c_start..];
                let c_end = match cell_block.find("</c>") {
                    Some(idx) => idx + 4,
                    None => match cell_block.find("/>") {
                        Some(idx) => idx + 2,
                        None => break,
                    }
                };

                let cell_text = &cell_block[..c_end];
                rem = &cell_block[c_end..];

                // Extract coordinate r="..."
                if let Some(r_pos) = cell_text.find(r#"r=""#) {
                    let r_after = &cell_text[r_pos + 3..];
                    if let Some(r_end) = r_after.find('"') {
                        let coord = &r_after[..r_end];

                        // Extract value <v>...</v> or <t>...</t>
                        let val = if let Some(v_start) = cell_text.find("<v>") {
                            let v_after = &cell_text[v_start + 3..];
                            let v_end = v_after.find("</v>").unwrap_or(0);
                            let raw_v = &v_after[..v_end];
                            // Check if shared string index
                            if cell_text.contains(r#"t="s""#) {
                                if let Ok(idx) = raw_v.parse::<usize>() {
                                    shared_strings.get(idx).cloned().unwrap_or_else(|| raw_v.to_string())
                                } else {
                                    raw_v.to_string()
                                }
                            } else {
                                raw_v.to_string()
                            }
                        } else if let Some(t_start) = cell_text.find("<t>") {
                            let t_after = &cell_text[t_start + 3..];
                            let t_end = t_after.find("</t>").unwrap_or(0);
                            t_after[..t_end].to_string()
                        } else {
                            String::new()
                        };

                        // Extract formula <f>...</f>
                        let formula = if let Some(f_start) = cell_text.find("<f>") {
                            let f_after = &cell_text[f_start + 3..];
                            let f_end = f_after.find("</f>").unwrap_or(0);
                            Some(format!("={}", &f_after[..f_end]))
                        } else {
                            None
                        };

                        let is_bold = cell_text.contains(r#"s="1""#);

                        let formula_json = if let Some(ref f) = formula {
                            format!(r#","formula":"{}""#, Self::escape_xml(f))
                        } else {
                            String::new()
                        };

                        cells_parts.push(format!(
                            r#""{}":{{"value":"{}"{}{}}}"#,
                            coord,
                            Self::escape_xml(&val),
                            if is_bold { r#","bold":true"# } else { "" },
                            formula_json
                        ));
                    }
                }
            }

            sheets_json_parts.push(format!(
                r#"{{"name":"{}","cells":{{{}}}}}"#,
                sheet_name,
                cells_parts.join(",")
            ));
        }

        Some(format!(r#"{{"title":"Spreadsheet.xlsx","sheets":[{}]}}"#, sheets_json_parts.join(",")))
    }
}
