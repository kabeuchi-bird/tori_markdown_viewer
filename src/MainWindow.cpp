#include "MainWindow.h"
#include "ViewerWidget.h"
#include "ThemeManager.h"

#include <QMenuBar>
#include <QMenu>
#include <QToolBar>
#include <QAction>
#include <QActionGroup>
#include <QFontComboBox>
#include <QSpinBox>
#include <QToolButton>
#include <QFileDialog>
#include <QFileSystemWatcher>
#include <QCloseEvent>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QMimeData>
#include <QUrl>
#include <QLabel>
#include <QStatusBar>
#include <QTimer>
#include <QApplication>
#include <QPalette>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , m_themeManager(new ThemeManager(this))
    , m_watcher(new QFileSystemWatcher(this))
{
    setWindowTitle("tori markdown viewer");
    setAcceptDrops(true);
    resize(1024, 768);

    m_viewer = new ViewerWidget(this);
    setCentralWidget(m_viewer);

    setupMenuBar();
    setupToolBar();

    // Restore settings
    m_schemeOverride = m_settings.colorScheme();
    m_viewer->setWrap(m_settings.wordWrap());
    m_viewer->setFont(m_settings.fontFamily(), m_settings.fontSize());

    // Restore mode
    const ViewMode savedMode = m_settings.viewMode();
    switch (savedMode) {
        case ViewMode::Decorated: m_decoratedAction->setChecked(true); break;
        case ViewMode::Source:    m_sourceAction->setChecked(true);    break;
        default:                  m_normalAction->setChecked(true);    break;
    }
    m_viewer->setMode(savedMode);

    // Restore wrap action state
    m_wrapAction->setChecked(m_settings.wordWrap());

    // Restore font combo / size spinner
    if (!m_settings.fontFamily().isEmpty()) {
        m_fontCombo->setCurrentFont(QFont(m_settings.fontFamily()));
    }
    m_sizeSpinner->setValue(m_settings.fontSize());

    // Update scheme button label
    switch (m_schemeOverride) {
        case ColorScheme::Light: m_schemeButton->setText("Light"); break;
        case ColorScheme::Dark:  m_schemeButton->setText("Dark");  break;
        default:                 m_schemeButton->setText("Auto");  break;
    }

    // Restore geometry
    const QByteArray geom = m_settings.windowGeometry();
    if (!geom.isEmpty()) {
        restoreGeometry(geom);
    }

    // Connect OS scheme changes
    connect(m_themeManager, &ThemeManager::schemeChanged,
            this, &MainWindow::onOsSchemeChanged);

    // File watcher
    connect(m_watcher, &QFileSystemWatcher::fileChanged,
            this, &MainWindow::onFileChanged);

    // Apply initial theme (trigger render)
    applyCurrentTheme();

    // Open last file if any
    const QString lastFile = m_settings.lastFile();
    if (!lastFile.isEmpty()) {
        openFile(lastFile);
    }
}

MainWindow::~MainWindow() = default;

void MainWindow::setupMenuBar() {
    QMenu *fileMenu = menuBar()->addMenu(tr("&File"));
    QAction *openAct = fileMenu->addAction(tr("&Open..."), this, &MainWindow::onOpenFile);
    openAct->setShortcut(QKeySequence::Open);
    fileMenu->addSeparator();
    fileMenu->addAction(tr("&Quit"), qApp, &QApplication::quit, QKeySequence::Quit);

    QMenu *viewMenu = menuBar()->addMenu(tr("&View"));
    m_normalAction    = viewMenu->addAction(tr("Normal"),    this, &MainWindow::onModeNormal);
    m_decoratedAction = viewMenu->addAction(tr("Decorated"), this, &MainWindow::onModeDecorated);
    m_sourceAction    = viewMenu->addAction(tr("Source"),    this, &MainWindow::onModeSource);

    for (QAction *a : {m_normalAction, m_decoratedAction, m_sourceAction}) {
        a->setCheckable(true);
    }

    m_modeGroup = new QActionGroup(this);
    m_modeGroup->addAction(m_normalAction);
    m_modeGroup->addAction(m_decoratedAction);
    m_modeGroup->addAction(m_sourceAction);
    m_modeGroup->setExclusive(true);
    m_normalAction->setChecked(true);
}

void MainWindow::setupToolBar() {
    QToolBar *bar = addToolBar(tr("Main Toolbar"));
    bar->setMovable(false);

    // Open button
    QAction *openAct = bar->addAction(tr("Open"), this, &MainWindow::onOpenFile);
    openAct->setToolTip(tr("Open file (Ctrl+O)"));
    bar->addSeparator();

    // Mode buttons (reuse actions created in menu)
    m_normalAction->setText(tr("Normal"));
    m_decoratedAction->setText(tr("Decorated"));
    m_sourceAction->setText(tr("Source"));

    bar->addAction(m_normalAction);
    bar->addAction(m_decoratedAction);
    bar->addAction(m_sourceAction);
    bar->addSeparator();

    // Word wrap toggle
    m_wrapAction = bar->addAction(tr("Wrap"), this, [this](bool checked) {
        onWrapToggled(checked);
    });
    m_wrapAction->setCheckable(true);
    m_wrapAction->setChecked(true);
    m_wrapAction->setToolTip(tr("Toggle word wrap"));
    bar->addSeparator();

    // Font combo
    bar->addWidget(new QLabel(tr(" Font: "), bar));
    m_fontCombo = new QFontComboBox(bar);
    m_fontCombo->setToolTip(tr("Select font"));
    connect(m_fontCombo, &QFontComboBox::currentFontChanged,
            this, &MainWindow::onFontChanged);
    bar->addWidget(m_fontCombo);

    // Size spinner
    bar->addWidget(new QLabel(tr(" Size: "), bar));
    m_sizeSpinner = new QSpinBox(bar);
    m_sizeSpinner->setRange(6, 72);
    m_sizeSpinner->setValue(14);
    m_sizeSpinner->setSuffix("pt");
    m_sizeSpinner->setToolTip(tr("Font size"));
    connect(m_sizeSpinner, QOverload<int>::of(&QSpinBox::valueChanged),
            this, &MainWindow::onFontSizeChanged);
    bar->addWidget(m_sizeSpinner);
    bar->addSeparator();

    // Color scheme button (cycles Auto → Light → Dark → Auto)
    m_schemeButton = new QToolButton(bar);
    m_schemeButton->setText("Auto");
    m_schemeButton->setToolTip(tr("Color scheme: Auto / Light / Dark"));
    connect(m_schemeButton, &QToolButton::clicked,
            this, &MainWindow::onSchemeButtonClicked);
    bar->addWidget(m_schemeButton);
}

