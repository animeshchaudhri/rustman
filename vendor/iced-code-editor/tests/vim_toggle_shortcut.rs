use iced::widget::canvas;
use iced::{Event, Point, Rectangle, Size, keyboard, mouse};
use iced_code_editor::{CodeEditor, Message, VimMode};

fn key_event(
    key: &str,
    code: keyboard::key::Code,
    modifiers: keyboard::Modifiers,
) -> Event {
    let key = keyboard::Key::Character(key.into());
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Code(code),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn routed_message(editor: &mut CodeEditor, event: &Event) -> Option<Message> {
    editor.request_focus();
    let _ = editor.update(&Message::CanvasFocusGained);
    canvas::Program::<Message>::update(
        editor,
        &mut (),
        event,
        Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0)),
        mouse::Cursor::Unavailable,
    )
    .and_then(|action| action.into_inner().0)
}

#[test]
fn vim_toggle_shortcut_routes_real_keyboard_event() {
    let mut editor = CodeEditor::new("abc", "txt");
    let toggle_modifiers =
        keyboard::Modifiers::COMMAND | keyboard::Modifiers::ALT;

    let message = routed_message(
        &mut editor,
        &key_event("v", keyboard::key::Code::KeyV, toggle_modifiers),
    );
    assert!(matches!(message, Some(Message::ToggleVimMode)));

    let paste_message = routed_message(
        &mut editor,
        &key_event(
            "v",
            keyboard::key::Code::KeyV,
            keyboard::Modifiers::COMMAND,
        ),
    );
    assert!(!matches!(paste_message, Some(Message::ToggleVimMode)));
}

#[test]
fn write_shortcut_routes_to_shared_write_request() {
    let mut editor = CodeEditor::new("abc", "txt").with_vim_enabled(true);
    let save_modifiers = if cfg!(target_os = "macos") {
        keyboard::Modifiers::COMMAND
    } else {
        keyboard::Modifiers::CTRL
    };

    let message = routed_message(
        &mut editor,
        &key_event("s", keyboard::key::Code::KeyS, save_modifiers),
    );

    assert!(matches!(message, Some(Message::WriteRequested)));
    assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
}

#[test]
fn vim_toggle_message_preserves_content_and_reenters_normal() {
    let mut editor = CodeEditor::new("abc", "txt");
    let original = editor.content();

    let _ = editor.update(&Message::ToggleVimMode);
    assert!(editor.vim_enabled());
    assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
    assert_eq!(editor.content(), original);
    assert!(!editor.is_modified());

    let _ = editor.update(&Message::VimKey('i'));
    assert_eq!(editor.vim_mode(), Some(VimMode::Insert));

    let _ = editor.update(&Message::ToggleVimMode);
    assert!(!editor.vim_enabled());
    assert_eq!(editor.vim_mode(), None);
    assert_eq!(editor.content(), original);
    assert!(!editor.is_modified());

    let _ = editor.update(&Message::ToggleVimMode);
    assert_eq!(editor.vim_mode(), Some(VimMode::Normal));
    assert_eq!(editor.content(), original);
    assert!(!editor.is_modified());
}
