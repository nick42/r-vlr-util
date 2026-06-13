#pragma once

#include <cstdint>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

#include "../r-vlr-util/r-vlr-util.h"

namespace vlr {

class SResult {
public:
    static constexpr std::int32_t Success = VRU_STATUS_OK;
    static constexpr std::int32_t Success_WithNuance = VRU_STATUS_INSUFFICIENT_BUFFER;

    constexpr SResult(std::int32_t value = Success) noexcept : value_{value} {}
    [[nodiscard]] constexpr bool isSuccess() const noexcept { return value_ >= 0; }
    [[nodiscard]] constexpr bool isFailure() const noexcept { return value_ < 0; }
    [[nodiscard]] constexpr std::int32_t asHRESULT() const noexcept { return value_; }
    constexpr operator std::int32_t() const noexcept { return value_; }

private:
    std::int32_t value_;
};

namespace util {

inline std::uint32_t crc32(std::string_view value) noexcept {
    return vru_crc32(reinterpret_cast<const std::uint8_t*>(value.data()), value.size());
}

class CStringConversion {
public:
    SResult MultiByte_to_UTF16(std::string_view input, std::wstring& output) const {
        std::size_t required{};
        auto status = vru_utf8_to_utf16(
            reinterpret_cast<const std::uint8_t*>(input.data()), input.size(), nullptr, 0, &required);
        if (status != VRU_STATUS_INSUFFICIENT_BUFFER && status != VRU_STATUS_OK) {
            return status;
        }
        output.resize(required);
        status = vru_utf8_to_utf16(
            reinterpret_cast<const std::uint8_t*>(input.data()), input.size(),
            reinterpret_cast<std::uint16_t*>(output.data()), output.size(), &required);
        return status;
    }

    SResult UTF16_to_MultiByte(std::wstring_view input, std::string& output) const {
        static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
        std::size_t required{};
        auto status = vru_utf16_to_utf8(
            reinterpret_cast<const std::uint16_t*>(input.data()), input.size(), nullptr, 0, &required);
        if (status != VRU_STATUS_INSUFFICIENT_BUFFER && status != VRU_STATUS_OK) {
            return status;
        }
        output.resize(required);
        status = vru_utf16_to_utf8(
            reinterpret_cast<const std::uint16_t*>(input.data()), input.size(),
            reinterpret_cast<std::uint8_t*>(output.data()), output.size(), &required);
        return status;
    }
};

} // namespace util

namespace ModuleContext::Runtime {
inline bool IsDebuggerAttached() noexcept {
    return vru_is_debugger_attached() != 0;
}
} // namespace ModuleContext::Runtime

namespace win32 {
class CGUID {
public:
    CGUID() = default;
    explicit CGUID(VruGuid value) noexcept : value_{value} {}

    SResult CreateGUID() noexcept { return vru_guid_create(&value_); }
    [[nodiscard]] const VruGuid& value() const noexcept { return value_; }

private:
    VruGuid value_{};
};
} // namespace win32

} // namespace vlr
