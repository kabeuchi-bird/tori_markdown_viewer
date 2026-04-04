# tori markdown viewer

Linux（KDE Plasma）向けのMarkdownビューアです。

## 機能

- **3種の表示モード切り替え**
  - **Normal** — GitHub風のシンプルなレンダリング
  - **Decorated** — グラデーション・シャドウを使った装飾マシマシレンダリング
  - **Source** — Markdownソーステキスト（シンタックスハイライト付き）
- **折り返し切り替え** — ウィンドウ幅で折り返す／折り返さない（横スクロール）
- **フォント・サイズ選択** — ツールバーから自由に変更可能
- **ライト／ダークモード**
  - デフォルトはOS（KDE Plasma）のカラースキームに自動追従
  - ツールバーのボタンでAuto / Light / Darkに切り替え可能
- **ファイル変更検知** — 編集中のファイルをリアルタイム自動リロード
- **ドラッグ＆ドロップ** でファイルを開く
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

- CMake 3.16 以上
- C++17 対応コンパイラ（GCC 10+ / Clang 12+）
- Qt6 (Widgets, WebEngineWidgets)
- [md4c](https://github.com/mity/md4c) ライブラリ

#### インストール（Debian/Ubuntu系）

```bash
sudo apt install cmake ninja-build qt6-base-dev qt6-webengine-dev libmd4c-dev
```

#### ビルド

```bash
git clone https://github.com/kabeuchi-bird/tori_markdown_viewer.git
cd tori_markdown_viewer
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j$(nproc)
```

#### 起動

```bash
./build/tori_markdown_viewer [ファイル.md]
```

## 使い方

| 操作 | 方法 |
|------|------|
| ファイルを開く | ツールバーの「Open」ボタン、またはドラッグ＆ドロップ |
| 表示モード切り替え | ツールバーの `Normal` / `Decorated` / `Source` ボタン |
| 折り返し切り替え | ツールバーの `Wrap` ボタン |
| フォント変更 | ツールバーのフォントコンボボックス・サイズスピンボックス |
| カラースキーム | ツールバーの `Auto` / `Light` / `Dark` ボタン（クリックで切り替え） |

## 技術仕様

| 項目 | 詳細 |
|------|------|
| 言語 | C++17 |
| GUIフレームワーク | Qt6 (Widgets + WebEngineWidgets) |
| Markdownパーサ | [md4c](https://github.com/mity/md4c)（CommonMark準拠） |
| HTMLレンダリング | QWebEngineView |
| ソースビュー | QPlainTextEdit + カスタム QSyntaxHighlighter |
| 設定保存先 | `~/.config/kabeuchi-bird/tori_markdown_viewer.ini` |

## ライセンス

[GNU General Public License v3.0](LICENSE)
