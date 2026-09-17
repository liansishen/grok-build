//! Minimal-mode chrome is localized end-to-end under `GROK_LANGUAGE`.
//!
//! ```bash
//! cargo test -p xai-grok-pager-pty-harness --test minimal_zh_localization -- --nocapture
//! ```
//!
//! Unlike the sibling PTY scenarios these tests are deliberately NOT `#[ignore]`d: the localization
//! gate has to run in an ordinary `cargo test`, so a locale regression cannot pass by never
//! launching the binary. `pager_binary()` honours `PAGER_BINARY` and otherwise builds/uses
//! `target/debug/xai-grok-pager`.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn minimal_zh_cn_ui_is_chinese() {
    xai_grok_pager_pty_harness::scenarios::minimal_zh_localization::assert_minimal_ui_language(
        "zh-CN",
    )
    .await
    .expect("GROK_LANGUAGE=zh-CN must paint Chinese minimal chrome");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn minimal_en_ui_stays_english() {
    xai_grok_pager_pty_harness::scenarios::minimal_zh_localization::assert_minimal_ui_language(
        "en",
    )
    .await
    .expect("GROK_LANGUAGE=en must keep the minimal chrome English");
}
