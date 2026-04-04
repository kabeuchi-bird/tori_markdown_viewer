#pragma once

#include <QSettings>
#include <QString>
#include <QByteArray>

enum class ViewMode {
    Normal,
    Decorated,
    Source
};

enum class ColorScheme {
    Auto,
    Light,
    Dark
};

class AppSettings {
public:
    AppSettings();

    ViewMode viewMode() const;
    void setViewMode(ViewMode mode);

    bool wordWrap() const;
    void setWordWrap(bool wrap);

    QString fontFamily() const;
    void setFontFamily(const QString &family);

    int fontSize() const;
    void setFontSize(int size);

    ColorScheme colorScheme() const;
    void setColorScheme(ColorScheme scheme);

    QString lastFile() const;
    void setLastFile(const QString &path);

    QByteArray windowGeometry() const;
    void setWindowGeometry(const QByteArray &geometry);

private:
    QSettings m_settings;
};
