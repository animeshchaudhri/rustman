use serde::Serialize as _;


pub fn pretty_json(text: &str, use_tabs: bool) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    if use_tabs {
        let mut buf = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        value.serialize(&mut ser).ok()?;
        String::from_utf8(buf).ok()
    } else {
        serde_json::to_string_pretty(&value).ok()
    }
}
