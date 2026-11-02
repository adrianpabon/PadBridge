cmake_minimum_required(VERSION 3.21)

# One self-contained exe: static CRT here, in ViGEmClient and in vcpkg's x64-windows-static SDL2.
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
if(NOT DEFINED VCPKG_TARGET_TRIPLET)
    set(VCPKG_TARGET_TRIPLET x64-windows-static)
endif()

project(PadBridge LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
# Nefarius' client for the ViGEmBus driver (MIT, archived upstream), pinned to its last commit.
FetchContent_Declare(ViGEmClient
    GIT_REPOSITORY https://github.com/nefarius/ViGEmClient.git
    GIT_TAG b66d02d57e32cc8595369c53418b843e958649b4)
FetchContent_MakeAvailable(ViGEmClient)

find_package(SDL2 CONFIG REQUIRED)

add_executable(PadBridge src/main.cpp)
target_compile_definitions(PadBridge PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX UNICODE _UNICODE)
target_link_libraries(PadBridge PRIVATE
    ViGEmClient::ViGEmClient
    $<IF:$<TARGET_EXISTS:SDL2::SDL2-static>,SDL2::SDL2-static,SDL2::SDL2>
    setupapi)
