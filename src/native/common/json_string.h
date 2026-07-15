#pragma once

#include <cstdint>
#include <string>
#include <string_view>

namespace taskbar_widgets::json {

inline bool HexValue(char value, uint32_t& result) {
    if (value >= '0' && value <= '9') {
        result = static_cast<uint32_t>(value - '0');
        return true;
    }
    if (value >= 'a' && value <= 'f') {
        result = static_cast<uint32_t>(value - 'a' + 10);
        return true;
    }
    if (value >= 'A' && value <= 'F') {
        result = static_cast<uint32_t>(value - 'A' + 10);
        return true;
    }
    return false;
}

inline bool ReadHex4(std::string_view value, size_t offset, uint32_t& result) {
    if (offset + 4 > value.size()) {
        return false;
    }

    result = 0;
    for (size_t index = offset; index < offset + 4; ++index) {
        uint32_t digit = 0;
        if (!HexValue(value[index], digit)) {
            return false;
        }
        result = (result << 4) | digit;
    }
    return true;
}

inline bool AppendUtf8CodePoint(std::string& output, uint32_t codePoint) {
    if (codePoint <= 0x7F) {
        output.push_back(static_cast<char>(codePoint));
    } else if (codePoint <= 0x7FF) {
        output.push_back(static_cast<char>(0xC0 | (codePoint >> 6)));
        output.push_back(static_cast<char>(0x80 | (codePoint & 0x3F)));
    } else if (codePoint >= 0xD800 && codePoint <= 0xDFFF) {
        return false;
    } else if (codePoint <= 0xFFFF) {
        output.push_back(static_cast<char>(0xE0 | (codePoint >> 12)));
        output.push_back(static_cast<char>(0x80 | ((codePoint >> 6) & 0x3F)));
        output.push_back(static_cast<char>(0x80 | (codePoint & 0x3F)));
    } else if (codePoint <= 0x10FFFF) {
        output.push_back(static_cast<char>(0xF0 | (codePoint >> 18)));
        output.push_back(static_cast<char>(0x80 | ((codePoint >> 12) & 0x3F)));
        output.push_back(static_cast<char>(0x80 | ((codePoint >> 6) & 0x3F)));
        output.push_back(static_cast<char>(0x80 | (codePoint & 0x3F)));
    } else {
        return false;
    }
    return true;
}

inline bool DecodeString(std::string_view raw, std::string& output) {
    output.clear();
    output.reserve(raw.size());

    for (size_t index = 0; index < raw.size(); ++index) {
        const char value = raw[index];
        if (value != '\\') {
            if (static_cast<unsigned char>(value) < 0x20) {
                return false;
            }
            output.push_back(value);
            continue;
        }

        if (++index >= raw.size()) {
            return false;
        }

        switch (raw[index]) {
            case '"':
            case '\\':
            case '/':
                output.push_back(raw[index]);
                break;
            case 'b':
                output.push_back('\b');
                break;
            case 'f':
                output.push_back('\f');
                break;
            case 'n':
                output.push_back('\n');
                break;
            case 'r':
                output.push_back('\r');
                break;
            case 't':
                output.push_back('\t');
                break;
            case 'u': {
                uint32_t codePoint = 0;
                if (!ReadHex4(raw, index + 1, codePoint)) {
                    return false;
                }
                index += 4;

                if (codePoint >= 0xD800 && codePoint <= 0xDBFF) {
                    if (index + 6 >= raw.size() || raw[index + 1] != '\\' ||
                        raw[index + 2] != 'u') {
                        return false;
                    }
                    uint32_t lowSurrogate = 0;
                    if (!ReadHex4(raw, index + 3, lowSurrogate) ||
                        lowSurrogate < 0xDC00 || lowSurrogate > 0xDFFF) {
                        return false;
                    }
                    codePoint = 0x10000 + ((codePoint - 0xD800) << 10) +
                                (lowSurrogate - 0xDC00);
                    index += 6;
                }

                if (!AppendUtf8CodePoint(output, codePoint)) {
                    return false;
                }
                break;
            }
            default:
                return false;
        }
    }

    return true;
}

inline bool ExtractStringUtf8(std::string_view json,
                              std::string_view key,
                              std::string& output) {
    std::string pattern = "\"";
    pattern.append(key);
    pattern.push_back('"');

    const size_t keyPosition = json.find(pattern);
    if (keyPosition == std::string_view::npos) {
        return false;
    }

    size_t position = json.find(':', keyPosition + pattern.size());
    if (position == std::string_view::npos) {
        return false;
    }
    position = json.find_first_not_of(" \t\r\n", position + 1);
    if (position == std::string_view::npos || json[position] != '"') {
        return false;
    }

    const size_t start = ++position;
    for (; position < json.size(); ++position) {
        if (json[position] == '"') {
            return DecodeString(json.substr(start, position - start), output);
        }
        if (json[position] == '\\') {
            if (++position >= json.size()) {
                return false;
            }
        }
    }

    return false;
}

}  // namespace taskbar_widgets::json
