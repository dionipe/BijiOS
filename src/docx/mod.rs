/**
 * OpenXML (.docx) Packaging and Extraction Engine
 * Clean-room implementation inspired by storytold/wordcraft
 * Generates ISO/IEC 29500 compliant Microsoft Word .docx files in pure Rust.
 */

pub struct DocxEngine;

impl DocxEngine {
    /// Computes standard IEEE 802.3 CRC-32 checksum
    pub fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFFFFFFu32;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if (crc & 1) != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    /// Packs files into a standard uncompressed (STORED) ZIP archive
    pub fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = Vec::new();
        let mut central_directory = Vec::new();

        for (filename, data) in entries {
            let offset = zip.len() as u32;
            let crc = Self::crc32(data);
            let size = data.len() as u32;
            let name_bytes = filename.as_bytes();
            let name_len = name_bytes.len() as u16;

            // 1. Local File Header
            zip.extend_from_slice(&0x04034B50u32.to_le_bytes()); // Local header signature
            zip.extend_from_slice(&20u16.to_le_bytes());        // Version needed (2.0)
            zip.extend_from_slice(&0u16.to_le_bytes());         // General purpose flag
            zip.extend_from_slice(&0u16.to_le_bytes());         // Compression (0 = STORED)
            zip.extend_from_slice(&0u16.to_le_bytes());         // Mod time
            zip.extend_from_slice(&0u16.to_le_bytes());         // Mod date
            zip.extend_from_slice(&crc.to_le_bytes());          // CRC-32
            zip.extend_from_slice(&size.to_le_bytes());         // Compressed size
            zip.extend_from_slice(&size.to_le_bytes());         // Uncompressed size
            zip.extend_from_slice(&name_len.to_le_bytes());     // Filename length
            zip.extend_from_slice(&0u16.to_le_bytes());         // Extra field length
            zip.extend_from_slice(name_bytes);                  // Filename
            zip.extend_from_slice(data);                        // File data

            // 2. Central Directory Header
            central_directory.extend_from_slice(&0x02014B50u32.to_le_bytes()); // Central dir signature
            central_directory.extend_from_slice(&20u16.to_le_bytes());        // Version made by
            central_directory.extend_from_slice(&20u16.to_le_bytes());        // Version needed
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // General purpose flag
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Compression (0 = STORED)
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Mod time
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Mod date
            central_directory.extend_from_slice(&crc.to_le_bytes());          // CRC-32
            central_directory.extend_from_slice(&size.to_le_bytes());         // Compressed size
            central_directory.extend_from_slice(&size.to_le_bytes());         // Uncompressed size
            central_directory.extend_from_slice(&name_len.to_le_bytes());     // Filename length
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Extra field length
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Comment length
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Disk start
            central_directory.extend_from_slice(&0u16.to_le_bytes());         // Internal attributes
            central_directory.extend_from_slice(&0u32.to_le_bytes());         // External attributes
            central_directory.extend_from_slice(&offset.to_le_bytes());       // Relative offset of local header
            central_directory.extend_from_slice(name_bytes);                  // Filename
        }

        let cd_offset = zip.len() as u32;
        let cd_size = central_directory.len() as u32;
        let entry_count = entries.len() as u16;

        zip.extend_from_slice(&central_directory);

        // 3. End of Central Directory Record
        zip.extend_from_slice(&0x06054B50u32.to_le_bytes()); // EOCD signature
        zip.extend_from_slice(&0u16.to_le_bytes());         // Number of this disk
        zip.extend_from_slice(&0u16.to_le_bytes());         // Disk with start of CD
        zip.extend_from_slice(&entry_count.to_le_bytes());  // Total entries on this disk
        zip.extend_from_slice(&entry_count.to_le_bytes());  // Total entries
        zip.extend_from_slice(&cd_size.to_le_bytes());      // CD size
        zip.extend_from_slice(&cd_offset.to_le_bytes());    // CD offset
        zip.extend_from_slice(&0u16.to_le_bytes());         // Comment length

