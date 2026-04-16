# CLAUDE.md — flowbit

## Project Overview

flowbit は GitHub Project v2 を読み取り専用で閲覧する TUI ツール。
Rust (edition 2024) で ratatui + octocrab を使って実装する。

## Documentation

- 要件定義: `docs/REQUIREMENTS.md`
- 実装計画: `docs/IMPLEMENTATION_PLAN.md`

## Architecture

Component パターン + Action ベースの副作用分離。

```
src/
  main.rs              # エントリポイント + tokio runtime
  app.rs               # App 構造体 + イベントループ
  event.rs             # AppEvent + キーイベントハンドリング
  action.rs            # Action enum（副作用の宣言的記述）
  config.rs            # TOML 設定 + token 解決
  cache.rs             # JSON キャッシュ
  api/
    client.rs          # GithubClient（octocrab GraphQL）
    queries.rs         # GraphQL クエリ定数
    types.rs           # API レスポンス DTO → ProjectItem 変換
  model/
    project_item.rs    # ProjectItem, ItemKind, StatusColumn
    filter.rs          # Filter + matches() + クエリパーサー
  views/
    mod.rs             # View trait
    board.rs           # カンバンボード
    list.rs            # テーブル一覧
    detail.rs          # 詳細ペイン
    help.rs            # ヘルプモーダル
  widgets/
    filter_input.rs    # フィルタ入力バー
    toast.rs           # トースト通知
    status_bar.rs      # ステータスバー
    loading.rs         # ローディング表示
```

## Key Design Decisions

- **View trait**: `handle_event() -> Action` + `render()` で責務分離
- **Action enum**: キーハンドリングと副作用実行を分離。テスタビリティ確保
- **ProjectItem DTO**: GitHub GraphQL の複雑な union 型を薄い独自型に変換。UI 層は GitHub 固有型に一切依存しない
- **フィルタは index ベース**: `Vec<ProjectItem>` を複製せず `Vec<usize>` で参照
- **mpsc チャネル**: ratatui（同期）と tokio（非同期）の橋渡し
- **octocrab 生 GraphQL**: graphql_client は使わない。スキーマ管理コスト回避

## Build & Run

```bash
cargo build
cargo run
```

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | TUI フレームワーク + ターミナルバックエンド |
| `octocrab` | GitHub API クライアント（GraphQL） |
| `tokio` | 非同期ランタイム |
| `serde` + `serde_json` + `toml` | シリアライゼーション |
| `chrono` | 日時処理 |
| `anyhow` | エラーハンドリング |
| `tracing` + `tracing-subscriber` + `tracing-appender` | ロギング |
| `dirs` | XDG パス解決 |
| `open` | クロスプラットフォームブラウザ起動 |
| `unicode-width` | Unicode 表示幅計算 |

## Conventions

- エラーハンドリングは `anyhow::Result` を基本とする
- ログは `tracing` マクロ (`info!`, `warn!`, `error!`) を使用
- GraphQL クエリは `api/queries.rs` に文字列定数として集約
- GitHub API レスポンス型は `api/types.rs` に閉じ込め、`model/` に変換して使用
- TUI の状態は `AppState` に集約。各 View は `&AppState` を参照のみ
- ユーザー入力 → `Action` → 副作用実行の一方向フロー

## Config

設定ファイル: `~/.config/flowbit/config.toml`
キャッシュ: `~/.cache/flowbit/cache.json`
ログ: `~/.local/state/flowbit/flowbit.log`

Token 解決順: `GITHUB_TOKEN` env → `gh auth token` → `config.toml` の `github.token`
設定ファイル未存在時: テンプレートを自動生成して案内

## Implementation Phases

1. **Phase 1**: Config + API + Domain Model（CLI で JSON ダンプ）
2. **Phase 2**: TUI Core + Board View
3. **Phase 3**: List View + Detail + Tabs + Browser
4. **Phase 4**: Filter + Default Filter + Cache + Polish
