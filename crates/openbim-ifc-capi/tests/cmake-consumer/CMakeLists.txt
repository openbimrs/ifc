# A black-box CMake consumer of the openbim_ifc package (#41): builds the C
# smoke test (tests/c/smoke.c) as C11 and as C++17 against
# openbim_ifc::openbim_ifc and runs both through ctest. Driven by
# scripts/check-cmake.py, which copies nothing: it points this project at an
# installed prefix, an unpacked release archive, or the source tree.
#
#   -DOPENBIM_IFC_SOURCE_TREE=<crate dir>   add_subdirectory (cargo build)
#   -DCMAKE_PREFIX_PATH=<prefix>            find_package (installed package)
#   -DOPENBIM_IFC_LINKAGE=SHARED|STATIC
cmake_minimum_required(VERSION 3.21)
project(openbim_ifc_cmake_consumer LANGUAGES C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(OPENBIM_IFC_SOURCE_TREE)
  add_subdirectory("${OPENBIM_IFC_SOURCE_TREE}" openbim-ifc)
else()
  find_package(openbim_ifc 0.1 CONFIG REQUIRED)
endif()
if(NOT DEFINED OPENBIM_IFC_LINKAGE)
  set(OPENBIM_IFC_LINKAGE SHARED)
endif()

set(smoke_source "${CMAKE_CURRENT_LIST_DIR}/../c/smoke.c")
# The same file compiled as C++ proves the header's C++ compatibility.
configure_file("${smoke_source}" "${CMAKE_CURRENT_BINARY_DIR}/smoke.cpp" COPYONLY)
add_executable(smoke_c "${smoke_source}")
add_executable(smoke_cxx "${CMAKE_CURRENT_BINARY_DIR}/smoke.cpp")

foreach(smoke IN ITEMS smoke_c smoke_cxx)
  target_link_libraries(${smoke} PRIVATE openbim_ifc::openbim_ifc)
  if(MSVC)
    # C4996: MSVC deprecates fopen in favour of fopen_s, which C11 does not
    # require; the smoke test stays portable C.
    target_compile_options(${smoke} PRIVATE /W4 /WX /wd4996)
  else()
    target_compile_options(${smoke} PRIVATE -Wall -Wextra -Werror)
  endif()
  if(APPLE)
    # Resolve every symbol at link time, as GNU ld does.
    target_link_options(${smoke} PRIVATE LINKER:-undefined,error)
  endif()
  if(WIN32 AND OPENBIM_IFC_LINKAGE STREQUAL "SHARED")
    # Windows has no rpath: the DLL must sit next to the executable.
    add_custom_command(TARGET ${smoke} POST_BUILD
      COMMAND "${CMAKE_COMMAND}" -E copy_if_different
              "$<TARGET_FILE:openbim_ifc::openbim_ifc_shared>" "$<TARGET_FILE_DIR:${smoke}>"
      VERBATIM)
  endif()
endforeach()

enable_testing()
add_test(NAME smoke_c COMMAND smoke_c)
add_test(NAME smoke_cxx COMMAND smoke_cxx)
