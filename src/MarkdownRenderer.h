#pragma once

#include "AppSettings.h"
#include <QString>

class MarkdownRenderer {
public:
    // Renders markdown text to a full HTML document.
    // css      : theme CSS to embed in <style>
    // fontFamily: font family name (empty = use system default)
    // fontSize : font size in pt
    // wrap     : true = word-wrap enabled in body, false = nowrap
    static QString render(const QString &markdown,
                          const QString &css,
                          const QString &fontFamily,
                          int fontSize,
                          bool wrap);
};
