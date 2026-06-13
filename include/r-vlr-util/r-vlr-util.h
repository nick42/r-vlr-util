#pragma once

#include <stddef.h>
#include <stdint.h>

#if defined(_WIN32)
#  if defined(R_VLR_UTIL_BUILD)
#    define VRU_API __declspec(dllexport)
#  else
#    define VRU_API __declspec(dllimport)
#  endif
#else
#  define VRU_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum {
    VRU_STATUS_OK = 0,
    VRU_STATUS_INSUFFICIENT_BUFFER = 1,
    VRU_STATUS_INVALID_ARGUMENT = -1,
    VRU_STATUS_CONVERSION_FAILED = -2,
    VRU_STATUS_NOT_IMPLEMENTED = -3
};

typedef struct VruGuid {
    uint32_t data1;
    uint16_t data2;
    uint16_t data3;
    uint8_t data4[8];
} VruGuid;

VRU_API uint32_t vru_abi_version(void);
VRU_API uint32_t vru_crc32(const uint8_t* data, size_t length);
VRU_API uint8_t vru_string_equal_utf8(
    const uint8_t* left,
    size_t left_length,
    const uint8_t* right,
    size_t right_length,
    uint8_t case_insensitive);
VRU_API int32_t vru_utf8_to_utf16(
    const uint8_t* input,
    size_t input_length,
    uint16_t* output,
    size_t output_capacity,
    size_t* required);
VRU_API int32_t vru_utf16_to_utf8(
    const uint16_t* input,
    size_t input_length,
    uint8_t* output,
    size_t output_capacity,
    size_t* required);
VRU_API uint8_t vru_file_exists_utf16(const uint16_t* path, size_t path_length);
VRU_API uint8_t vru_directory_exists_utf16(const uint16_t* path, size_t path_length);
VRU_API uint8_t vru_is_debugger_attached(void);
VRU_API int32_t vru_guid_create(VruGuid* output);

#ifdef __cplusplus
}
#endif
