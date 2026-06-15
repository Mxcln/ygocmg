cmake_minimum_required(VERSION 3.20)
project(script_validator_helper LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if (NOT OCGCORE_DIR)
  message(FATAL_ERROR "OCGCORE_DIR is required")
endif()
if (NOT LUA_SRC_DIR)
  message(FATAL_ERROR "LUA_SRC_DIR is required")
endif()

file(GLOB LUA_SOURCES CONFIGURE_DEPENDS
  "${LUA_SRC_DIR}/src/*.c"
)
list(FILTER LUA_SOURCES EXCLUDE REGEX ".*/lua\\.c$")
list(FILTER LUA_SOURCES EXCLUDE REGEX ".*/luac\\.c$")
list(FILTER LUA_SOURCES EXCLUDE REGEX ".*/linit\\.c$")
list(FILTER LUA_SOURCES EXCLUDE REGEX ".*/onelua\\.c$")
set_source_files_properties(${LUA_SOURCES} PROPERTIES LANGUAGE CXX)

add_library(lua STATIC
  ${LUA_SOURCES}
)

target_include_directories(lua PUBLIC
  "${LUA_SRC_DIR}/src"
)

file(GLOB OCGCORE_SOURCES CONFIGURE_DEPENDS
  "${OCGCORE_DIR}/*.cpp"
)

add_library(ocgcore STATIC
  ${OCGCORE_SOURCES}
)

target_include_directories(ocgcore PUBLIC
  "${OCGCORE_DIR}"
  "${LUA_SRC_DIR}/src"
)

target_link_libraries(ocgcore PUBLIC lua)

add_executable(script-validator-helper
  src/main.cpp
)

target_include_directories(script-validator-helper PRIVATE
  ${CMAKE_CURRENT_SOURCE_DIR}/src
  "${OCGCORE_DIR}"
  "${LUA_SRC_DIR}/src"
)

target_link_libraries(script-validator-helper PRIVATE ocgcore lua)

if (MSVC)
  target_compile_definitions(lua PRIVATE _CRT_SECURE_NO_WARNINGS)
  target_compile_definitions(ocgcore PRIVATE _CRT_SECURE_NO_WARNINGS)
  target_compile_definitions(script-validator-helper PRIVATE _CRT_SECURE_NO_WARNINGS)
  target_compile_options(script-validator-helper PRIVATE /W4 /permissive- /utf-8)
else()
  target_compile_options(script-validator-helper PRIVATE -Wall -Wextra -Wpedantic)
endif()
