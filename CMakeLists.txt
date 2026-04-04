cmake_minimum_required(VERSION 3.16)
project(tori_markdown_viewer VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets WebEngineWidgets)
find_package(PkgConfig REQUIRED)
pkg_check_modules(MD4C REQUIRED md4c-html)

qt_add_executable(tori_markdown_viewer
    src/main.cpp
    src/MainWindow.cpp
    src/ViewerWidget.cpp
    src/MarkdownRenderer.cpp
    src/ThemeManager.cpp
    src/AppSettings.cpp
    src/SourceHighlighter.cpp
    resources.qrc
)

qt_add_resources(tori_markdown_viewer "themes"
    PREFIX "/themes"
    FILES
        themes/normal_light.css
        themes/normal_dark.css
        themes/decorated_light.css
        themes/decorated_dark.css
)

target_link_libraries(tori_markdown_viewer
    PRIVATE
        Qt6::Widgets
        Qt6::WebEngineWidgets
        ${MD4C_LIBRARIES}
)

target_include_directories(tori_markdown_viewer
    PRIVATE
        src
        ${MD4C_INCLUDE_DIRS}
)

install(TARGETS tori_markdown_viewer
    RUNTIME DESTINATION bin
)
