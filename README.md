# tori markdown viewer

Linux向けのMarkdownビューアです。eguiを使用しているため、デスクトップ環境に依存せず動作します。

## 機能

- **3種の表示モード切り替え**
  - **Normal** — GitHub風のシンプルなレンダリング
  - **Decorated** — 背景パネルと余白を加えた読みやすいレンダリング
  - **Source** — Markdownソーステキスト（読み取り専用）
- **折り返し切り替え** — ウィンドウ幅で折り返す／折り返さない（横スクロール）
- **フォントサイズ変更** — ツールバーのドラッグ値で 8〜72pt を自由に変更
- **フォント選択** — システムにインストールされた全フォントを一覧から選択可能（検索フィルタ付き）。「System default」を選ぶとegui組み込みフォントを使用
- **ライト／ダークモード**
  - デフォルトはOSのカラースキームに自動追従
  - ツールバーのボタンで Auto / Light / Dark に切り替え可能
- **ファイル変更検知** — 編集中のファイルをリアルタイム自動リロード
- **ドラッグ＆ドロップ** でファイルを開く
- **コマンドライン引数** でファイルを指定して起動可能
- **設定の永続化** — フォント・モード・ウィンドウサイズ等を次回起動時に復元

## スクリーンショット

| Normal（ライト） | Decorated（ダーク） |
|:-:|:-:|
| *(Normal Light)* | *(Decorated Dark)* |

## インストール

### AppImage（推奨）

[Releases](../../releases) から最新の `.AppImage` ファイルをダウンロードして実行します。

```bash
chmod +x tori_markdown_viewer-x86_64.AppImage
./tori_markdown_viewer-x86_64.AppImage
```

ファイルを指定して起動することもできます：

```bash
./tori_markdown_viewer-x86_64.AppImage README.md
```

AppImageはシステムへのインストール不要で、単体のファイルとして動作します。

### ソースからビルド

#### 必要なもの

- [Rust](https://rustup.rs/) 1.75 以上（`cargo` 付属）
- Linux の場合：以下のシステムライブラリ

```bash
sudo apt install \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libdbus-1-dev libgtk-3-dev pkg-config
```

#### ビルド

```bash
git clone https://github.com/kabeuchi-bird/tori_markdown_viewer.git
cd tori_markdown_viewer
cargo build --release
```

#### 起動

```bash
./target/release/tori_markdown_viewer [ファイル.md]
```

## 使い方

| 操作 | 方法 |
|------|------|
| ファイルを開く | ツールバーの「Open」ボタン、またはドラッグ＆ドロップ |
| 表示モード切り替え | ツールバーの `Normal` / `Decorated` / `Source` ボタン |
| 折り返し切り替え | ツールバーの `Wrap` チェックボックス |
| フォントサイズ変更 | ツールバーの `Size` ドラッグ値 |
| フォント変更 | ツールバーの `Font` コンボボックス（検索フィルタ付き） |
| カラースキーム | ツールバー右端の `Auto` / `Light` / `Dark` ボタン（クリックで切り替え） |

## 技術仕様

| 項目 | 詳細 |
|------|------|
| 言語 | Rust 2021 edition |
| GUIフレームワーク | [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) / [egui](https://github.com/emilk/egui) 0.28 |
| Markdownレンダリング | [egui_commonmark](https://github.com/lampsitter/egui_commonmark) 0.17（CommonMark準拠） |
| ファイル監視 | [notify](https://github.com/notify-rs/notify) 6 |
| フォント列挙 | [fontdb](https://github.com/RazrFalcon/fontdb) 0.22 |
| ファイルダイアログ | [rfd](https://github.com/PolyMeilex/rfd) 0.14（xdg-portal対応） |
| 設定保存先 | `~/.local/share/tori_markdown_viewer/` |

## ライセンス

[GNU General Public License v3.0](LICENSE)
