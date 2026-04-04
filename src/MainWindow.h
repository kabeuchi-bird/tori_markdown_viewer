#pragma once

#include "AppSettings.h"
#include <QMainWindow>
#include <QString>

QT_BEGIN_NAMESPACE
class QAction;
class QActionGroup;
class QFontComboBox;
class QSpinBox;
class QToolButton;
class QFileSystemWatcher;
class QLabel;
QT_END_NAMESPACE

class ViewerWidget;
class ThemeManager;

class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override;

    void openFile(const QString &path);

protected:
    void closeEvent(QCloseEvent *event) override;
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dropEvent(QDropEvent *event) override;

private slots:
    void onOpenFile();
    void onModeNormal();
    void onModeDecorated();
    void onModeSource();
    void onWrapToggled(bool checked);
    void onFontChanged(const QFont &font);
    void onFontSizeChanged(int size);
    void onSchemeButtonClicked();
    void onOsSchemeChanged(bool dark);
    void onFileChanged(const QString &path);

private:
    void setupMenuBar();
    void setupToolBar();
    void applyCurrentTheme();
    bool isDark() const;

    ViewerWidget      *m_viewer      = nullptr;
    ThemeManager      *m_themeManager = nullptr;
    AppSettings        m_settings;

    // Toolbar widgets
    QAction           *m_normalAction    = nullptr;
    QAction           *m_decoratedAction = nullptr;
    QAction           *m_sourceAction    = nullptr;
    QActionGroup      *m_modeGroup       = nullptr;
    QAction           *m_wrapAction      = nullptr;
    QFontComboBox     *m_fontCombo       = nullptr;
    QSpinBox          *m_sizeSpinner     = nullptr;
    QToolButton       *m_schemeButton    = nullptr;

    QFileSystemWatcher *m_watcher    = nullptr;
    QString             m_currentFile;
    ColorScheme         m_schemeOverride = ColorScheme::Auto;
};
