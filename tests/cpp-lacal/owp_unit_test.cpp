// CPP-OMS-003: OWP framing, LA-CAL names and the global-element envelope,
// without any network. Expected strings are written by hand from the spec
// rules quoted in docs/task-cpp-oms-003-lacal-runtime.md.
#include "ams_gra/lacal/owp.hpp"
#include "check.hpp"

#include <string>
#include <vector>

namespace owp = ams_gra::lacal::owp;

TEST_CASE(identifiers_follow_the_owp_grammar) {
    for (const char* good : {"a", "Z9", "task102-log", "sub-1", "a.b_c-d", "0"}) {
        CHECK(owp::is_identifier(good));
    }
    for (const char* bad : {"", "bad topic", "pool 7: \"hot\"", "a/b", "tab\there", "é", "a,b", "a:b"}) {
        CHECK(!owp::is_identifier(bad));
    }
}

TEST_CASE(global_element_names_use_one_formatter) {
    CHECK_EQ(owp::global_element_name("https://www.vdl.afrl.af.mil/programs/oam", "Log"), std::string("Log"));
    CHECK_EQ(owp::global_element_name("urn:test", "MessageA"), std::string("{urn:test}MessageA"));
    // Near misses are ordinary namespaces: no normalisation.
    CHECK_EQ(owp::global_element_name("https://www.vdl.afrl.af.mil/programs/oam/", "Log"),
             std::string("{https://www.vdl.afrl.af.mil/programs/oam/}Log"));
    CHECK_EQ(owp::global_element_name("http://www.vdl.afrl.af.mil/programs/oam", "Log"),
             std::string("{http://www.vdl.afrl.af.mil/programs/oam}Log"));
    CHECK_EQ(owp::global_element_name("", "Log"), std::string("{}Log"));
    CHECK(!owp::check_message_qname("urn:test", "MessageA").has_value());
    CHECK(owp::check_message_qname("urn:test", "").has_value());
    CHECK(owp::check_message_qname("urn:test", "Message A").has_value());
    CHECK(owp::check_message_qname("urn:te}st", "MessageA").has_value());
    CHECK(owp::check_message_qname("urn:test", "Mess{age").has_value());
    CHECK(owp::check_message_qname("urn: test", "MessageA").has_value());
    CHECK(owp::check_message_qname("urn:test", "Message\tA").has_value());
}