        zip
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Converts HTML content into standard OpenXML word/document.xml body
    pub fn html_to_document_xml(html: &str) -> String {
        let mut paragraphs = Vec::new();

        // Simple tag tokenizer for clean OpenXML elements
        let clean_html = html
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n");

        // Split into block elements: h1, h2, h3, blockquote, table, or p
        let mut remaining = clean_html.as_str();

        while !remaining.is_empty() {
            if let Some(h1_start) = remaining.find("<h1") {
                if h1_start > 0 {
                    let before = &remaining[..h1_start];
                    Self::append_plain_paragraphs(&mut paragraphs, before);
                }
                if let Some(h1_end) = remaining[h1_start..].find("</h1>") {
                    let full_tag = &remaining[h1_start..h1_start + h1_end + 5];
                    let text = Self::strip_tags(full_tag);
                    if !text.trim().is_empty() {
                        paragraphs.push(format!(
                            r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:rPr><w:b/><w:color w:val="1E40AF"/><w:sz w:val="40"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                            Self::escape_xml(&text)
                        ));
                    }
                    remaining = &remaining[h1_start + h1_end + 5..];
                    continue;
                }
            }

            if let Some(h2_start) = remaining.find("<h2") {
                if h2_start > 0 {
                    let before = &remaining[..h2_start];
                    Self::append_plain_paragraphs(&mut paragraphs, before);
                }
                if let Some(h2_end) = remaining[h2_start..].find("</h2>") {
                    let full_tag = &remaining[h2_start..h2_start + h2_end + 5];
                    let text = Self::strip_tags(full_tag);
                    if !text.trim().is_empty() {
                        paragraphs.push(format!(
                            r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:rPr><w:b/><w:color w:val="2563EB"/><w:sz w:val="32"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                            Self::escape_xml(&text)
                        ));
                    }
                    remaining = &remaining[h2_start + h2_end + 5..];
                    continue;
                }
            }

            if let Some(h3_start) = remaining.find("<h3") {
                if h3_start > 0 {
                    let before = &remaining[..h3_start];
                    Self::append_plain_paragraphs(&mut paragraphs, before);
                }
                if let Some(h3_end) = remaining[h3_start..].find("</h3>") {
                    let full_tag = &remaining[h3_start..h3_start + h3_end + 5];
                    let text = Self::strip_tags(full_tag);
                    if !text.trim().is_empty() {
                        paragraphs.push(format!(
                            r#"<w:p><w:pPr><w:pStyle w:val="Heading3"/></w:pPr><w:r><w:rPr><w:b/><w:color w:val="3B82F6"/><w:sz w:val="28"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                            Self::escape_xml(&text)
                        ));
                    }
                    remaining = &remaining[h3_start + h3_end + 5..];
                    continue;
                }
            }

            if let Some(bq_start) = remaining.find("<blockquote") {
                if bq_start > 0 {
                    let before = &remaining[..bq_start];
                    Self::append_plain_paragraphs(&mut paragraphs, before);
                }
                if let Some(bq_end) = remaining[bq_start..].find("</blockquote>") {
                    let full_tag = &remaining[bq_start..bq_start + bq_end + 13];
                    let text = Self::strip_tags(full_tag);
                    if !text.trim().is_empty() {
                        paragraphs.push(format!(
                            r#"<w:p><w:pPr><w:ind w:left="720"/></w:pPr><w:r><w:rPr><w:i/><w:color w:val="475569"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                            Self::escape_xml(&text)
                        ));
                    }
                    remaining = &remaining[bq_start + bq_end + 13..];
                    continue;
                }
            }

            if let Some(tbl_start) = remaining.find("<table") {
                if tbl_start > 0 {
                    let before = &remaining[..tbl_start];
                    Self::append_plain_paragraphs(&mut paragraphs, before);
                }
                if let Some(tbl_end) = remaining[tbl_start..].find("</table>") {
                    let table_html = &remaining[tbl_start..tbl_start + tbl_end + 8];
                    paragraphs.push(Self::html_table_to_docx(table_html));
                    remaining = &remaining[tbl_start + tbl_end + 8..];
                    continue;
                }
            }

            // Fallback for remaining text or paragraphs
            Self::append_plain_paragraphs(&mut paragraphs, remaining);
            break;
        }

        if paragraphs.is_empty() {
            paragraphs.push(r#"<w:p><w:r><w:t></w:t></w:r></w:p>"#.to_string());
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {}
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/>
    </w:sectPr>
  </w:body>
</w:document>"#,
            paragraphs.join("\n    ")
        )
    }

    fn strip_tags(html: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for ch in html.chars() {
            if ch == '<' {
                in_tag = true;
            } else if ch == '>' {
                in_tag = false;
            } else if !in_tag {
                out.push(ch);
            }
        }
        out
    }

    fn append_plain_paragraphs(paragraphs: &mut Vec<String>, html_chunk: &str) {
        let stripped = Self::strip_tags(html_chunk);
        for line in stripped.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                paragraphs.push(format!(
                    r#"<w:p><w:r><w:t>{}</w:t></w:r></w:p>"#,
                    Self::escape_xml(trimmed)
                ));
            }
        }
    }

    fn html_table_to_docx(table_html: &str) -> String {
        let mut rows_xml = Vec::new();
        let mut cur = table_html;

        while let Some(tr_start) = cur.find("<tr") {
            if let Some(tr_end) = cur[tr_start..].find("</tr>") {
                let tr_content = &cur[tr_start..tr_start + tr_end + 5];
                let mut cells_xml = Vec::new();
                let mut cell_cur = tr_content;

                while let Some(td_start) = cell_cur.find("<td")
                    .or_else(|| cell_cur.find("<th")) {
                    let is_th = cell_cur[td_start..].starts_with("<th");
                    let close_tag = if is_th { "</th>" } else { "</td>" };
                    if let Some(td_end) = cell_cur[td_start..].find(close_tag) {
                        let cell_html = &cell_cur[td_start..td_start + td_end + close_tag.len()];
                        let text = Self::strip_tags(cell_html);
                        let b_open = if is_th { "<w:b/>" } else { "" };
                        cells_xml.push(format!(
                            r#"<w:tc><w:p><w:r><w:rPr>{}</w:rPr><w:t>{}</w:t></w:r></w:p></w:tc>"#,
                            b_open,
                            Self::escape_xml(text.trim())
                        ));
                        cell_cur = &cell_cur[td_start + td_end + close_tag.len()..];
                    } else {
                        break;
                    }
                }

                if !cells_xml.is_empty() {
                    rows_xml.push(format!(r#"<w:tr>{}</w:tr>"#, cells_xml.join("")));
                }
                cur = &cur[tr_start + tr_end + 5..];
            } else {
                break;
            }
        }

        format!(
            r#"<w:tbl><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders></w:tblPr>{}</w:tbl>"#,
            rows_xml.join("")
        )
    }

    /// Creates a complete, genuine .docx ZIP binary from HTML content
    pub fn create_docx_package(html_content: &str) -> Vec<u8> {
        let content_types = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
</Types>"#;

        let root_rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

        let doc_rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
</Relationships>"#;

        let settings = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:zoom w:percent="100"/>
</w:settings>"#;

        let styles = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault>
      <w:rPr>
        <w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/>
        <w:sz w:val="22"/>
        <w:color w:val="111827"/>
      </w:rPr>
    </w:rPrDefault>
  </w:docDefaults>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:spacing w:before="240" w:after="120"/></w:pPr>
    <w:rPr><w:b/><w:color w:val="1E40AF"/><w:sz w:val="40"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading2">
    <w:name w:val="heading 2"/>
    <w:pPr><w:spacing w:before="200" w:after="80"/></w:pPr>
    <w:rPr><w:b/><w:color w:val="2563EB"/><w:sz w:val="32"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading3">
    <w:name w:val="heading 3"/>
    <w:pPr><w:spacing w:before="160" w:after="60"/></w:pPr>
    <w:rPr><w:b/><w:color w:val="3B82F6"/><w:sz w:val="28"/></w:rPr>
  </w:style>
</w:styles>"#;

        let doc_xml = Self::html_to_document_xml(html_content);
        let doc_xml_bytes = doc_xml.as_bytes();

        let entries: Vec<(&str, &[u8])> = vec![
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", root_rels),
            ("word/_rels/document.xml.rels", doc_rels),
            ("word/document.xml", doc_xml_bytes),
            ("word/styles.xml", styles),
            ("word/settings.xml", settings),
        ];

        Self::build_zip(&entries)
    }

    /// Extracts HTML text from a standard .docx ZIP package
    pub fn extract_html_from_docx(bytes: &[u8]) -> Option<String> {
        if bytes.len() < 30 || &bytes[0..4] != b"PK\x03\x04" {
            return None;
        }

        // Search for "word/document.xml" entry in ZIP
        let target = b"word/document.xml";
        let mut idx = 0;

        while idx + 30 < bytes.len() {
            if &bytes[idx..idx + 4] == b"PK\x03\x04" {
                let compression = u16::from_le_bytes([bytes[idx + 8], bytes[idx + 9]]);
                let comp_size = u32::from_le_bytes([bytes[idx + 18], bytes[idx + 19], bytes[idx + 20], bytes[idx + 21]]) as usize;
                let name_len = u16::from_le_bytes([bytes[idx + 26], bytes[idx + 27]]) as usize;
                let extra_len = u16::from_le_bytes([bytes[idx + 28], bytes[idx + 29]]) as usize;
                let data_offset = idx + 30 + name_len + extra_len;

                if idx + 30 + name_len <= bytes.len() {
                    let entry_name = &bytes[idx + 30..idx + 30 + name_len];
                    if entry_name == target && compression == 0 {
                        if data_offset + comp_size <= bytes.len() {
                            let doc_bytes = &bytes[data_offset..data_offset + comp_size];
                            if let Ok(xml_str) = std::str::from_utf8(doc_bytes) {
                                return Some(Self::document_xml_to_html(xml_str));
                            }
                        }
                    }
                }
                idx = data_offset + comp_size;
            } else {
                idx += 1;
            }
        }

        None
    }

    fn document_xml_to_html(xml: &str) -> String {
        let mut html = String::new();
        let mut p_start = 0;

        while let Some(start) = xml[p_start..].find("<w:p") {
            let actual_start = p_start + start;
            if let Some(end) = xml[actual_start..].find("</w:p>") {
                let p_content = &xml[actual_start..actual_start + end + 6];
                let is_h1 = p_content.contains(r#"w:val="Heading1""#);
                let is_h2 = p_content.contains(r#"w:val="Heading2""#);
                let is_h3 = p_content.contains(r#"w:val="Heading3""#);

                let text = Self::extract_text_from_runs(p_content);
                if !text.trim().is_empty() {
                    if is_h1 {
                        html.push_str(&format!("<h1 style=\"color:#1e40af;\">{}</h1>\n", text));
                    } else if is_h2 {
                        html.push_str(&format!("<h2 style=\"color:#2563eb;\">{}</h2>\n", text));
                    } else if is_h3 {
                        html.push_str(&format!("<h3 style=\"color:#3b82f6;\">{}</h3>\n", text));
                    } else {
                        html.push_str(&format!("<p>{}</p>\n", text));
                    }
                }
                p_start = actual_start + end + 6;
            } else {
                break;
            }
        }

        if html.is_empty() {
            html = "<p>Dokumen Kosong</p>".to_string();
        }
        html
    }

    fn extract_text_from_runs(p_xml: &str) -> String {
        let mut result = String::new();
        let mut cur = p_xml;

        while let Some(t_start) = cur.find("<w:t") {
            if let Some(tag_end) = cur[t_start..].find('>') {
                let actual_data = t_start + tag_end + 1;
                if let Some(close) = cur[actual_data..].find("</w:t>") {
                    let text = &cur[actual_data..actual_data + close];
                    result.push_str(text);
                    cur = &cur[actual_data + close + 6..];
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }
}
