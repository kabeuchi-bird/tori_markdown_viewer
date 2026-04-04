#pragma once

#include "AppSettings.h"
#include <QStackedWidget>
#include <QString>

QT_BEGIN_NAMESPACE
class QWebEngineView;
class QPlainTextEdit;
QT_END_NAMESPACE

class SourceHighlighter;

class ViewerWidget : public QStackedWidget {
    Q_OBJECT
public:
    explicit ViewerWidget(QWidget *parent = nullptr);

    void setMode(ViewMode mode);
    ViewMode mode() const { return m_mode; }

    // Switch mode and update CSS in a single render pass (avoids double setHtml call).
    void setModeAndCss(ViewMode mode, const QString &css);

    void setWrap(bool wrap);
    bool wrap() const { return m_wrap; }

    void setFont(const QString &family, int size);

    // Load a file and render it. Call after setMode/setWrap/setFont are configured.
    void loadFile(const QString &path);

    // Re-render with current settings (e.g. after theme change).
    void refresh(const QString &css);

    QString currentMarkdown() const { return m_markdown; }

private:
    void applyWrapToSource();
    void applyFontToSource();

    QWebEngineView  *m_webView   = nullptr;
    QPlainTextEdit  *m_sourceEdit = nullptr;
    SourceHighlighter *m_highlighter = nullptr;

    ViewMode m_mode  = ViewMode::Normal;
    bool     m_wrap  = true;
    QString  m_fontFamily;
    int      m_fontSize = 14;
    QString  m_markdown;
    QString  m_currentCss;
};