void MainWindow::openFile(const QString &path) {
    if (path.isEmpty()) return;

    // Update file watcher
    if (!m_currentFile.isEmpty()) {
        m_watcher->removePath(m_currentFile);
    }
    m_currentFile = path;
    m_watcher->addPath(path);

    m_settings.setLastFile(path);

    setWindowTitle(QString("tori markdown viewer — %1")
                       .arg(QFileInfo(path).fileName()));

    m_viewer->loadFile(path);
    statusBar()->showMessage(path, 3000);
}

void MainWindow::closeEvent(QCloseEvent *event) {
    m_settings.setWindowGeometry(saveGeometry());
    event->accept();
}

void MainWindow::dragEnterEvent(QDragEnterEvent *event) {
    if (event->mimeData()->hasUrls()) {
        event->acceptProposedAction();
    }
}

void MainWindow::dropEvent(QDropEvent *event) {
    const QList<QUrl> urls = event->mimeData()->urls();
    if (!urls.isEmpty()) {
        openFile(urls.first().toLocalFile());
    }
}

void MainWindow::onOpenFile() {
    const QString path = QFileDialog::getOpenFileName(
        this,
        tr("Open Markdown File"),
        m_currentFile.isEmpty() ? QDir::homePath()
                                 : QFileInfo(m_currentFile).absolutePath(),
        tr("Markdown Files (*.md *.markdown *.txt);;All Files (*)")
    );
    if (!path.isEmpty()) {
        openFile(path);
    }
}

void MainWindow::onModeNormal() {
    m_viewer->setMode(ViewMode::Normal);
    m_settings.setViewMode(ViewMode::Normal);
    applyCurrentTheme();
}

void MainWindow::onModeDecorated() {
    m_viewer->setMode(ViewMode::Decorated);
    m_settings.setViewMode(ViewMode::Decorated);
    applyCurrentTheme();
}

void MainWindow::onModeSource() {
    m_viewer->setMode(ViewMode::Source);
    m_settings.setViewMode(ViewMode::Source);
}

void MainWindow::onWrapToggled(bool checked) {
    m_viewer->setWrap(checked);
    m_settings.setWordWrap(checked);
}

void MainWindow::onFontChanged(const QFont &font) {
    m_settings.setFontFamily(font.family());
    m_viewer->setFont(font.family(), m_sizeSpinner->value());
}

void MainWindow::onFontSizeChanged(int size) {
    m_settings.setFontSize(size);
    m_viewer->setFont(m_fontCombo->currentFont().family(), size);
}

void MainWindow::onSchemeButtonClicked() {
    // Cycle: Auto → Light → Dark → Auto
    switch (m_schemeOverride) {
        case ColorScheme::Auto:  m_schemeOverride = ColorScheme::Light; break;
        case ColorScheme::Light: m_schemeOverride = ColorScheme::Dark;  break;
        case ColorScheme::Dark:  m_schemeOverride = ColorScheme::Auto;  break;
    }

    switch (m_schemeOverride) {
        case ColorScheme::Light: m_schemeButton->setText("Light"); break;
        case ColorScheme::Dark:  m_schemeButton->setText("Dark");  break;
        default:                 m_schemeButton->setText("Auto");  break;
    }

    m_settings.setColorScheme(m_schemeOverride);
    applyCurrentTheme();
}

void MainWindow::onOsSchemeChanged(bool /*dark*/) {
    if (m_schemeOverride == ColorScheme::Auto) {
        applyCurrentTheme();
    }
}

void MainWindow::onFileChanged(const QString &path) {
    // Re-add path in case it was replaced (inotify removes after delete)
    QTimer::singleShot(100, this, [this, path]() {
        if (!m_watcher->files().contains(path)) {
            m_watcher->addPath(path);
        }
        m_viewer->loadFile(path);
        statusBar()->showMessage(tr("File reloaded: %1").arg(path), 2000);
    });
}

void MainWindow::applyCurrentTheme() {
    const bool dark = isDark();
    const ViewMode mode = m_viewer->mode();
    const QString css = m_themeManager->getCss(mode, dark);
    m_viewer->refresh(css);
}

bool MainWindow::isDark() const {
    switch (m_schemeOverride) {
        case ColorScheme::Light: return false;
        case ColorScheme::Dark:  return true;
        default:                 return m_themeManager->detectOsDark();
    }
}
