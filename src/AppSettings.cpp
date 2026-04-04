#include "AppSettings.h"

AppSettings::AppSettings()
    : m_settings("kabeuchi-bird", "tori_markdown_viewer")
{}

ViewMode AppSettings::viewMode() const {
    const QString val = m_settings.value("viewMode", "normal").toString();
    if (val == "decorated") return ViewMode::Decorated;
    if (val == "source")    return ViewMode::Source;
    return ViewMode::Normal;
}

void AppSettings::setViewMode(ViewMode mode) {
    QString val;
    switch (mode) {
        case ViewMode::Decorated: val = "decorated"; break;
        case ViewMode::Source:    val = "source";    break;
        default:                  val = "normal";    break;
    }
    m_settings.setValue("viewMode", val);
}

bool AppSettings::wordWrap() const {
    return m_settings.value("wordWrap", true).toBool();
}

void AppSettings::setWordWrap(bool wrap) {
    m_settings.setValue("wordWrap", wrap);
}

QString AppSettings::fontFamily() const {
    return m_settings.value("fontFamily", "").toString();
}

void AppSettings::setFontFamily(const QString &family) {
    m_settings.setValue("fontFamily", family);
}

int AppSettings::fontSize() const {
    return m_settings.value("fontSize", 14).toInt();
}

void AppSettings::setFontSize(int size) {
    m_settings.setValue("fontSize", size);
}

ColorScheme AppSettings::colorScheme() const {
    const QString val = m_settings.value("colorScheme", "auto").toString();
    if (val == "light") return ColorScheme::Light;
    if (val == "dark")  return ColorScheme::Dark;
    return ColorScheme::Auto;
}

void AppSettings::setColorScheme(ColorScheme scheme) {
    QString val;
    switch (scheme) {
        case ColorScheme::Light: val = "light"; break;
        case ColorScheme::Dark:  val = "dark";  break;
        default:                 val = "auto";  break;
    }
    m_settings.setValue("colorScheme", val);
}

QString AppSettings::lastFile() const {
    return m_settings.value("lastFile", "").toString();
}

void AppSettings::setLastFile(const QString &path) {
    m_settings.setValue("lastFile", path);
}

QByteArray AppSettings::windowGeometry() const {
    return m_settings.value("windowGeometry").toByteArray();
}

void AppSettings::setWindowGeometry(const QByteArray &geometry) {
    m_settings.setValue("windowGeometry", geometry);
}
