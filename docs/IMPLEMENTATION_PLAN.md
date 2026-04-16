# flowbit Implementation Plan

## Overview

全 4 フェーズで段階的に実装する。各フェーズは独立してビルド・動作確認可能な状態で完了させる。

## Architecture

```
src/
  main.rs              # エントリポイント + tokio runtime
  app.rs               # App 構造体 + イベントループ
  event.rs             # AppEvent 定義 + キーイベント → Action 変換
  action.rs            # Action enum（副作用の宣言的記述）
  config.rs            # TOML 設定読み込み + token 解決
  cache.rs             # JSON キャッシュ読み書き
  api/
    mod.rs             # pub mod
    client.rs          # GithubClient（octocrab GraphQL ラッパー）
    queries.rs         # GraphQL クエリ文字列定数
    types.rs           # API レスポンス → ProjectItem 変換 DTO
  model/
    mod.rs             # pub mod
    project_item.rs    # ProjectItem, ItemKind（ドメインモデル）
    filter.rs          # Filter 構造体 + matches() ロジック
  views/
    mod.rs             # View trait 定義
    board.rs           # Board（カンバン）ビュー
    list.rs            # List（テーブル）ビュー
    detail.rs          # 詳細ペイン
    help.rs            # ? キーヘルプモーダル
  widgets/
    mod.rs             # pub mod
    filter_input.rs    # フィルタ入力バー
    toast.rs           # トースト通知
    status_bar.rs      # ステータスバー
    loading.rs         # ローディングスピナー
```

### Core Design Patterns

**View trait**
```rust
pub trait View {
    fn handle_event(&mut self, event: AppEvent, state: &mut AppState) -> Action;
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState);
}
```

**Action enum**（副作用の宣言的記述）
```rust
pub enum Action {
    None,
    SwitchView(ActiveView),
    Refresh,
    OpenUrl(String),
    SetFilter(Filter),
    ClearFilter,
    ShowToast(String, ToastLevel),
    Quit,
}
```

**AppState**（UI とデータの分離）
```rust
pub struct AppState {
    pub items: Vec<ProjectItem>,
    pub status_columns: Vec<StatusColumn>,  // option 順保持
    pub filtered_indices: Vec<usize>,       // ID ベースフィルタ結果
    pub active_view: ActiveView,
    pub board_state: BoardState,
    pub list_state: ListState,
    pub filter: Filter,
    pub default_filter: Filter,             // config 由来
    pub last_updated: Option<DateTime<Utc>>,
    pub is_stale: bool,
    pub is_loading: bool,
    pub toast: Option<Toast>,
    pub show_help: bool,
}
```

---

## Phase 1: Foundation (Config + API + Domain Model)

**Goal**: 設定読み込み → GitHub API 接続 → データ取得 → stdout に JSON ダンプできる状態

### Tasks

1. **Cargo.toml 依存追加**
   - `tokio`, `serde`, `serde_json`, `toml`, `chrono`, `anyhow`, `tracing`, `tracing-subscriber`
   - `dirs` (XDG パス解決), `open` (ブラウザ起動), `unicode-width`

2. **config.rs**
   - `Config`, `GithubConfig`, `ProjectConfig`, `UiConfig`, `FilterConfig` 構造体
   - `~/.config/flowbit/config.toml` 読み込み
   - `GITHUB_TOKEN` 環境変数優先の token 解決
   - 設定ファイル未存在時のエラーメッセージ

3. **model/project_item.rs**
   - `ProjectItem`, `ItemKind`, `StatusColumn` 定義
   - `Display` trait 実装

4. **model/filter.rs**
   - `Filter` 構造体 + `matches(&self, &ProjectItem) -> bool`
   - フィルタ文字列パーサー（`label:bug assignee:alice` 形式）
   - デフォルトフィルタの `FilterConfig → Filter` 変換

5. **api/types.rs**
   - GraphQL レスポンスの serde 用中間型
   - 中間型 → `ProjectItem` 変換

6. **api/queries.rs**
   - Project metadata 取得クエリ（Status field の option 一覧）
   - Project items 取得クエリ（ページネーション対応）

7. **api/client.rs**
   - `GithubClient` 構造体
   - `fetch_project_metadata()` → `Vec<StatusColumn>`
   - `fetch_items()` → `Vec<ProjectItem>`（全ページ取得）

8. **cache.rs**
   - `save()` / `load()` で `~/.cache/flowbit/cache.json` 読み書き
   - `CachedSnapshot` 構造体（fetched_at + items）

9. **main.rs**
   - Config 読み込み → GithubClient 生成 → データ取得 → JSON 出力
   - エラー時はキャッシュから読み込み

### Completion Criteria
- `cargo run` で GitHub Project v2 の item 一覧が JSON で stdout に出力される
- キャッシュファイルが生成される
- 設定エラー時に適切なメッセージが表示される

### Technical Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| GraphQL union 型のデシリアライズ | serde の `#[serde(untagged)]` + 中間型で吸収。octocrab の生 GraphQL を使い、型安全は DTO 変換時に担保 |
| Status field 特定の複雑さ | metadata クエリで field 一覧取得 → name マッチ → option_id/name/order を HashMap 化 |
| ページネーション | cursor ベースで再帰取得。100 件/ページ × ループ |

---

## Phase 2: TUI Core (Event Loop + Board View)

**Goal**: ratatui で Board ビューが動作する最小 TUI

### Tasks

