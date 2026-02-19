pub struct FallingGroundEditor {
    chart_path: String,
    editor_state: EditorState,
    selection: SelectionState,
    view: ViewState,
    clipboard: ClipboardManager,
    undo: UndoManager,
    status: String,
    pending_toasts: Vec<(String, bool)>,
    i18n: crate::i18n::I18n,
}
