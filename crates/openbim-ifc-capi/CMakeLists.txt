# CMake package for the openbim-ifc C ABI (#41).
#
# Two ways in, one target set:
#
#   add_subdirectory(<checkout>/crates/openbim-ifc-capi)   # builds with cargo
#   find_package(openbim_ifc 0.1 CONFIG)                   # installed / archive
#
# Either way a consumer links `openbim_ifc::openbim_ifc`, which is the shared
# library unless OPENBIM_IFC_LINKAGE is STATIC; `openbim_ifc::openbim_ifc_shared`
# and `openbim_ifc::openbim_ifc_static` name one variant explicitly.
#
# Building this directory on its own and running `cmake --install` produces
# the package: the header, both libraries and the config files under
# lib/cmake/openbim_ifc. Release archives are exactly that install tree.
cmake_minimum_required(VERSION 3.21)

# One version, the crate's: the package can never disagree with the library.
file(STRINGS "${CMAKE_CURRENT_LIST_DIR}/Cargo.toml" _openbim_ifc_version_line
  REGEX "^version = \"[0-9]+\\.[0-9]+\\.[0-9]+\"$" LIMIT_COUNT 1)
string(REGEX REPLACE "^version = \"([^\"]+)\"$" "\\1" _openbim_ifc_version
  "${_openbim_ifc_version_line}")
if(NOT _openbim_ifc_version MATCHES "^[0-9]+\\.[0-9]+\\.[0-9]+$")
  message(FATAL_ERROR "openbim_ifc: no version found in ${CMAKE_CURRENT_LIST_DIR}/Cargo.toml")
endif()

project(openbim_ifc VERSION ${_openbim_ifc_version} LANGUAGES C)