1. **action.rs**
   - `Action` enum 定義

2. **event.rs**
   - `AppEvent` enum（`Key(KeyEvent)`, `Tick`, `ApiResult(...)`)
   - crossterm イベントポーリング（`tokio::spawn` + `mpsc`）

3. **app.rs**
   - `App` 構造体（`AppState` + `GithubClient` + `CacheStore`）
   - イベントループ: poll → handle → render
   - `Action` に基づく副作用実行

4. **views/mod.rs**
   - `View` trait 定義
   - `ActiveView` enum

5. **views/board.rs**
   - `BoardView` 実装
   - Status カラム横並びレンダリング
   - hjkl ナビゲーション（カラム間 + カラム内移動）
   - フォーカスカラム中心表示（横スクロール代替）
   - カード表示（`#number title [assignee]`）

6. **widgets/status_bar.rs**
   - モード / ビュー / フィルタ / 件数 / 更新時刻 表示

7. **widgets/loading.rs**
   - 起動時ローディング表示

8. **main.rs 更新**
   - TUI 起動に切り替え
   - alternate screen + raw mode
   - panic hook でターミナル復帰

### Completion Criteria
- `cargo run` でカンバンボードが表示される
- hjkl でカード間を移動できる
- `q` で正常終了、ターミナルが壊れない
- ステータスバーに基本情報が表示される

---

## Phase 3: List View + Detail + Tabs

**Goal**: List ビュー、詳細ペイン、タブ切り替えが動作する

### Tasks

1. **views/list.rs**
   - `ListView` 実装
   - テーブル形式（repo, #, title, kind, status, assignee, updated, labels）
   - jk ナビゲーション
   - カラム幅の動的計算（`unicode-width` 使用）

2. **views/detail.rs**
   - `DetailPane` 実装
   - 選択 item のメタ情報表示
   - Board/List の右側または下部にスプリット表示

3. **Tab 切り替え**
   - `Tab` キーで Board ↔ List
   - ビュー切り替え時のフォーカス保持

4. **ブラウザ連携**
   - `Enter` / `o` で `open::that(url)` 実行
   - 失敗時トースト通知

5. **widgets/toast.rs**
   - トースト通知表示（3 秒自動消去）
   - `ToastLevel`（Info / Warn / Error）で色分け

6. **views/help.rs**
   - `?` キーでキーバインドヘルプモーダル

### Completion Criteria
- `Tab` で Board ↔ List が切り替わる
- List でテーブル形式の一覧が表示される
- 選択 item の詳細が表示される
- `Enter` でブラウザが開く
- `?` でヘルプが表示される

---

## Phase 4: Filter + Polish

**Goal**: フィルタ機能、デフォルトフィルタ、エラーハンドリング完成

### Tasks

1. **widgets/filter_input.rs**
   - `/` でフィルタ入力モード
   - テキスト入力 → Enter で適用
   - `Esc` でキャンセル/解除
   - `label:xxx assignee:xxx is:pr` パース

2. **デフォルトフィルタ統合**
   - `config.toml` の `[filter]` → 起動時に `AppState.filter` に適用
   - `AppState.default_filter` に保持
   - `/` で上書き時の挙動
   - `Esc` でデフォルトフィルタも含めクリア

3. **リフレッシュ完成**
   - `r` キーで API 再取得
   - loading 状態表示
   - 成功時キャッシュ更新
   - 失敗時トースト + stale 継続

4. **stale 表示**
   - ステータスバーに `[STALE]` 表示
   - 起動時キャッシュ表示 → stale 状態から開始

5. **ロギング統合**
   - `tracing` + `tracing-appender` でファイルログ
   - API リクエスト/レスポンス概要
   - エラー詳細

6. **エッジケース対応**
   - 空 Project（item 0 件）
   - Status field 未存在
   - 全 item が Status 未設定
   - 端末リサイズ対応
   - 極端に長いタイトル/ラベルの truncation

### Completion Criteria
- フィルタが動作する（title, label, assignee, kind, status）
- デフォルトフィルタが起動時に適用される
- `r` でリフレッシュが動作する
- API 失敗時に stale 表示で継続する
- ログファイルが出力される

---

## Dependency Graph

```
Phase 1 ──→ Phase 2 ──→ Phase 3 ──→ Phase 4
(API/Data)   (Board TUI)  (List/Detail) (Filter/Polish)
```

各フェーズは前フェーズの完了に依存する。Phase 内のタスクは概ね上から順だが、独立したものは並行可能。

## Additional Crates (Cargo.toml)

```toml
[dependencies]
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = "0.28"
octocrab = "0.49"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
dirs = "6"
open = "5"
unicode-width = "0.2"
```

## Technical Decisions

| Decision | Rationale |
|----------|-----------|
| octocrab 生 GraphQL（graphql_client 不使用） | スキーマ管理の手間を避ける。DTO で型安全を担保 |
| mpsc チャネルで TUI ↔ API 通信 | ratatui の同期レンダリングと tokio の非同期を橋渡し |
| フィルタ結果は index ベース | `Vec<ProjectItem>` 複製を避け、メモリ効率を確保 |
| `Action` enum で副作用分離 | イベントハンドリングとビジネスロジックのテスタビリティ確保 |
| `open` crate でブラウザ起動 | macOS/Linux/Windows のクロスプラットフォーム対応を crate に委譲 |
| `unicode-width` で表示幅計算 | 日本語/emoji 含むタイトルの正確な幅計算 |
