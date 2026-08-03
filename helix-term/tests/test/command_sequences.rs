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

#[tokio::test(flavor = "multi_thread")]
async fn single_command_final_window_close_exits_cleanly() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_close_config()?)
        .build()?;

    test_key_sequence(&mut app, Some("i<C-q>"), None, true).await
}

#[tokio::test(flavor = "multi_thread")]
async fn command_sequence_after_final_window_close_exits_cleanly() -> anyhow::Result<()> {
    let mut app = AppBuilder::new()
        .with_config(final_window_sequence_config()?)
        .build()?;

    test_key_sequence(&mut app, Some("i<C-q>"), None, true).await
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
