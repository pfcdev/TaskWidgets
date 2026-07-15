#include "layout_math.h"
#include "../common/json_string.h"

#include <cassert>
#include <string>

int main() {
    using taskbar_widgets::ClampHostWidth;
    using taskbar_widgets::LeftForWidget;
    using taskbar_widgets::PositionedWidget;

    assert(LeftForWidget(PositionedWidget{200, 100, 0}, 1000) == 800);
    assert(LeftForWidget(PositionedWidget{200, 50, -20}, 1000) == 280);
    assert(LeftForWidget(PositionedWidget{200, 200, 900}, 1000) == 1440);
    assert(ClampHostWidth(-10) == 1);
    assert(ClampHostWidth(5000) == 4096);

    std::string decoded;
    assert(taskbar_widgets::json::ExtractStringUtf8(
        "{\"location\":\"\\u0130stanbul, \\u0130stanbul\"}", "location", decoded));
    assert(decoded == "\xC4\xB0stanbul, \xC4\xB0stanbul");
    assert(taskbar_widgets::json::ExtractStringUtf8(
        "{\"condition\":\"\\uD83C\\uDF24\"}", "condition", decoded));
    assert(decoded == "\xF0\x9F\x8C\xA4");
    assert(taskbar_widgets::json::ExtractStringUtf8(
        "{\"title\":\"A \\\"quoted\\\" title\"}", "title", decoded));
    assert(decoded == "A \"quoted\" title");
    assert(!taskbar_widgets::json::ExtractStringUtf8(
        "{\"location\":\"\\u013X\"}", "location", decoded));
    return 0;
}
