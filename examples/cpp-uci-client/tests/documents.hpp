// CPP-OMS-008 TEST-ONLY: independently hand-written authentic UCI `Log`
// documents (compact OMS JSON, schema member order), shared by the wire peer
// and the receiver test. NEVER produced by the generated encoder.
//
// The base document is the Task 093 golden Log
// (tests/fixtures/service-generate/task093-real-log{25,26}.json, reused by
// Task 102 and CPP-OMS-003); the wire peer checks that, minus whitespace, its
// own base text equals that fixture. Variants change only bounded, schema-
// valid fields so the typed summary of each delivery is predictable.
#pragma once

#include <cstdlib>
#include <iostream>
#include <string>

namespace documents {

inline std::string replace_once(const std::string& text, const std::string& from, const std::string& to) {
    const std::size_t at = text.find(from);
    if (at == std::string::npos || text.find(from, at + 1) != std::string::npos) {
        std::cerr << "test setup: '" << from << "' must occur exactly once\n";
        std::exit(2);
    }
    std::string result = text;
    result.replace(at, from.size(), to);
    return result;
}

/// Task 093 golden body for `schema` ("002.5.0" or "002.6.0").
inline std::string golden(const std::string& schema) {
    return R"({"SecurityInformation":{"Classification":"U","OwnerProducer":[{"GovernmentIdentifier":"USA"},{"NATO_SpecialWord":"NATO:EXAMPLE"}]},)"
           R"("MessageHeader":{"SystemID":{"UUID":"12345678-1234-1234-8234-123456789abc"},"Timestamp":"2026-01-02T03:04:05Z","SchemaVersion":")" +
           schema +
           R"(","Mode":"LIVE"},)"
           R"("MessageData":{"Timestamp":"2026-01-02T03:04:06Z","ServiceUpTime":"PT01.2300S","Severity":"INFO","Label":"Task093","Details":"Independent real Log Binary codec qualification","Data":"0080FF"}})";
}

/// The n-th (1-based) variant of a reception sequence. Odd n: severity
/// WARNING, ServiceID/SubsystemID present, Data absent. Even n: severity
/// INFO, classification U, the golden body. The MessageData timestamp's
/// seconds field is the sequence number (n <= 59) so the order is visible.
inline std::string variant(const std::string& schema, int n) {
    std::string doc = golden(schema);
    char seconds[3];
    seconds[0] = static_cast<char>('0' + (n / 10) % 10);
    seconds[1] = static_cast<char>('0' + n % 10);
    seconds[2] = '\0';
    doc = replace_once(doc, R"("Timestamp":"2026-01-02T03:04:06Z")",
                       std::string(R"("Timestamp":"2026-01-02T03:05:)") + seconds + R"(Z")");
    if (n % 2 == 1) {
        doc = replace_once(doc, R"("Severity":"INFO")", R"("Severity":"WARNING")");
        doc = replace_once(doc, R"("ServiceUpTime":"PT01.2300S",)",
                           R"("ServiceUpTime":"PT01.2300S","ServiceID":{"UUID":"550e8400-e29b-41d4-a716-446655440108"},)"
                           R"("SubsystemID":{"UUID":"550e8400-e29b-41d4-a716-446655440208"},)");
        doc = replace_once(doc, R"(,"Data":"0080FF")", "");
    }
    return doc;
}

/// Severity name the typed summary must show for variant n.
inline const char* variant_severity(int n) { return n % 2 == 1 ? "WARNING" : "INFO"; }

/// Seconds field (two digits) of variant n's MessageData.Timestamp.
inline std::string variant_log_timestamp(int n) {
    std::string text = "2026-01-02T03:05:00Z";
    text[17] = static_cast<char>('0' + (n / 10) % 10);
    text[18] = static_cast<char>('0' + n % 10);
    return text;
}

/// Broker-admitted but decoder-rejected: Sleet validates the anonymous
/// xs:hexBinary of Log/MessageData/Data only as a string (Task 102 /
/// CPP-OMS-003 observation), the generated decoder rejects the odd digit count.
inline std::string decoder_rejected(const std::string& schema) {
    return replace_once(golden(schema), R"("Data":"0080FF")", R"("Data":"000")");
}

/// Broker-rejected: missing the required MessageData.Severity.
inline std::string broker_rejected(const std::string& schema) {
    return replace_once(golden(schema), R"("Severity":"INFO",)", "");
}

}  // namespace documents
