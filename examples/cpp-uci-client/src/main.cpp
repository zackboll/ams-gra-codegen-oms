// CPP-OMS-008: standalone C++17 UCI Log receiver (application entry point).
//
// Connects to Sleet through the existing C++ LA-CAL runtime
// (cpp/oms-lacal-runtime: Boost.Beast WebSocket, subprotocol `owp`, verbose
// INIT/INFO), subscribes through the production-GENERATED receive endpoint
// service_api::function_log::exchange_receive (global message `Log`, topic
// `uci.log`) and receives generated programs::oam::LogMT values decoded by the
// generated service_codec::decode_LogMT. Built for exactly one UCI release
// (src/release.hpp). See examples/cpp-uci-client/README.md.
//
// main() owns every runtime operation. SIGINT/SIGTERM handlers only store
// the signal number in a volatile sig_atomic_t; the loop observes it within
// ~25 ms and performs the normal UNSUB + close on this thread.
#include <signal.h>

#include <chrono>
#include <csignal>
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

volatile std::sig_atomic_t g_stop_signal = 0;

extern "C" void cpp_uci_client_on_stop_signal(int signo) { g_stop_signal = signo; }

bool install_stop_handlers() {
    struct sigaction action {};
    action.sa_handler = cpp_uci_client_on_stop_signal;
    sigemptyset(&action.sa_mask);
    action.sa_flags = 0;  // the main loop polls with bounded waits; no restart semantics needed
    return sigaction(SIGINT, &action, nullptr) == 0 && sigaction(SIGTERM, &action, nullptr) == 0;
}

int stop_signal() { return static_cast<int>(g_stop_signal); }

void line(const std::string& text) { std::cout << text << std::endl; }

void report_shutdown(const cpp_uci_client::ShutdownReport& report) {
    if (report.subscribed) {
        line(report.unsubscribe_line);
    }
    for (const std::string& notice : report.notices) {
        line(notice);
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
    using Clock = std::chrono::steady_clock;

    if (!install_stop_handlers()) {
        std::cerr << "INTERNAL cannot install SIGINT/SIGTERM handlers\n";
        return status::internal;
    }
    line("CPP-UCI-CLIENT built-for=UCI-" + std::string(rel::uci_release) + " schema=" +
         std::string(rel::schema_version) + " message=" + std::string(rel::global_message) + " topic=" +
         std::string(rel::application_topic) + " max-messages=" + std::to_string(options.max_messages) +
         " duration-seconds=" + std::to_string(options.duration_seconds));

    cpp_uci_client::ClientSettings settings;
    settings.url = options.url;
    settings.service_id = options.service_id;
    cpp_uci_client::LogClient client(settings);

    // The first failure is the reported outcome; shutdown is always attempted.
    const auto finish = [&client](int primary) {
        const cpp_uci_client::ShutdownReport report = client.shutdown();
        report_shutdown(report);
        if (primary != status::ok) {
            return primary;
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
    // READY only after the broker's +OK for SUB.
    line("READY sid=" + client.subscription_id() + " message=" + std::string(rel::global_message) + " topic=" +
         std::string(rel::application_topic) + " (SUB acknowledged by the broker)");

    const Clock::time_point deadline = Clock::now() + std::chrono::seconds(options.duration_seconds);
    std::uint64_t displayed = 0;
    for (;;) {
        if (stop_signal() != 0) {
            line("STOP signal=" + std::to_string(stop_signal()) + " after " + std::to_string(displayed) +
                 " Log message(s)");
            return finish(status::ok);
        }
        if (displayed >= options.max_messages) {
            line("STOP max-messages=" + std::to_string(options.max_messages) + " reached");
            return finish(status::ok);
        }
        if (Clock::now() >= deadline) {
            line("STOP duration-seconds=" + std::to_string(options.duration_seconds) + " elapsed after " +
                 std::to_string(displayed) + " Log message(s)");
            return finish(status::ok);
        }
        cpp_uci_client::Next next = client.next(deadline);
        switch (next.kind) {
            case cpp_uci_client::Next::Kind::log:
                ++displayed;
                line(cpp_uci_client::format_summary(next.item.sequence, next.item.summary, rel::schema_version));
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
        const cpp_uci_client::ParseResult parsed = cpp_uci_client::parse_arguments(args);
        switch (parsed.kind) {
            case cpp_uci_client::ParseResult::Kind::help:
                std::cout << cpp_uci_client::help_text("cpp-uci-client", rel::uci_release, rel::schema_version,
                                                       rel::global_message, rel::application_topic);
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
