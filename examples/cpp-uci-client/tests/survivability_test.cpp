// Three-family receiver-core tests against pinned Sleet using independent OWP.
#include <chrono>
#include <fstream>
#include <stdexcept>
#include <string>
#include <variant>

#include "check.hpp"
#include "client.hpp"
#include "raw_owp_client.hpp"

namespace app = cpp_uci_client;
using namespace std::chrono_literals;

namespace {
using app::Family;
std::string port;
const std::string schema(app::release::schema_version);
std::array<std::string, 3> publications;

app::ClientSettings settings() {
    app::ClientSettings s;
    s.url = "ws://127.0.0.1:" + port;
    s.service_id = "cpp-uci-three";
    s.queue_capacity = 3;
    return s;
}

void start(app::Client& client) {
    if (client.connect()) {
        throw std::runtime_error("could not connect");
    }
    for (auto family : app::families) {
        if (client.subscribe(family)) {
            throw std::runtime_error("could not subscribe");
        }
        CHECK_EQ(client.subscription_id(family), "sub-" + std::to_string(app::index(family) + 1));
    }
}

raw::Client publisher() { return {"127.0.0.1", port, "cpp-uci-wire", schema, 5s}; }

void publish(raw::Client& wire, Family family) {
    CHECK_EQ(wire.command(publications[app::index(family)]), std::string("+OK"));
}

app::Next next(app::Client& client) {
    const auto deadline = app::Client::Clock::now() + 5s;
    do {
        auto result = client.next(deadline);
        if (result.kind != app::Next::Kind::idle) {
            return result;
        }
    } while (app::Client::Clock::now() < deadline);
    return {};
}

void check_clean(const app::ShutdownReport& report) {
    CHECK(report.clean());
    CHECK_EQ(report.unsubscribe_lines.size(), app::families.size());
    for (std::size_t i = 0; i < report.unsubscribe_lines.size(); ++i) {
        CHECK(report.unsubscribe_lines[i].rfind("UNSUBSCRIBED sid=sub-" + std::to_string(3 - i) + " ", 0) == 0);
    }
    for (const auto& stats : report.per_family) {
        CHECK_EQ(stats.callbacks_after_unsubscribe, std::uint64_t{0});
    }
}

TEST_CASE(three_families_share_queue_and_pending_discard) {
    auto wire = publisher();
    app::Client client(settings());
    start(client);
    for (auto family : app::families) {
        publish(wire, family);
        const auto wanted = app::index(family) + 1;
        CHECK(check::eventually([&] { return client.receiver().queue().size() == wanted; }, 5s));
    }
    for (auto family : app::families) {
        publish(wire, family);
    }
    CHECK(check::eventually([&] { return client.receiver().queue().counters().rejected_full == 3; }, 5s));
    const auto failure = next(client);
    CHECK(failure.kind == app::Next::Kind::failure && failure.exit_status == app::exit_status::session);
    CHECK(failure.line.rfind("QUEUE-OVERFLOW application queue capacity=3", 0) == 0);
    const auto report = client.shutdown();
    check_clean(report);
    CHECK(report.session_failed);
    CHECK_EQ(report.queue.accepted, std::uint64_t{3});
    CHECK_EQ(report.queue.rejected_full, std::uint64_t{3});
    CHECK_EQ(report.queue.rejected_closed, std::uint64_t{0});
    CHECK_EQ(report.queue.high_water, std::size_t{3});
    CHECK_EQ(report.pending_discarded, std::size_t{3});
    CHECK_EQ(report.stats.delivered, std::uint64_t{6});
    for (const auto& stats : report.per_family) {
        CHECK_EQ(stats.callbacks, std::uint64_t{2});
    }
    const auto repeated = client.shutdown();
    CHECK_EQ(repeated.pending_discarded, report.pending_discarded);
    CHECK(repeated.unsubscribe_lines == report.unsubscribe_lines);
}

TEST_CASE(callback_exception_and_per_sid_quiescence) {
    auto wire = publisher();
    app::Client client(settings(), app::summarize, app::summarize_rdma,
                       [](const app::release::m::SurvivabilityRiskLevelMT&) -> app::SurvivabilitySummary {
                           throw std::runtime_error("PRIVATE-CALLBACK-TEXT");
                       });
    start(client);
    publish(wire, Family::survivability);
    const auto failure = next(client);
    CHECK(failure.kind == app::Next::Kind::failure && failure.exit_status == app::exit_status::session);
    CHECK_EQ(failure.line, std::string("CALLBACK-FAILURE SurvivabilityRiskLevel category=handler_exception payload-bytes=unavailable"));
    std::string line;
    CHECK(client.unsubscribe(Family::survivability, line));
    publish(wire, Family::survivability);
    publish(wire, Family::rdma);
    const auto rdma = next(client);
    CHECK(rdma.kind == app::Next::Kind::message && std::holds_alternative<app::RdmaSummary>(rdma.item.summary));
    CHECK(client.unsubscribe(Family::rdma, line));
    publish(wire, Family::rdma);
    publish(wire, Family::log);
    const auto log = next(client);
    CHECK(log.kind == app::Next::Kind::message && std::holds_alternative<app::LogSummary>(log.item.summary));
    const auto report = client.shutdown();
    check_clean(report);
    publish(wire, Family::log);
    CHECK(report.session_failed);
    CHECK_EQ(report.stats.handler_exceptions, std::uint64_t{1});
    CHECK_EQ(report.stats.delivered, std::uint64_t{2});
    for (auto family : app::families) {
        const auto& counts = report.per_family[app::index(family)];
        CHECK_EQ(counts.callbacks, std::uint64_t{1});
        CHECK_EQ(counts.handler_exceptions, family == Family::survivability ? std::uint64_t{1} : std::uint64_t{0});
        CHECK_EQ(client.receiver().callbacks(family), std::uint64_t{1});
    }
}
}  // namespace

int main(int argc, char** argv) {
    if (argc != 3) {
        return 2;
    }
    port = argv[1];
    std::ifstream input(argv[2]);
    for (auto& frame : publications) {
        if (!std::getline(input, frame)) {
            return 2;
        }
    }
    return check::run_all("CPP_OMS_010_RECEIVER_COMPLETE", 1, argv);
}