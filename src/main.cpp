#include "MainWindow.h"
#include <QApplication>
#include <QCommandLineParser>

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);
    app.setApplicationName("tori_markdown_viewer");
    app.setApplicationVersion("0.1.0");
    app.setOrganizationName("kabeuchi-bird");

    QCommandLineParser parser;
    parser.setApplicationDescription("Markdown viewer for Linux / KDE Plasma");
    parser.addHelpOption();
    parser.addVersionOption();
    parser.addPositionalArgument("file", QCoreApplication::translate("main", "Markdown file to open."));
    parser.process(app);

    MainWindow window;
    window.show();

    const QStringList args = parser.positionalArguments();
    if (!args.isEmpty()) {
        window.openFile(args.first());
    }

    return app.exec();
}
