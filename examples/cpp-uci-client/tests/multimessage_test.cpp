// Focused receiver-core cases. Traffic is independent raw OWP against Sleet.
#include <chrono>
#include <fstream>
#include <stdexcept>
#include <string>
#include <vector>

#include "check.hpp"
#include "client.hpp"
#include "documents.hpp"
#include "raw_owp_client.hpp"

namespace app = cpp_uci_client;
namespace m = programs::oam;
using namespace std::chrono_literals;

namespace {
const std::string schema(app::release::schema_version);
using app::Family;
std::string port, rdma_json, many_endpoints;

app::ClientSettings settings(std::size_t capacity = 64) {
    app::ClientSettings s;
    s.url = "ws://127.0.0.1:" + port;
    s.service_id = "cpp-uci-client";
    s.queue_capacity = capacity;
    return s;
}

void start(app::Client& client) {
    if (client.connect() || client.subscribe() || client.subscribe(Family::rdma)) {
        throw std::runtime_error("could not establish combined session");
    }
    CHECK_EQ(client.subscription_id(Family::log), std::string("sub-1"));
    CHECK_EQ(client.subscription_id(Family::rdma), std::string("sub-2"));
}

raw::Client publisher() { return {"127.0.0.1", port, "cpp-uci-wire", schema, 5s}; }

void publish(raw::Client& wire, Family family) {
    const auto frame = family == Family::log ? "PUB uci.log {\"Log\":" + documents::variant(schema, 1) + "}"
                                             : "PUB uci.rdma " + rdma_json;
    CHECK_EQ(wire.command(frame), std::string("+OK"));
}

app::Next next(app::Client& client) {
    const auto deadline = app::Client::Clock::now() + 5s;
    while (app::Client::Clock::now() < deadline) {
        auto result = client.next(deadline);
        if (result.kind != app::Next::Kind::idle) {
            return result;
        }
    }
    return {};
}

TEST_CASE(shared_queue_overflow_and_pending_shutdown) {
    auto wire = publisher();
    app::Client client(settings(2));
    start(client);
    for (Family family : {Family::log, Family::rdma, Family::log, Family::rdma}) {
        publish(wire, family);
    }
    CHECK(check::eventually([&] { return client.receiver().queue().counters().rejected_full == 2; }, 5s));
    const auto outcome = next(client);
    CHECK(outcome.kind == app::Next::Kind::failure && outcome.exit_status == app::exit_status::session);
    CHECK(outcome.line.rfind("QUEUE-OVERFLOW application queue capacity=2", 0) == 0);
    const auto report = client.shutdown();
    CHECK(report.clean() && report.session_failed);
    CHECK_EQ(report.queue.accepted, std::uint64_t{2});
    CHECK_EQ(report.queue.rejected_full, std::uint64_t{2});
    CHECK_EQ(report.pending_discarded, std::size_t{2});
    CHECK_EQ(report.per_family[app::index(Family::log)].callbacks, std::uint64_t{2});
    CHECK_EQ(report.per_family[app::index(Family::rdma)].callbacks, std::uint64_t{2});
    CHECK(report.unsubscribe_lines.size() == 2 && report.unsubscribe_lines[0].rfind("UNSUBSCRIBED sid=sub-2 ", 0) == 0);
    CHECK_EQ(client.shutdown().pending_discarded, std::size_t{2});
}

TEST_CASE(rdma_unsubscribe_quiesces_only_its_own_sid) {
    auto wire = publisher();
    app::Client client(settings());
    start(client);
    publish(wire, Family::rdma);
    CHECK(next(client).kind == app::Next::Kind::message);
    std::string line;
    CHECK(client.unsubscribe(Family::rdma, line));
    CHECK(line.rfind("UNSUBSCRIBED sid=sub-2 ", 0) == 0);
    publish(wire, Family::rdma);
    publish(wire, Family::log);
    const auto received = next(client);
    CHECK(received.kind == app::Next::Kind::message && std::holds_alternative<app::LogSummary>(received.item.summary));
    const auto report = client.shutdown();
    CHECK(report.clean());
    publish(wire, Family::log);
    publish(wire, Family::rdma);
    CHECK_EQ(client.receiver().callbacks(Family::log), std::uint64_t{1});
    CHECK_EQ(client.receiver().callbacks(Family::rdma), std::uint64_t{1});
    CHECK_EQ(client.receiver().queue().counters().rejected_closed, std::uint64_t{0});
}

TEST_CASE(rdma_callback_failure_is_attributed_and_contained) {
    auto wire = publisher();
    app::Client client(settings(), app::summarize, [](const m::RDMAInitializeSetupMT&) -> app::RdmaSummary {
        throw std::runtime_error("do not echo this callback text");
    });
    start(client);
    publish(wire, Family::rdma);
    const auto outcome = next(client);
    CHECK(outcome.kind == app::Next::Kind::failure && outcome.exit_status == app::exit_status::session);
    CHECK(outcome.line.rfind("CALLBACK-FAILURE the typed RDMA_InitializeSetup", 0) == 0);
    CHECK(outcome.line.find("do not echo") == std::string::npos);
    const auto report = client.shutdown();
    CHECK(report.clean() && report.session_failed);
    CHECK_EQ(report.stats.handler_exceptions, std::uint64_t{1});
    CHECK_EQ(report.per_family[app::index(Family::rdma)].handler_exceptions, std::uint64_t{1});
    CHECK_EQ(report.per_family[app::index(Family::log)].handler_exceptions, std::uint64_t{0});
}

TEST_CASE(rdma_summary_caps_unbounded_endpoints) {
    const auto decoded = service_codec::decode_RDMAInitializeSetupMT(many_endpoints);
    CHECK(decoded.ok());
    if (decoded.ok()) {
        const auto summary = app::summarize_rdma(decoded.value());
        CHECK_EQ(summary.endpoint_count, std::size_t{6});
        CHECK_EQ(summary.shown(), std::size_t{4});
        CHECK_EQ(summary.endpoints[0].address, std::string("fE80::aBcD"));
        const auto line = app::format_summary(1, summary, schema);
        CHECK(line.find(" endpoints-omitted=2") != std::string::npos);
        CHECK(line.find("203.0.113.77") == std::string::npos);
        CHECK(line.find("do not echo") == std::string::npos);
    }
}
}  // namespace

int main(int argc, char** argv) {
    if (argc != 4) {
        return 2;
    }
    port = argv[1];
    std::ifstream envelope(argv[2]);
    std::ifstream body(argv[3]);
    if (!std::getline(envelope, rdma_json) || !std::getline(body, many_endpoints)) {
        return 2;
    }
    return check::run_all("CPP_OMS_009_RECEIVER_COMPLETE", 1, argv);
}