TEST_CASE(client_frames_are_exact) {
    CHECK_EQ(owp::build_init({"1.0"}, "002.5.0", "task102-log", true),
             std::string(R"(INIT {"versions":["1.0"],"schema":"002.5.0","verbose":true,"service_id":"task102-log"})"));
    CHECK_EQ(owp::build_pub("t", "{urn:test}MessageA", R"({"{urn:test}Count":7})"),
             std::string(R"(PUB t {"{urn:test}MessageA":{"{urn:test}Count":7}})"));
    CHECK_EQ(owp::build_pub("t", "Log", "{}"), std::string(R"(PUB t {"Log":{}})"));
    CHECK_EQ(owp::build_sub("sub-1", "Log", "task102-log", std::nullopt), std::string("SUB sub-1 Log task102-log"));
    CHECK_EQ(owp::build_sub("sub-2", "{urn:test}MessageA", "input-topic", std::string_view("worker-group-1")),
             std::string("SUB sub-2 {urn:test}MessageA input-topic worker-group-1"));
    CHECK_EQ(owp::build_unsub("sub-9"), std::string("UNSUB sub-9"));
    CHECK_EQ(owp::json_quote("a\"b\\c\n\x01"), std::string(R"("a\"b\\c\n\u0001")"));
}

TEST_CASE(server_frames_parse_like_the_spec) {
    using Kind = owp::ServerFrame::Kind;
    CHECK(owp::parse_server_frame("+OK").kind == Kind::ok);
    CHECK(owp::parse_server_frame("+OK  ").kind == Kind::ok);
    CHECK(owp::parse_server_frame("+OK extra").kind == Kind::invalid);
    auto err = owp::parse_server_frame("-ERR Illegal-State topic not allowed");
    CHECK(err.kind == Kind::err);
    CHECK_EQ(err.code, std::string("Illegal-State"));
    CHECK_EQ(err.details, std::string("topic not allowed"));
    auto bare = owp::parse_server_frame("-ERR\tInvalid-Message");
    CHECK(bare.kind == Kind::err);
    CHECK(bare.details.empty());
    CHECK(owp::parse_server_frame("-ERR").kind == Kind::invalid);
    CHECK(owp::parse_server_frame("-ERR Not-A-Code x").kind == Kind::invalid);
    auto msg = owp::parse_server_frame("MSG \t sub-1  {\"Log\": {}} ");
    CHECK(msg.kind == Kind::msg);
    CHECK_EQ(msg.sid, std::string("sub-1"));
    CHECK_EQ(msg.payload, std::string("{\"Log\": {}} "));
    CHECK(owp::parse_server_frame("MSG sub-1").kind == Kind::invalid);
    CHECK(owp::parse_server_frame("MSG bad/sid {}").kind == Kind::invalid);
    CHECK(owp::parse_server_frame("INFO {}").kind == Kind::info);
    CHECK(owp::parse_server_frame("INFO").kind == Kind::invalid);
    CHECK(owp::parse_server_frame("").kind == Kind::invalid);
    CHECK(owp::parse_server_frame("PING").kind == Kind::invalid);
    CHECK(owp::parse_server_frame("msg sub-1 {}").kind == Kind::invalid);  // keywords are case-sensitive
}

TEST_CASE(envelope_requires_exactly_the_subscribed_member) {
    auto ok = owp::unwrap_global_element(R"( { "Log" : {"A":[1,2,{"b":null}]} } )", "Log");
    CHECK(ok.ok);
    CHECK_EQ(std::string(ok.body), std::string(R"({"A":[1,2,{"b":null}]})"));
    // An escaped member name is compared after unescaping.
    auto escaped = owp::unwrap_global_element(R"({"Log":7})", "Log");
    CHECK(escaped.ok);
    CHECK_EQ(std::string(escaped.body), std::string("7"));
    auto clark = owp::unwrap_global_element(R"({"{urn:test}MessageA":{"x":"}"}})", "{urn:test}MessageA");
    CHECK(clark.ok);
    CHECK_EQ(std::string(clark.body), std::string(R"({"x":"}"})"));
    const std::vector<std::string> rejected = {
        "{}",
        R"({"Log":{},"Log":{}})",
        R"({"Log":{},"Other":{}})",
        R"({"LogMT":{}})",
        R"({"{https://www.vdl.afrl.af.mil/programs/oam}Log":{}})",
        R"(["Log",{}])",
        R"("Log")",
        R"({"Log":{})",
        R"({"Log":{}} x)",
        R"({"Log":{}}})",
        R"({"Log":[1,]})",
        R"({"Log":01})",
        R"({"Log":"\ud800"})",
        "{\"Log\":\"a\x01\"}",
        R"({"Log":tru})",
        R"({"Log" {}})",
        "",
    };
    for (const std::string& document : rejected) {
        auto result = owp::unwrap_global_element(document, "Log");
        CHECK(!result.ok);
        CHECK(!result.problem.empty());
    }
    // Deep nesting is bounded, not a stack overflow.
    std::string deep = R"({"Log":)";
    deep.append(5000, '[');
    deep.append(5000, ']');
    deep.push_back('}');
    CHECK(!owp::unwrap_global_element(deep, "Log").ok);
}

TEST_CASE(info_payload_is_checked) {
    const std::vector<std::string> offered = {"1.0"};
    CHECK(!owp::check_info(R"({"version":"1.0","server_id":"s","uuids":{},"system_label":"x"})", offered).has_value());
    CHECK(owp::check_info(R"({"version":"2.0","server_id":"s"})", offered).has_value());
    CHECK(owp::check_info(R"({"server_id":"s"})", offered).has_value());
    CHECK(owp::check_info(R"({"version":"1.0"})", offered).has_value());
    CHECK(owp::check_info(R"({"version":1,"server_id":"s"})", offered).has_value());
    CHECK(owp::check_info(R"({"version":"1.0","version":"1.0","server_id":"s"})", offered).has_value());
    CHECK(owp::check_info(R"([])", offered).has_value());
}

int main(int argc, char** argv) { return check::run_all("CPP_OMS_003_OWP_UNIT_COMPLETE", argc, argv); }
