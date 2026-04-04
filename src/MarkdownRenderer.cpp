#include "MarkdownRenderer.h"

#include <md4c-html.h>
#include <QString>
#include <QByteArray>

// md4c output callback: appends rendered HTML to a QByteArray
static void md4c_process_output(const MD_CHAR *text, MD_SIZE size, void *userdata) {
    auto *output = static_cast<QByteArray *>(userdata);
    output->append(text, static_cast<int>(size));
}

QString MarkdownRenderer::render(const QString &markdown,
                                 const QString &css,
                                 const QString &fontFamily,
                                 int fontSize,
                                 bool wrap)
{
    // Convert markdown to HTML fragment via md4c
    const QByteArray mdUtf8 = markdown.toUtf8();
    QByteArray htmlFragment;
    htmlFragment.reserve(mdUtf8.size() * 2);

    const unsigned int flags =
        MD_HTML_FLAG_DEBUG * 0 |
        0;  // default flags

    md_html(mdUtf8.constData(),
            static_cast<MD_SIZE>(mdUtf8.size()),
            md4c_process_output,
            &htmlFragment,
            MD_DIALECT_COMMONMARK,
            flags);

    // Build font CSS
    QString fontCss;
    const QString fontFamilyEscaped = fontFamily.isEmpty()
        ? "system-ui, sans-serif"
        : QString("\"%1\", system-ui, sans-serif").arg(fontFamily);

    fontCss = QString(
        "body { font-family: %1; font-size: %2pt; }\n"
        "code, pre { font-family: \"Monospace\", monospace; }\n"
    ).arg(fontFamilyEscaped).arg(fontSize);

    if (!wrap) {
        fontCss += "body { white-space: pre; overflow-x: auto; }\n"
                   "p, li, blockquote { white-space: nowrap; }\n";
    }

    // Assemble full HTML document
    const QString html = QString(
        "<!DOCTYPE html>\n"
        "<html>\n"
        "<head>\n"
        "<meta charset=\"UTF-8\">\n"
        "<style>\n"
        "%1\n"  // theme CSS
        "%2\n"  // font + wrap CSS
        "</style>\n"
        "</head>\n"
        "<body>\n"
        "%3\n"  // rendered HTML
        "</body>\n"
        "</html>\n"
    ).arg(css, fontCss, QString::fromUtf8(htmlFragment));

    return html;
}
