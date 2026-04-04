#include "ViewerWidget.h"
#include "MarkdownRenderer.h"
#include "SourceHighlighter.h"

#include <QWebEngineView>
#include <QPlainTextEdit>
#include <QFile>
#include <QTextStream>
#include <QFont>
#include <QPalette>
#include <QScrollBar>
#include <QApplication>

static constexpr int PAGE_WEB    = 0;
static constexpr int PAGE_SOURCE = 1;

ViewerWidget::ViewerWidget(QWidget *parent)
    : QStackedWidget(parent)
{
    m_webView = new QWebEngineView(this);
    m_webView->setContextMenuPolicy(Qt::NoContextMenu);
    addWidget(m_webView);  // index 0

    m_sourceEdit = new QPlainTextEdit(this);
    m_sourceEdit->setReadOnly(true);
    m_sourceEdit->setLineWrapMode(QPlainTextEdit::WidgetWidth);
    addWidget(m_sourceEdit);  // index 1

    m_highlighter = new SourceHighlighter(m_sourceEdit->document());
}

void ViewerWidget::setModeAndCss(ViewMode mode, const QString &css) {
    m_currentCss = css;  // update CSS before setMode so it renders exactly once with correct CSS
    setMode(mode);
}

void ViewerWidget::setMode(ViewMode mode) {
    m_mode = mode;
    if (mode == ViewMode::Source) {
        setCurrentIndex(PAGE_SOURCE);
        m_sourceEdit->setPlainText(m_markdown);
    } else {
        setCurrentIndex(PAGE_WEB);
        if (!m_markdown.isEmpty()) {
            const QString html = MarkdownRenderer::render(
                m_markdown, m_currentCss, m_fontFamily, m_fontSize, m_wrap);
            m_webView->setHtml(html);
        }
    }
}

void ViewerWidget::setWrap(bool wrap) {
    m_wrap = wrap;
    if (m_mode == ViewMode::Source) {
        applyWrapToSource();
    } else if (!m_markdown.isEmpty()) {
        const QString html = MarkdownRenderer::render(
            m_markdown, m_currentCss, m_fontFamily, m_fontSize, m_wrap);
        m_webView->setHtml(html);
    }
}

void ViewerWidget::setFont(const QString &family, int size) {
    m_fontFamily = family;
    m_fontSize   = size;
    if (m_mode == ViewMode::Source) {
        applyFontToSource();
    } else if (!m_markdown.isEmpty()) {
        const QString html = MarkdownRenderer::render(
            m_markdown, m_currentCss, m_fontFamily, m_fontSize, m_wrap);
        m_webView->setHtml(html);
    }
}

void ViewerWidget::loadFile(const QString &path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return;
    }
    QTextStream stream(&file);
    stream.setEncoding(QStringConverter::Utf8);
    m_markdown = stream.readAll();

    if (m_mode == ViewMode::Source) {
        m_sourceEdit->setPlainText(m_markdown);
    } else {
        const QString html = MarkdownRenderer::render(
            m_markdown, m_currentCss, m_fontFamily, m_fontSize, m_wrap);
        m_webView->setHtml(html);
    }
}

void ViewerWidget::refresh(const QString &css) {
    m_currentCss = css;
    if (m_mode != ViewMode::Source && !m_markdown.isEmpty()) {
        const QString html = MarkdownRenderer::render(
            m_markdown, m_currentCss, m_fontFamily, m_fontSize, m_wrap);
        m_webView->setHtml(html);
    }
}

void ViewerWidget::applyWrapToSource() {
    m_sourceEdit->setLineWrapMode(
        m_wrap ? QPlainTextEdit::WidgetWidth : QPlainTextEdit::NoWrap);
}

void ViewerWidget::applyFontToSource() {
    QFont font;
    if (!m_fontFamily.isEmpty()) {
        font.setFamily(m_fontFamily);
    } else {
        font.setFamily("Monospace");
        font.setStyleHint(QFont::Monospace);
    }
    font.setPointSize(m_fontSize);
    m_sourceEdit->setFont(font);
}