if(NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  message(FATAL_ERROR "openbim_ifc supports 64-bit targets only")
endif()
if(WIN32 AND NOT MSVC)
  message(FATAL_ERROR "openbim_ifc on Windows requires MSVC (the Rust target is *-pc-windows-msvc)")
endif()

set(_openbim_ifc_install_default OFF)
if(PROJECT_IS_TOP_LEVEL)
  set(_openbim_ifc_install_default ON)
  if(NOT CMAKE_CONFIGURATION_TYPES AND NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "Build type" FORCE)
  endif()
endif()
option(OPENBIM_IFC_ENABLE_INSTALL "Install the openbim_ifc CMake package" ${_openbim_ifc_install_default})

set(OPENBIM_IFC_LINKAGE "SHARED" CACHE STRING "Library behind openbim_ifc::openbim_ifc (SHARED or STATIC)")
set_property(CACHE OPENBIM_IFC_LINKAGE PROPERTY STRINGS SHARED STATIC)
string(TOUPPER "${OPENBIM_IFC_LINKAGE}" _openbim_ifc_linkage)
if(NOT _openbim_ifc_linkage MATCHES "^(SHARED|STATIC)$")
  message(FATAL_ERROR "OPENBIM_IFC_LINKAGE must be SHARED or STATIC, not ${OPENBIM_IFC_LINKAGE}")
endif()
set(OPENBIM_IFC_CARGO_FEATURES "" CACHE STRING "Cargo features of openbim-ifc-capi to enable (e.g. rusty_alloc)")
set(OPENBIM_IFC_CARGO_TARGET "" CACHE STRING "Optional Rust target triple")
# Reuse an existing cargo target directory when the environment names one, so
# a CMake build next to `cargo build` does not compile the dependency tree twice.
if(DEFINED ENV{CARGO_TARGET_DIR} AND NOT "$ENV{CARGO_TARGET_DIR}" STREQUAL "")
  set(_openbim_ifc_target_dir_default "$ENV{CARGO_TARGET_DIR}")
else()
  set(_openbim_ifc_target_dir_default "${CMAKE_CURRENT_BINARY_DIR}/cargo-target")
endif()
set(OPENBIM_IFC_CARGO_TARGET_DIR "${_openbim_ifc_target_dir_default}" CACHE PATH "Cargo target directory")
find_program(OPENBIM_IFC_CARGO_EXECUTABLE cargo REQUIRED)

set(_openbim_ifc_crate_dir "${CMAKE_CURRENT_LIST_DIR}")
set(_openbim_ifc_header_dir "${_openbim_ifc_crate_dir}/include")
set(_openbim_ifc_artifacts "${OPENBIM_IFC_CARGO_TARGET_DIR}")
set(_openbim_ifc_target_args)
if(OPENBIM_IFC_CARGO_TARGET)
  list(APPEND _openbim_ifc_target_args --target "${OPENBIM_IFC_CARGO_TARGET}")
  string(APPEND _openbim_ifc_artifacts "/${OPENBIM_IFC_CARGO_TARGET}")
endif()
set(_openbim_ifc_feature_args)
if(OPENBIM_IFC_CARGO_FEATURES)
  list(APPEND _openbim_ifc_feature_args --features "${OPENBIM_IFC_CARGO_FEATURES}")
endif()

# Platform file names, all in one place. The SONAME / install name stamped by
# build.rs is the shared file name, so these must not be renamed on install.
set(_openbim_ifc_implib "")
if(WIN32)
  set(_openbim_ifc_shared "openbim_ifc_capi.dll")
  set(_openbim_ifc_implib "openbim_ifc_capi.dll.lib")
  set(_openbim_ifc_static "openbim_ifc_capi.lib")
  # `rustc --print native-static-libs` for x86_64-pc-windows-msvc; the C
  # runtime (/defaultlib:msvcrt) follows CMake's runtime-library selection.
  set(_openbim_ifc_static_deps "kernel32.lib;ntdll.lib;userenv.lib;ws2_32.lib;dbghelp.lib")
elseif(APPLE)
  set(_openbim_ifc_shared "libopenbim_ifc_capi.dylib")
  set(_openbim_ifc_static "libopenbim_ifc_capi.a")
  set(_openbim_ifc_static_deps "-lc;-lm")
else()
  set(_openbim_ifc_shared "libopenbim_ifc_capi.so")
  set(_openbim_ifc_static "libopenbim_ifc_capi.a")
  # `rustc --print native-static-libs` for x86_64-unknown-linux-gnu, minus
  # the -lc every C driver adds itself.
  set(_openbim_ifc_static_deps "gcc_s;util;rt;pthread;m;dl")
endif()

set(_openbim_ifc_byproducts)
foreach(_profile IN ITEMS debug release)
  foreach(_file IN ITEMS ${_openbim_ifc_shared} ${_openbim_ifc_static} ${_openbim_ifc_implib})
    list(APPEND _openbim_ifc_byproducts "${_openbim_ifc_artifacts}/${_profile}/${_file}")
  endforeach()
endforeach()

# One cargo invocation builds both libraries (crate-type staticlib + cdylib).
add_custom_target(openbim_ifc_cargo_build ALL
  COMMAND "${CMAKE_COMMAND}" -E env "CARGO_TARGET_DIR=${OPENBIM_IFC_CARGO_TARGET_DIR}"
          "${OPENBIM_IFC_CARGO_EXECUTABLE}" build --locked
          $<$<NOT:$<CONFIG:Debug>>:--release>
          --manifest-path "${_openbim_ifc_crate_dir}/Cargo.toml"
          --package openbim-ifc-capi ${_openbim_ifc_target_args} ${_openbim_ifc_feature_args}
  WORKING_DIRECTORY "${_openbim_ifc_crate_dir}"
  BYPRODUCTS ${_openbim_ifc_byproducts}
  COMMENT "cargo build -p openbim-ifc-capi"
  COMMAND_EXPAND_LISTS
  VERBATIM
)

function(_openbim_ifc_locations target file)
  set_target_properties(${target} PROPERTIES
    IMPORTED_LOCATION_DEBUG "${_openbim_ifc_artifacts}/debug/${file}"
    IMPORTED_LOCATION_RELEASE "${_openbim_ifc_artifacts}/release/${file}"
    IMPORTED_LOCATION_RELWITHDEBINFO "${_openbim_ifc_artifacts}/release/${file}"
    IMPORTED_LOCATION_MINSIZEREL "${_openbim_ifc_artifacts}/release/${file}"
    IMPORTED_CONFIGURATIONS "DEBUG;RELEASE;RELWITHDEBINFO;MINSIZEREL"
    INTERFACE_INCLUDE_DIRECTORIES "${_openbim_ifc_header_dir}")
endfunction()

add_library(openbim_ifc_shared SHARED IMPORTED GLOBAL)
add_library(openbim_ifc::openbim_ifc_shared ALIAS openbim_ifc_shared)
add_dependencies(openbim_ifc_shared openbim_ifc_cargo_build)
_openbim_ifc_locations(openbim_ifc_shared "${_openbim_ifc_shared}")
if(WIN32)
  set_target_properties(openbim_ifc_shared PROPERTIES
    IMPORTED_IMPLIB_DEBUG "${_openbim_ifc_artifacts}/debug/${_openbim_ifc_implib}"
    IMPORTED_IMPLIB_RELEASE "${_openbim_ifc_artifacts}/release/${_openbim_ifc_implib}"
    IMPORTED_IMPLIB_RELWITHDEBINFO "${_openbim_ifc_artifacts}/release/${_openbim_ifc_implib}"
    IMPORTED_IMPLIB_MINSIZEREL "${_openbim_ifc_artifacts}/release/${_openbim_ifc_implib}")
elseif(APPLE)
  # The dylib's install name is `@rpath/libopenbim_ifc_capi.dylib` (build.rs),
  # which dyld resolves only through the consumer's LC_RPATH. CMake adds that
  # rpath itself only when the dylib exists at configure time -- true for an
  # installed package, false on a fresh source build, where cargo has not run
  # yet; the consumer then links without one and fails to load
  # (axiolid/kernel#113). So this target supplies the directory itself.
  set_property(TARGET openbim_ifc_shared APPEND PROPERTY INTERFACE_LINK_OPTIONS
    "LINKER:-rpath,${_openbim_ifc_artifacts}/$<IF:$<CONFIG:Debug>,debug,release>")
endif()

add_library(openbim_ifc_static STATIC IMPORTED GLOBAL)
add_library(openbim_ifc::openbim_ifc_static ALIAS openbim_ifc_static)
add_dependencies(openbim_ifc_static openbim_ifc_cargo_build)
_openbim_ifc_locations(openbim_ifc_static "${_openbim_ifc_static}")
set_property(TARGET openbim_ifc_static PROPERTY INTERFACE_LINK_LIBRARIES "${_openbim_ifc_static_deps}")

add_library(openbim_ifc_selected INTERFACE)
add_library(openbim_ifc::openbim_ifc ALIAS openbim_ifc_selected)
if(_openbim_ifc_linkage STREQUAL "STATIC")
  target_link_libraries(openbim_ifc_selected INTERFACE openbim_ifc::openbim_ifc_static)
else()
  target_link_libraries(openbim_ifc_selected INTERFACE openbim_ifc::openbim_ifc_shared)
endif()

if(OPENBIM_IFC_ENABLE_INSTALL)
  # Fixed relative layout: the config file finds everything from its own
  # location, so the tree can be moved or unpacked anywhere.
  set(OPENBIM_IFC_SYSTEM_NAME "${CMAKE_SYSTEM_NAME}")
  if(CMAKE_SYSTEM_PROCESSOR MATCHES "^(aarch64|arm64|ARM64)$")
    set(OPENBIM_IFC_PROCESSOR_REGEX "^(aarch64|arm64|ARM64)$")
  else()
    set(OPENBIM_IFC_PROCESSOR_REGEX "^(x86_64|AMD64|amd64|x64)$")
  endif()
  if(OPENBIM_IFC_CARGO_TARGET MATCHES "^aarch64-")
    set(OPENBIM_IFC_PROCESSOR_REGEX "^(aarch64|arm64|ARM64)$")
  elseif(OPENBIM_IFC_CARGO_TARGET MATCHES "^x86_64-")
    set(OPENBIM_IFC_PROCESSOR_REGEX "^(x86_64|AMD64|amd64|x64)$")
  endif()
  set(OPENBIM_IFC_STATIC_DEPS "${_openbim_ifc_static_deps}")
  set(OPENBIM_IFC_SHARED_FILE "${_openbim_ifc_shared}")
  set(OPENBIM_IFC_STATIC_FILE "${_openbim_ifc_static}")
  set(OPENBIM_IFC_IMPLIB_FILE "${_openbim_ifc_implib}")
  # The name build.rs stamps into the shared library. Declaring it lets CMake
  # plan the consumer's runtime search path without reading the file.
  if(APPLE)
    set(OPENBIM_IFC_SHARED_SONAME "@rpath/${_openbim_ifc_shared}")
  else()
    set(OPENBIM_IFC_SHARED_SONAME "${_openbim_ifc_shared}")
  endif()
  set(OPENBIM_IFC_VERSION "${PROJECT_VERSION}")
  set(OPENBIM_IFC_VERSION_MAJOR "${PROJECT_VERSION_MAJOR}")
  set(OPENBIM_IFC_VERSION_MINOR "${PROJECT_VERSION_MINOR}")
  configure_file("${CMAKE_CURRENT_LIST_DIR}/cmake/openbim_ifc-config.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/openbim_ifc-config.cmake" @ONLY)
  configure_file("${CMAKE_CURRENT_LIST_DIR}/cmake/openbim_ifc-config-version.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/openbim_ifc-config-version.cmake" @ONLY)

  set(_profile_dir "${_openbim_ifc_artifacts}/$<IF:$<CONFIG:Debug>,debug,release>")
  install(FILES "${_openbim_ifc_header_dir}/openbim_ifc.h" DESTINATION include)
  if(WIN32)
    install(FILES "${_profile_dir}/${_openbim_ifc_shared}" DESTINATION bin)
    install(FILES "${_profile_dir}/${_openbim_ifc_implib}" DESTINATION lib)
  else()
    install(FILES "${_profile_dir}/${_openbim_ifc_shared}" DESTINATION lib)
  endif()
  install(FILES "${_profile_dir}/${_openbim_ifc_static}" DESTINATION lib)
  install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/openbim_ifc-config.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/openbim_ifc-config-version.cmake"
    DESTINATION lib/cmake/openbim_ifc)
  install(FILES
    "${_openbim_ifc_crate_dir}/../../LICENSE"
    "${_openbim_ifc_crate_dir}/../../NOTICE"
    DESTINATION share/doc/openbim_ifc)
endif()
