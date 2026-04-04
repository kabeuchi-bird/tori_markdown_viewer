#pragma once

#include "AppSettings.h"
#include <QObject>
#include <QString>
#include <QMap>

class ThemeManager : public QObject {
    Q_OBJECT
public:
    explicit ThemeManager(QObject *parent = nullptr);

    // Returns CSS for the given mode and dark flag.
    // For Source mode, returns an empty string (handled by QPlainTextEdit palette).
    QString getCss(ViewMode mode, bool dark) const;

    // Detects whether the OS is currently in dark mode.
    bool detectOsDark() const;

signals:
    void schemeChanged(bool dark);

public slots:
    void onOsSchemeChanged();

private:
    QString loadCss(const QString &resourcePath) const;

    mutable QMap<QString, QString> m_cache;
};
