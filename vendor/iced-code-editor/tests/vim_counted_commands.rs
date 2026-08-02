use iced_code_editor::{CodeEditor, Message, VimMode};

fn vim_keys(editor: &mut CodeEditor, keys: &str) {
    for key in keys.chars() {
        let _ = editor.update(&Message::VimKey(key));
    }
}

#[test]
fn vim_counted_line_jumps_use_one_based_targets() {
    let content = (1..=10)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut editor = CodeEditor::new(&content, "txt").with_vim_enabled(true);

    vim_keys(&mut editor, "5G");
    assert_eq!(editor.cursor_position(), (4, 0));

    vim_keys(&mut editor, "3gg");
    assert_eq!(editor.cursor_position(), (2, 0));

    vim_keys(&mut editor, "99G");
    assert_eq!(editor.cursor_position(), (9, 0));

    vim_keys(&mut editor, "gg");
    assert_eq!(editor.cursor_position(), (0, 0));

    vim_keys(&mut editor, "G");
    assert_eq!(editor.cursor_position(), (9, 0));
}

#[test]
fn vim_five_yy_yanks_five_lines() {
    let mut editor = CodeEditor::new("one\ntwo\nthree\nfour\nfive\nsix", "txt")
        .with_vim_enabled(true);

    vim_keys(&mut editor, "5yyp");

    assert_eq!(
        editor.content(),
        "one\none\ntwo\nthree\nfour\nfive\ntwo\nthree\nfour\nfive\nsix"
    );
    assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
}

#[test]
fn vim_counted_dd_and_cc_apply_to_requested_lines() {
    let mut deleted =
        CodeEditor::new("one\ntwo\nthree\nfour\nfive\nsix\nseven", "txt")
            .with_vim_enabled(true);
    vim_keys(&mut deleted, "5dd");
    assert_eq!(deleted.content(), "six\nseven");
    assert_eq!(deleted.vim_mode(), Some(VimMode::Normal));

    let mut changed =
        CodeEditor::new("one\ntwo\nthree\nfour\nfive\nsix\nseven", "txt")
            .with_vim_enabled(true);
    vim_keys(&mut changed, "5cc");
    assert_eq!(changed.content(), "six\nseven");
    assert_eq!(changed.vim_mode(), Some(VimMode::Insert));
}
