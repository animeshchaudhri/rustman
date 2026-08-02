//! Parses spreadsheet response bodies (xlsx/xls/xlsb/ods) for preview.

use std::io::Cursor;

use calamine::{open_workbook_auto_from_rs, Reader};

/// A parsed spreadsheet, ready for a simple table preview.
#[derive(Debug, Clone)]
pub struct ParsedSheet {
    pub sheet_name: String,
    pub rows: Vec<Vec<String>>,
}

/// Parses the first worksheet of a spreadsheet file into rows of cell text.
///
/// Returns `Err` with a short human-readable message if the bytes aren't a
/// recognisable spreadsheet format, or the file has no sheets.
pub fn parse_first_sheet(bytes: &[u8]) -> Result<ParsedSheet, String> {
    let mut workbook =
        open_workbook_auto_from_rs(Cursor::new(bytes.to_vec())).map_err(|e| e.to_string())?;

    let worksheets = workbook.worksheets();
    let (sheet_name, range) = worksheets
        .into_iter()
        .next()
        .ok_or_else(|| "Spreadsheet has no sheets".to_owned())?;

    let rows = range
        .rows()
        .map(|row| row.iter().map(std::string::ToString::to_string).collect())
        .collect();

    Ok(ParsedSheet { sheet_name, rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Hand-builds a minimal, spec-valid .xlsx (a zip of a few small XML
    /// parts) entirely in memory, so this test needs no external fixture
    /// file or network access.
    fn build_minimal_xlsx() -> Vec<u8> {
        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#;

        let root_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

        let workbook_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#;

        let workbook = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#;

        let sheet1 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>Name</t></is></c><c r="B1" t="inlineStr"><is><t>Age</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Alice</t></is></c><c r="B2"><v>30</v></c></row>
</sheetData>
</worksheet>"#;

        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        for (name, contents) in [
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", root_rels),
            ("xl/_rels/workbook.xml.rels", workbook_rels),
            ("xl/workbook.xml", workbook),
            ("xl/worksheets/sheet1.xml", sheet1),
        ] {
            zip.start_file(name, options).expect("start_file");
            zip.write_all(contents.as_bytes()).expect("write");
        }
        zip.finish().expect("finish").into_inner()
    }

    #[test]
    fn parses_first_sheet_rows_and_header() {
        let xlsx = build_minimal_xlsx();

        let sheet = parse_first_sheet(&xlsx).expect("should parse as a spreadsheet");

        assert_eq!(sheet.sheet_name, "Sheet1");
        assert_eq!(sheet.rows.len(), 2, "rows were: {:?}", sheet.rows);
        assert_eq!(sheet.rows[0], vec!["Name".to_owned(), "Age".to_owned()]);
        assert_eq!(sheet.rows[1], vec!["Alice".to_owned(), "30".to_owned()]);
    }

    #[test]
    fn rejects_non_spreadsheet_bytes() {
        let result = parse_first_sheet(b"not a spreadsheet at all");
        assert!(result.is_err());
    }
}
