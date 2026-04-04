cmake_minimum_required(VERSION 3.16)
project(tori_markdown_viewer VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets WebEngineWidgets)

# Locate md4c without relying on pkg-config (md4c-html.pc absent on some distros)
find_library(MD4C_HTML_LIBRARY NAMES md4c-html REQUIRED)
find_library(MD4C_LIBRARY     NAMES md4c      REQUIRED)
find_path(MD4C_INCLUDE_DIR    NAMES md4c-html.h REQUIRED)

qt_add_executable(tori_markdown_viewer
    src/main.cpp
    src/MainWindow.cpp
    src/ViewerWidget.cpp
    src/MarkdownRenderer.cpp
    src/ThemeManager.cpp
    src/AppSettings.cpp
    src/SourceHighlighter.cpp
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
        ${MD4C_HTML_LIBRARY}
        ${MD4C_LIBRARY}
)

target_include_directories(tori_markdown_viewer
    PRIVATE
        src
        ${MD4C_INCLUDE_DIR}
)

install(TARGETS tori_markdown_viewer
    RUNTIME DESTINATION bin
)
