#include "ThemeManager.h"

#include <QApplication>
#include <QFile>
#include <QStyleHints>
#include <QPalette>

ThemeManager::ThemeManager(QObject *parent)
    : QObject(parent)
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    // colorSchemeChanged and Qt::ColorScheme were added in Qt 6.5
    connect(QApplication::styleHints(), &QStyleHints::colorSchemeChanged,
            this, &ThemeManager::onOsSchemeChanged);
#endif
}

QString ThemeManager::getCss(ViewMode mode, bool dark) const {
    if (mode == ViewMode::Source) {
        return QString();
    }

    QString key;
    if (mode == ViewMode::Decorated) {
        key = dark ? ":/themes/decorated_dark.css" : ":/themes/decorated_light.css";
    } else {
        key = dark ? ":/themes/normal_dark.css" : ":/themes/normal_light.css";
    }

    return loadCss(key);
}

bool ThemeManager::detectOsDark() const {
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    // Qt 6.5+ provides a reliable colorScheme() query
    const auto scheme = QApplication::styleHints()->colorScheme();
    if (scheme == Qt::ColorScheme::Dark)  return true;
    if (scheme == Qt::ColorScheme::Light) return false;
#endif
    // Fallback for Qt < 6.5: check palette window luminance
    const QColor bg = QApplication::palette().color(QPalette::Window);
    return bg.lightness() < 128;
}

void ThemeManager::onOsSchemeChanged() {
    emit schemeChanged(detectOsDark());
}

QString ThemeManager::loadCss(const QString &resourcePath) const {
    if (m_cache.contains(resourcePath)) {
        return m_cache.value(resourcePath);
    }

    QFile file(resourcePath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return QString();
    }

    const QString css = QString::fromUtf8(file.readAll());
    m_cache.insert(resourcePath, css);
    return css;
}
