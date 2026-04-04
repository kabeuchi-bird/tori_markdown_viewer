#pragma once

#include <QSyntaxHighlighter>
#include <QTextCharFormat>
#include <QRegularExpression>
#include <QVector>

class SourceHighlighter : public QSyntaxHighlighter {
    Q_OBJECT
public:
    explicit SourceHighlighter(QTextDocument *parent = nullptr);

protected:
    void highlightBlock(const QString &text) override;

private:
    struct Rule {
        QRegularExpression pattern;
        QTextCharFormat format;
    };

    QVector<Rule> m_rules;

    // Multi-line fenced code block state
    QTextCharFormat m_codeBlockFormat;
    QRegularExpression m_codeFenceStart;
    QRegularExpression m_codeFenceEnd;
};
