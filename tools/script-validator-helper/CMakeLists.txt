cmake_minimum_required(VERSION 3.20)
project(script_validator_helper LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(script-validator-helper
  src/main.cpp
)

target_include_directories(script-validator-helper PRIVATE
  ${CMAKE_CURRENT_SOURCE_DIR}/src
)

if (MSVC)
  target_compile_options(script-validator-helper PRIVATE /W4 /permissive- /utf-8)
else()
  target_compile_options(script-validator-helper PRIVATE -Wall -Wextra -Wpedantic)
endif()
