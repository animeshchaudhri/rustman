use iced_code_editor::{CodeEditor, Message, VimMode};

fn vim_keys(editor: &mut CodeEditor, keys: &str) {
    for key in keys.chars() {
        let _ = editor.update(&Message::VimKey(key));
    }
}

#[test]
fn vim_slash_search_and_n_capital_n_repeat() {
    let mut editor = CodeEditor::new("foo zero\nbar foo\nfoo end", "txt")
        .with_vim_enabled(true);

    vim_keys(&mut editor, "/foo\n");
    assert_eq!(editor.cursor_position(), (1, 4));
    assert_eq!(editor.vim_mode(), Some(VimMode::Normal));

    vim_keys(&mut editor, "n");
    assert_eq!(editor.cursor_position(), (2, 0));
    vim_keys(&mut editor, "n");
    assert_eq!(editor.cursor_position(), (0, 0));
    vim_keys(&mut editor, "N");
    assert_eq!(editor.cursor_position(), (2, 0));
}

#[test]
fn vim_colon_number_jumps_to_one_based_line() {
    let mut editor = CodeEditor::new("one\ntwo\nthree\nfour\nfive", "txt")
        .with_vim_enabled(true);

    vim_keys(&mut editor, ":4\n");
    assert_eq!(editor.cursor_position(), (3, 0));

    vim_keys(&mut editor, ":99\n");
    assert_eq!(editor.cursor_position(), (4, 0));
}

#[test]
fn vim_colon_q_and_wq_exit_vim_mode() {
    let mut quit = CodeEditor::new("one\ntwo", "txt").with_vim_enabled(true);
    vim_keys(&mut quit, ":q\n");
    assert!(!quit.vim_enabled());
    assert_eq!(quit.vim_mode(), None);
    assert_eq!(quit.content(), "one\ntwo");

    let mut write_quit =
        CodeEditor::new("one\ntwo", "txt").with_vim_enabled(true);
    vim_keys(&mut write_quit, ":wq\n");
    assert!(!write_quit.vim_enabled());
    assert_eq!(write_quit.vim_mode(), None);
    assert_eq!(write_quit.content(), "one\ntwo");
}

#[test]
fn vim_colon_w_keeps_vim_mode_enabled() {
    let mut editor = CodeEditor::new("one\ntwo", "txt").with_vim_enabled(true);

    vim_keys(&mut editor, ":w\n");

    assert!(editor.vim_enabled());
    assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
    assert_eq!(editor.content(), "one\ntwo");
}

#[test]
fn vim_escape_cancels_command_without_moving() {
    let mut editor =
        CodeEditor::new("start\nfoo\nend", "txt").with_vim_enabled(true);
    let original = editor.cursor_position();

    vim_keys(&mut editor, "/foo\u{1b}");
    assert_eq!(editor.cursor_position(), original);

    vim_keys(&mut editor, "n");
    assert_eq!(editor.cursor_position(), original);
}

#[test]
fn vim_command_line_backspace_edits_search_text() {
    let mut editor =
        CodeEditor::new("start\nfoo\nend", "txt").with_vim_enabled(true);

    vim_keys(&mut editor, "/fop\u{8}o\n");

    assert_eq!(editor.cursor_position(), (1, 0));
}
