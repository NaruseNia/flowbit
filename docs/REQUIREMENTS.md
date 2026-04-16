# flowbit Requirements Definition (v1)

GitHub Project v2 を読み取り専用で閲覧する TUI ツール。
単一 Project を対象に、Status ベースの Board 表示と List 表示を提供する。

## Technology Stack

| Item | Choice |
|------|--------|
| Language | Rust (edition 2024) |
| TUI Framework | ratatui + crossterm |
| GitHub API | octocrab (GraphQL) |
| Async Runtime | tokio |
| Config Format | TOML (`~/.config/flowbit/config.toml`) |
| Auth | Personal Access Token (`GITHUB_TOKEN` env var priority) |
| Cache | JSON (`~/.cache/flowbit/cache.json`) |
| Log | `~/.local/state/flowbit/flowbit.log` + `RUST_LOG` |

## Target Data

- 対象は単一の GitHub Project v2
- 表示対象 item は **Issue / PullRequest のみ**
- DraftIssue およびその他 item 種別（RedactedItem 等）は v1 では非表示
- 対象外 item は読み飛ばし、UI 上の件数集計に含めない
- Status は設定された single-select field から取得する（field 名は設定可能）
- Status 未設定 item は `No Status` として扱う

## Data Fetching

- 起動時に GitHub GraphQL API から Project metadata と item 一覧を取得する
- `r` キーで手動リフレッシュ
- GraphQL ページネーションに対応し、Project item を全件取得する
- 500 件超の Project は起動時に注意表示
- API エラー時は前回取得済みキャッシュがあればそれを表示する（stale 表示）
- 最終更新時刻と stale 状態を UI に表示する

## Configuration

### Config File: `~/.config/flowbit/config.toml`

```toml
[github]
# token は GITHUB_TOKEN 環境変数を優先。未設定時のみここを参照
token = "ghp_xxxx"
# GitHub Enterprise 対応用（デフォルト: https://api.github.com）
api_base_url = "https://api.github.com"

[project]
owner = "NaruseNia"
number = 1
# Board のカラムに使う single-select field 名（デフォルト: "Status"）
status_field = "Status"

[ui]
# 起動時のデフォルトビュー: "board" | "list"
default_view = "board"
# ブラウザ起動コマンド（空なら OS デフォルト）
open_command = ""
# 日付表示フォーマット
date_format = "%Y-%m-%d"

[filter]
# 起動時に自動適用されるデフォルトフィルタ（全て任意）
# Esc で解除可能
assignee = "NaruseNia"
labels = ["bug"]
kind = "issue"        # "issue" | "pr"
status = "In Progress"
```

### Token Resolution Order

1. `GITHUB_TOKEN` 環境変数
2. `config.toml` の `github.token`
3. いずれも未設定 → 起動エラー

## UI

### Layout

タブ切り替え（`Tab` キー）で Board / List を切り替える。

### Board View

- Status ごとにカラムを横並び表示（カンバン）
- カラム順は Project の Status option 定義順に従う
- `No Status` カラムは末尾に配置
- 各カラム内の並び順は `updated_at desc`、tie-breaker は `number asc`
- カラム数が端末幅を超える場合はフォーカスカラム中心表示
- カラム幅は最小 20 文字、最大は端末幅に応じて動的計算
- 各カードにはタイトル・Issue 番号・assignee アイコンを表示

### List View

テーブル形式で以下の列を表示:

| Column | Description |
|--------|-------------|
| `repo` | リポジトリ名 |
| `#` | Issue/PR 番号 |
| `title` | タイトル |
| `kind` | Issue / PR |
| `status` | ステータス |
| `assignee` | アサイニー |
| `updated` | 最終更新日 |
| `labels` | ラベル（先頭 2 件） |

並び順は `updated_at desc`、tie-breaker は `number asc`。

### Detail Pane

選択 item のメタ情報を表示:

- title
- `repo#number`
- kind (Issue / PR)
- status
- assignees
- labels
- created_at
- updated_at
- URL

### Status Bar

画面下部に常時表示:

```
NORMAL | view: Board | filter: assignee:alice label:bug | 42/128 items | updated: 12:31:02
```

- 現在のモード（NORMAL / FILTER）
- アクティブビュー
- 適用中フィルタ
- 表示件数 / 総件数
- 最終更新時刻（stale 時は `[STALE]` 表示）

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `h` / `←` | 左に移動（Board: 前のカラム） |
| `l` / `→` | 右に移動（Board: 次のカラム） |
| `g` | 先頭に移動 |
| `G` | 末尾に移動 |
| `Tab` | Board ↔ List 切り替え |

### Actions

| Key | Action |
|-----|--------|
| `Enter` / `o` | 選択 item をブラウザで開く |
| `r` | データリフレッシュ |
| `/` | フィルタ入力モード開始 |
| `Esc` | フィルタ解除 / モーダル閉じる |
| `?` | キーヘルプ表示 |
| `q` | 終了 |

## Filter

### Filter Conditions (v1)

| Condition | Syntax Example | Match |
|-----------|---------------|-------|
| title | `fix login` | 部分一致 |
| label | `label:bug` | 完全一致 |
| assignee | `assignee:alice` | 完全一致 |
| kind | `is:pr` / `is:issue` | 種別指定 |
| status | `status:done` | 完全一致 |
| number | `#123` | 完全一致 |

- 複数条件は AND 結合
- 大文字小文字は区別しない
- `/` でフィルタモードに入り、入力 → Enter で適用
- `Esc` でフィルタ解除

### Default Filter

- `config.toml` の `[filter]` セクションで起動時の初期フィルタを設定可能
- 起動時に自動適用される
- `/` で上書き可能
- `Esc` で解除するとデフォルトフィルタも含めてクリアされる

## Error Handling

### Fatal Errors (TUI 起動前にエラー表示して終了)

- 認証トークン未設定
- 設定ファイル不正（TOML パースエラー）
- 指定 Project が存在しない / 権限不足
- 指定 status_field が見つからない

### Transient Errors (トースト通知 + stale 継続)

- API レート制限
- ネットワーク失敗
- 部分的なデータ欠落

### Error Display Strategy

- 致命的エラー → stderr 出力 + 非ゼロ終了
- 一時エラー → TUI 下部トースト通知（3 秒自動消去）
- 重要な状態変化 → ステータスバーに常時表示（stale, loading 等）

## Cache

- 前回成功レスポンスを `~/.cache/flowbit/cache.json` に保存
- 起動時: キャッシュがあれば即表示 → バックグラウンドで API 取得は v1 では行わない（手動 `r` のみ）
- 手動リフレッシュ成功時: キャッシュ更新
- 手動リフレッシュ失敗時: stale 表示で既存データ継続

## Logging

- ログ出力先: `~/.local/state/flowbit/flowbit.log`
- `RUST_LOG` 環境変数で詳細度制御
- TUI には要約のみ、詳細はログファイルへ
- API リクエスト/レスポンスの概要をログ出力

## Out of Scope (v1)

- item の作成・編集・移動などの書き込み操作
- 複数 Project 表示・切り替え
- バックグラウンド自動更新（定期ポーリング）
- GitHub Project v2 のカスタムフィールド完全再現
- Markdown レンダリング
- カスタムカラーテーマ
- 対話的なソート変更
- URL クリップボードコピー
