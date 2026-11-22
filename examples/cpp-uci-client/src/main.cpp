// Standalone C++17 UCI receiver (application entry point). One Runtime and
// WebSocket carry Log and, optionally, RDMA_InitializeSetup via generated APIs.
//
// main() owns every runtime operation. SIGINT/SIGTERM handlers only store
// the signal number in a lock-free atomic; the loop observes it within
// ~25 ms and performs the normal UNSUB + close on this thread.
#include <signal.h>

#include <atomic>
#include <chrono>
#include <cstdint>
#include <exception>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

#include "cli.hpp"
#include "client.hpp"
#include "release.hpp"

namespace {

static_assert(std::atomic<int>::is_always_lock_free, "signal handler must not take locks");
std::atomic<int> g_stop_signal{0};

extern "C" void cpp_uci_client_on_stop_signal(int signo) { g_stop_signal.store(signo, std::memory_order_relaxed); }

bool install_stop_handlers() {
    struct sigaction action {};
    action.sa_handler = cpp_uci_client_on_stop_signal;
    sigemptyset(&action.sa_mask);
    action.sa_flags = 0;  // the main loop polls with bounded waits; no restart semantics needed
    return sigaction(SIGINT, &action, nullptr) == 0 && sigaction(SIGTERM, &action, nullptr) == 0;
}

int stop_signal() { return g_stop_signal.load(std::memory_order_relaxed); }

void line(const std::string& text) { std::cout << text << std::endl; }

void report_shutdown(const cpp_uci_client::ShutdownReport& report, bool with_rdma) {
    for (const std::string& unsubscribed : report.unsubscribe_lines) {
        line(unsubscribed);
    }
    for (const std::string& notice : report.notices) {
        line(notice);
    }
    if (with_rdma) {
        for (auto family : cpp_uci_client::families) {
            const auto& stats = report.per_family[cpp_uci_client::index(family)];
            line("FAMILY message=" + std::string(cpp_uci_client::message_name(family)) +
                 " callbacks=" + std::to_string(stats.callbacks) +
                 " callbacks-after-unsubscribe=" + std::to_string(stats.callbacks_after_unsubscribe) +
                 " decode-errors=" + std::to_string(stats.decode_errors) +
                 " handler-exceptions=" + std::to_string(stats.handler_exceptions));
        }
    }
    const auto& s = report.stats;
    line("CLOSED " + report.close_line + " callbacks=" + std::to_string(report.callbacks) +
         " callbacks-after-unsubscribe=" + std::to_string(report.callbacks_after_unsubscribe) +
         " pending-discarded=" + std::to_string(report.pending_discarded) +
         " queue-accepted=" + std::to_string(report.queue.accepted) +
         " queue-rejected-full=" + std::to_string(report.queue.rejected_full) +
         " queue-rejected-closed=" + std::to_string(report.queue.rejected_closed) +
         " queue-high-water=" + std::to_string(report.queue.high_water) +
         " delivered=" + std::to_string(s.delivered) + " decode-errors=" + std::to_string(s.decode_errors) +
         " handler-exceptions=" + std::to_string(s.handler_exceptions) +
         " unknown-subscription=" + std::to_string(s.unknown_subscription) +
         " discarded-on-close=" + std::to_string(s.discarded_on_close) +
         " runtime-threads=" + std::to_string(report.runtime_threads) + " sockets=" + std::to_string(report.sockets) +
         " sockets-at-start=" + std::to_string(report.sockets_at_start) +
         " resources-released=" + (report.resources_released() ? "yes" : "no"));
}

int run(const cpp_uci_client::Options& options) {
    namespace rel = cpp_uci_client::release;
    namespace status = cpp_uci_client::exit_status;
    using cpp_uci_client::Family;
    using Clock = std::chrono::steady_clock;

    if (!install_stop_handlers()) {
        std::cerr << "INTERNAL cannot install SIGINT/SIGTERM handlers\n";
        return status::internal;
    }
    line("CPP-UCI-CLIENT built-for=UCI-" + std::string(rel::uci_release) + " schema=" +
         std::string(rel::schema_version) + " message=" + std::string(rel::global_message) + " topic=" +
         std::string(rel::application_topic) + " max-messages=" + std::to_string(options.max_messages) +
         " duration-seconds=" + std::to_string(options.duration_seconds) +
         (options.with_rdma ? " also-message=" + std::string(rel::rdma_message) + " also-topic=" +
                                  std::string(rel::rdma_topic) : ""));

    cpp_uci_client::ClientSettings settings;
    settings.url = options.url;
    settings.service_id = options.service_id;
    cpp_uci_client::Client client(settings);

    // The first failure is the reported outcome; shutdown is always attempted.
    const auto finish = [&client, &options](int primary) {
        const cpp_uci_client::ShutdownReport report = client.shutdown();
        if (options.with_rdma && primary == status::ok && report.queue.rejected_full != 0) {
            line("QUEUE-OVERFLOW during shutdown; rejected=" + std::to_string(report.queue.rejected_full));
        }
        report_shutdown(report, options.with_rdma);
        if (primary != status::ok) {
            return primary;
        }
        if (options.with_rdma && report.session_failed) {
            return status::session;
        }
        if (!report.clean()) {
            return status::shutdown;
        }
        return stop_signal() != 0 ? status::signal_base + stop_signal() : status::ok;
    };
    const auto fail = [&finish](const std::string& text, int code) {
        std::cerr << text << '\n';
        line(text);
        return finish(code);
    };

    if (auto failure = client.connect()) {
        return fail(failure->line, failure->exit_status);
    }
    line("CONNECTED owp=1.0 schema=" + std::string(rel::schema_version) + " server=" + client.server_id());
    if (stop_signal() != 0) {
        line("STOP signal=" + std::to_string(stop_signal()) + " before subscribe");
        return finish(status::ok);
    }
    if (auto failure = client.subscribe()) {
        return fail(failure->line, failure->exit_status);
    }
    if (options.with_rdma) {
        if (stop_signal() != 0) {
            line("STOP signal=" + std::to_string(stop_signal()) + " before RDMA subscribe");
            return finish(status::ok);
        }
        if (auto failure = client.subscribe(Family::rdma)) {
            return fail(failure->line, failure->exit_status);
        }
        if (!client.connected()) {
            return fail("CONNECTION-LOST after final SUB acknowledgment", status::session);
        }
        if (stop_signal() != 0) {
            line("STOP signal=" + std::to_string(stop_signal()) + " during setup");
            return finish(status::ok);
        }
        line("READY sid=" + client.subscription_id() + " message=Log topic=uci.log sid=" +
             client.subscription_id(Family::rdma) + " message=" + std::string(rel::rdma_message) +
             " topic=" + std::string(rel::rdma_topic) + " (both SUBs acknowledged by the broker)");
    } else {
        line("READY sid=" + client.subscription_id() + " message=" + std::string(rel::global_message) + " topic=" +
             std::string(rel::application_topic) + " (SUB acknowledged by the broker)");
    }

    const Clock::time_point deadline = Clock::now() + std::chrono::seconds(options.duration_seconds);
    std::uint64_t displayed = 0;
    const std::string message_count_unit = options.with_rdma ? " typed message(s)" : " Log message(s)";
    for (;;) {
        if (stop_signal() != 0) {
            line("STOP signal=" + std::to_string(stop_signal()) + " after " + std::to_string(displayed) +
                 message_count_unit);
            return finish(status::ok);
        }
        if (displayed >= options.max_messages) {
            line("STOP max-messages=" + std::to_string(options.max_messages) + " reached");
            return finish(status::ok);
        }
        if (Clock::now() >= deadline) {
            line("STOP duration-seconds=" + std::to_string(options.duration_seconds) + " elapsed after " +
                 std::to_string(displayed) + message_count_unit);
            return finish(status::ok);
        }
        cpp_uci_client::Next next = client.next(deadline);
        switch (next.kind) {
            case cpp_uci_client::Next::Kind::message:
                ++displayed;
                line(cpp_uci_client::format_delivered(next.item));
                break;
            case cpp_uci_client::Next::Kind::notice:
                line(next.line);
                break;
            case cpp_uci_client::Next::Kind::failure:
                return fail(next.line, next.exit_status);
            case cpp_uci_client::Next::Kind::idle:
                break;
        }
    }
}

}  // namespace

int main(int argc, char** argv) {
    namespace rel = cpp_uci_client::release;
    namespace status = cpp_uci_client::exit_status;
    try {
        std::vector<std::string_view> args;
        for (int i = 1; i < argc; ++i) {
            args.emplace_back(argv[i]);
        }
        const cpp_uci_client::ParseResult parsed = cpp_uci_client::parse_arguments(args, rel::supports_rdma);
        switch (parsed.kind) {
            case cpp_uci_client::ParseResult::Kind::help:
                std::cout << cpp_uci_client::help_text("cpp-uci-client", rel::uci_release, rel::schema_version,
                                                       rel::global_message, rel::application_topic, rel::supports_rdma);
                return status::ok;
            case cpp_uci_client::ParseResult::Kind::error:
                std::cerr << "ERROR " << parsed.error << '\n';
                return status::usage;
            case cpp_uci_client::ParseResult::Kind::run:
                break;
        }
        return run(parsed.options);
    } catch (const std::exception& error) {
        std::cerr << "INTERNAL unexpected exception: " << cpp_uci_client::safe_text(error.what(), 200) << '\n';
        return status::internal;
    } catch (...) {
        std::cerr << "INTERNAL unexpected non-standard exception\n";
        return status::internal;
    }
}
