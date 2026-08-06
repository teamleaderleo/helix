use super::*;

fn keymap_config(keys: &str) -> anyhow::Result<Config> {
    let raw: helix_term::config::ConfigRaw = toml::from_str(keys)?;

    Ok(Config {
        theme: raw.theme,
        keys: raw.keys.unwrap_or_default(),
        editor: test_editor_config(),
    })
}

fn final_window_close_config() -> anyhow::Result<Config> {
    keymap_config(
        r#"
        [keys.insert]
        C-q = "wclose"
        "#,
    )
}

fn final_window_sequence_config() -> anyhow::Result<Config> {
    keymap_config(
        r#"
        [keys.insert]
        C-q = ["wclose", "normal_mode"]
        "#,
    )
}

fn normal_mode_final_window_sequence_config() -> anyhow::Result<Config> {
    keymap_config(
        r#"
        [keys.normal]
        C-q = ["wclose", "move_char_right"]
        "#,
    )
}

fn ordinary_sequence_config() -> anyhow::Result<Config> {
    keymap_config(
        r#"
        [keys.normal]
        C-q = ["insert_mode", "normal_mode"]
        "#,
    )
}

fn macro_final_window_sequence_config() -> anyhow::Result<Config> {
    keymap_config(
        r#"
        [keys.normal]
        C-x = "wclose"
        C-q = "@<C-x>l"
        "#,
    )
}

fn recorded_macro_config() -> anyhow::Result<Config> {
    keymap_config(
        r#"
        [keys.normal]
        C-x = "wclose"
        "#,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn single_command_final_window_close_exits_cleanly() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_close_config()?)
        .build()?;

    test_key_sequence(&mut app, Some("i<C-q>"), None, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn normal_mode_sequence_after_final_window_close_exits_cleanly() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(normal_mode_final_window_sequence_config()?)
        .build()?;

    test_key_sequence(&mut app, Some("<C-q>"), None, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn ordinary_command_sequence_runs_to_completion() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(ordinary_sequence_config()?)
        .build()?;

    test_key_sequence(
        &mut app,
        Some("<C-q>"),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            helpers::assert_status_not_error(&app.editor);
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn command_sequence_continues_when_another_window_remains() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_sequence_config()?)
        .build()?;

    test_key_sequence(
        &mut app,
        Some("<C-w>vi<C-q>"),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            helpers::assert_status_not_error(&app.editor);
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_final_window_close_keeps_sequence_context_alive() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_sequence_config()?)
        .build()?;

    test_key_sequence(
        &mut app,
        Some("iX<C-q>"),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            assert!(app.editor.is_err());
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn macro_stops_after_final_window_close() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(macro_final_window_sequence_config()?)
        .build()?;

    test_key_sequence(&mut app, Some("<C-q>"), None, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn macro_continues_when_another_window_remains() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(macro_final_window_sequence_config()?)
        .build()?;

    test_key_sequence(
        &mut app,
        Some("<C-w>v<C-q>"),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            assert!(app.editor.macro_replaying.is_empty());
            helpers::assert_status_not_error(&app.editor);
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_close_cleans_configured_macro_replay_state() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(macro_final_window_sequence_config()?)
        .build()?;

    test_key_sequence(
        &mut app,
        Some("iX<esc><C-q>"),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            assert!(app.editor.macro_replaying.is_empty());
            assert!(app.editor.is_err());
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn recorded_macro_stops_after_final_window_close() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(recorded_macro_config()?)
        .build()?;

    // Record a macro while two views exist. Replaying it with one remaining
    // view closes the editor on the first recorded key; later recorded keys
    // must not be dispatched against the empty editor.
    test_key_sequence(&mut app, Some("<C-w>vQ<C-x>lQq"), None, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn recorded_macro_continues_and_cleans_replay_state() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(recorded_macro_config()?)
        .build()?;

    // Record and replay the close-plus-movement macro while enough views
    // remain for both executions. The replay stack must be empty afterward.
    test_key_sequence(
        &mut app,
        Some("<C-w>v<C-w>vQ<C-x>lQq"),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            assert!(app.editor.macro_replaying.is_empty());
            helpers::assert_status_not_error(&app.editor);
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn counted_repeat_stops_after_replayed_final_window_close() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_sequence_config()?)
        .build()?;

    // Record an insert-mode close while another view remains. Replaying that
    // insertion twice closes the final view on the first iteration; the repeat
    // loop must not start a second iteration against the empty editor.
    test_key_sequence(&mut app, Some("<C-w>vi<C-q>2."), None, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn counted_repeat_clears_count_when_editor_remains() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_sequence_config()?)
        .build()?;

    // Three views allow the original insert and one dot-repeat to close two
    // views without terminating the editor. Command count cleanup remains
    // observable after the replay loop returns.
    test_key_sequence(
        &mut app,
        Some("<C-w>v<C-w>vi<C-q>."),
        Some(&|app| {
            assert_eq!(1, app.editor.tree.views().count());
            assert_eq!(helix_view::document::Mode::Normal, app.editor.mode());
            assert!(app.editor.count.is_none());
            helpers::assert_status_not_error(&app.editor);
        }),
        false,
    )
    .await
}
