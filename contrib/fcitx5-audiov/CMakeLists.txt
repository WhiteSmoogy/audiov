cmake_minimum_required(VERSION 3.16)
project(audiov_fcitx5_addon LANGUAGES CXX)

find_package(PkgConfig REQUIRED)
pkg_check_modules(FCITX5_CORE REQUIRED IMPORTED_TARGET Fcitx5Core)
pkg_check_modules(FCITX5_UTILS REQUIRED IMPORTED_TARGET Fcitx5Utils)
pkg_check_modules(FCITX5_MODULE REQUIRED IMPORTED_TARGET Fcitx5Module)

add_library(audiovfcitx5 MODULE audiovfcitx5.cpp)
target_compile_features(audiovfcitx5 PRIVATE cxx_std_20)
target_link_libraries(
    audiovfcitx5
    PRIVATE
    PkgConfig::FCITX5_CORE
    PkgConfig::FCITX5_UTILS
    PkgConfig::FCITX5_MODULE
)

set_target_properties(
    audiovfcitx5
    PROPERTIES
    PREFIX "lib"
    OUTPUT_NAME "audiovfcitx5"
)

install(TARGETS audiovfcitx5 LIBRARY DESTINATION lib/fcitx5)
install(FILES audiovfcitx5.conf DESTINATION share/fcitx5/addon)
