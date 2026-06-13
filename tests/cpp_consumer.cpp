#include <cassert>
#include <string>

#include "vlr-util/compat.hpp"

int main() {
    assert(vru_abi_version() == 1);
    assert(vlr::util::crc32("123456789") == 0xcbf43926u);

    vlr::util::CStringConversion conversion;
    std::wstring wide;
    assert(conversion.MultiByte_to_UTF16("Hello, world", wide).isSuccess());

    std::string narrow;
    assert(conversion.UTF16_to_MultiByte(wide, narrow).isSuccess());
    assert(narrow == "Hello, world");

    vlr::win32::CGUID guid;
    assert(guid.CreateGUID().isSuccess());
}
