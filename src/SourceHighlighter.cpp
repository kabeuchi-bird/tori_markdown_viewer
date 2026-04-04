#include "SourceHighlighter.h"
#include <QApplication>
#include <QPalette>

SourceHighlighter::SourceHighlighter(QTextDocument *parent)
    : QSyntaxHighlighter(parent)
{
    const bool dark = QApplication::palette().color(QPalette::Window).lightness() < 128;

    // ---- Heading (#, ##, ...) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(^#{1,6}\s.*)");
        r.format.setForeground(dark ? QColor("#a78bfa") : QColor("#764ba2"));
        r.format.setFontWeight(QFont::Bold);
        m_rules.append(r);
    }

    // ---- Bold (**text** or __text__) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"((\*\*|__).+?\1)");
        r.format.setFontWeight(QFont::Bold);
        r.format.setForeground(dark ? QColor("#e2d9f3") : QColor("#2d2540"));
        m_rules.append(r);
    }

    // ---- Italic (*text* or _text_) — avoid matching bold ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"((?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)|(?<!_)_(?!_)(.+?)(?<!_)_(?!_))");
        r.format.setFontItalic(true);
        r.format.setForeground(dark ? QColor("#c4b5fd") : QColor("#5a4f7c"));
        m_rules.append(r);
    }

    // ---- Inline code (`code`) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(`[^`]+`)");
        r.format.setFontFamilies({"Monospace"});
        r.format.setForeground(dark ? QColor("#d4afff") : QColor("#6541b8"));
        r.format.setBackground(dark ? QColor("#1e1530") : QColor("#ede9ff"));
        m_rules.append(r);
    }

    // ---- Link [text](url) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(\[([^\]]+)\]\([^\)]+\))");
        r.format.setForeground(dark ? QColor("#60a5fa") : QColor("#0366d6"));
        r.format.setFontUnderline(true);
        m_rules.append(r);
    }

    // ---- Image ![alt](url) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(!\[([^\]]*)\]\([^\)]+\))");
        r.format.setForeground(dark ? QColor("#34d399") : QColor("#22863a"));
        m_rules.append(r);
    }

    // ---- Blockquote (>) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(^>\s?.*)");
        r.format.setForeground(dark ? QColor("#9ca3af") : QColor("#6a737d"));
        r.format.setFontItalic(true);
        m_rules.append(r);
    }

    // ---- Horizontal rule (--- or ***) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(^(\-{3,}|\*{3,}|_{3,})\s*$)");
        r.format.setForeground(dark ? QColor("#6d4c9e") : QColor("#dfe2e5"));
        m_rules.append(r);
    }

    // ---- List items (-, *, +, or 1.) ----
    {
        Rule r;
        r.pattern = QRegularExpression(R"(^(\s*([-*+]|\d+\.)\s))");
        r.format.setForeground(dark ? QColor("#a78bfa") : QColor("#667eea"));
        r.format.setFontWeight(QFont::Bold);
        m_rules.append(r);
    }

    // ---- Fenced code block (``` or ~~~) ----
    m_codeFenceStart = QRegularExpression(R"(^(`{3,}|~{3,}))");
    m_codeFenceEnd   = QRegularExpression(R"(^(`{3,}|~{3,})\s*$)");
    m_codeBlockFormat.setFontFamilies({"Monospace"});
    m_codeBlockFormat.setForeground(dark ? QColor("#e2d9f3") : QColor("#24292e"));
    m_codeBlockFormat.setBackground(dark ? QColor("#130f1f") : QColor("#f6f8fa"));
}

void SourceHighlighter::highlightBlock(const QString &text) {
    // Handle multi-line fenced code blocks (state 1 = inside code block)
    setCurrentBlockState(0);

    if (previousBlockState() == 1) {
        // We are inside a fenced code block
        setFormat(0, text.length(), m_codeBlockFormat);
        if (m_codeFenceEnd.match(text).hasMatch()) {
            setCurrentBlockState(0);
        } else {
            setCurrentBlockState(1);
        }
        return;
    }

    if (m_codeFenceStart.match(text).hasMatch()) {
        setFormat(0, text.length(), m_codeBlockFormat);
        setCurrentBlockState(1);
        return;
    }

    // Apply single-line rules
    for (const Rule &rule : m_rules) {
        auto it = rule.pattern.globalMatch(text);
        while (it.hasNext()) {
            const auto match = it.next();
            setFormat(match.capturedStart(), match.capturedLength(), rule.format);
        }
    }
}
