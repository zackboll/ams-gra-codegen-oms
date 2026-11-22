// Test client for the independent Python TLS peer. No other TLS client and no
// test-only certificate or verification callback in the runtime.
#include "ams_gra/lacal/runtime.hpp"
#include "check.hpp"

#include <netdb.h>

#include <cstdlib>
#include <filesystem>
#include <iostream>

namespace lacal = ams_gra::lacal;
using namespace std::chrono_literals;

// GNU/Linux test-only seam: make DNS cancellation deterministic without
// touching the host resolver or adding production hooks. Asio cannot interrupt
// getaddrinfo once libc has entered it; close joins the resolver worker.
extern "C" int __real_getaddrinfo(const char*, const char*, const addrinfo*, addrinfo**);
extern "C" int __wrap_getaddrinfo(const char* host, const char* service, const addrinfo* hints, addrinfo** out) {
    if (host != nullptr && std::string_view(host) == "slow-resolution.invalid") {
        std::this_thread::sleep_for(350ms);
        return __real_getaddrinfo("localhost", service, hints, out);
    }
    if (host != nullptr && std::string_view(host) == "failed-resolution.invalid") {
        return EAI_NONAME;
    }
    return __real_getaddrinfo(host, service, hints, out);
}

namespace {

// The TLS lifecycle tests need a large frame, not a second UCI codec. Genuine
// generated types and codecs are exercised separately by real_log_sleet.cpp.
struct Payload { std::size_t bytes; };

int socket_count() {
    int count = 0;
    for (const auto& entry : std::filesystem::directory_iterator("/proc/self/fd")) {
        std::error_code ec;
        auto link = std::filesystem::read_symlink(entry.path(), ec);
        if (!ec && link.native().rfind("socket:[", 0) == 0) {
            ++count;
        }
    }
    return count;
}

void need(bool ok, const char* message) {
    if (!ok) {
        std::cerr << message << '\n';
        std::exit(1);
    }
}

}  // namespace

template<>
struct lacal::payload_codec<Payload> {
    static constexpr bool bound = true;
    struct CodecError { std::string message; std::string path; };
    template<class T> struct Encoded {
        T data;
        bool ok() const { return true; }
        const T& value() const { return data; }
        CodecError error() const { return {}; }
    };
    static Encoded<std::string> encode(const Payload& payload) {
        return {"\"" + std::string(payload.bytes, 'x') + "\""};
    }
    static Encoded<Payload> decode(std::string_view body) { return {{body.size()}}; }
};

int main(int argc, char** argv) {
    need(argc == 6, "usage: probe URL CA-or-- CONNECT_BUDGET_MS connect-result scenario");
    const int sockets = socket_count();
    lacal::Config config;
    config.url = argv[1];
    if (std::string_view(argv[2]) != "-") {
        config.tls_ca_file = argv[2];
    }
    config.service_id = "cpp-oms-011";
    config.schema_version = "002.6.0";
    config.connect_timeout = std::chrono::milliseconds(std::stoi(argv[3]));
    config.operation_timeout = 2s;
    const std::string_view scenario(argv[5]);
    config.close_timeout = scenario == "write-drain" ? 5s : 400ms;
    const auto start = std::chrono::steady_clock::now();
    auto runtime = lacal::Runtime::connect(std::move(config));
    const auto connect_ms = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start);
    const std::string_view result = runtime.ok() ? "ok" : lacal::to_string(runtime.error().kind);
    std::cout << "connect=" << result << " elapsed_ms=" << connect_ms.count() << '\n';
    if (!runtime.ok()) {
        std::cout << runtime.error().message << '\n';
    }
    need(result == argv[4], "unexpected connect result");
    if (runtime.ok()) {
        if (scenario == "sub-ack") {
            auto sub = runtime->subscribe<Payload>(lacal::owp::oam_namespace, "Log", "telemetry", std::nullopt,
                                                   [](const Payload&) {});
            need(!sub && sub.error().kind == lacal::ErrorKind::connection_closed, "SUB lost acknowledgment not reported");
        } else if (scenario == "active-read" || scenario == "peer-close-timeout" || scenario == "peer-close-clean") {
            const auto event = runtime->poll_event(2s);
            need(event && event->kind == lacal::EventKind::connection_closed, "active read loss not reported");
            const auto reason = scenario == "peer-close-timeout" ? "shutdown timed out"
                              : scenario == "peer-close-clean" ? "peer closed the WebSocket" : "read failed";
            need(event->error.message.find(reason) != std::string::npos, "wrong read loss reason");
        } else if (scenario == "active-pub") {
            auto status = runtime->publish<Payload>(lacal::owp::oam_namespace, "Log", "telemetry", {64});
            need(!status && status.error().kind == lacal::ErrorKind::connection_closed, "PUB lost acknowledgment not reported");
        } else if (scenario == "write-drop" || scenario == "write-close" || scenario == "write-drain") {
            constexpr std::size_t bytes = 32U * 1024U * 1024U;
            const bool drains = scenario == "write-drain";
            lacal::Status result_write;
            std::thread writer([&] {
                result_write = runtime->publish<Payload>(lacal::owp::oam_namespace, "Log", "telemetry", {bytes});
            });
            need(check::eventually([&] {
                auto state = lacal::detail::TestAccess::io_snapshot(*runtime, 100ms);
                return state && state->write_in_flight && state->in_flight_bytes >= bytes;
            }), "large frame never had an in-flight write");
            std::cout << "in-flight-write" << std::endl;
            if (scenario == "write-close") {
                need(runtime->close().error().kind == lacal::ErrorKind::timeout, "write drain timeout not reported");
            } else if (drains) {
                lacal::Status close;
                std::thread closer([&] { close = runtime->close(); });
                need(check::eventually([&] {
                    const auto state = lacal::detail::TestAccess::io_snapshot(*runtime, 100ms);
                    return state && state->write_in_flight && state->close_deferred && !state->close_handshake_started;
                }), "WebSocket Close was not deferred to the end of the write");
                std::cout << "close-deferred" << std::endl;
                closer.join();
                need(close.ok(), "deferred WebSocket/TLS close failed");
            }
            writer.join();
            need(!result_write && result_write.error().kind == lacal::ErrorKind::connection_closed,
                 "large PUB loss not reported");
            need(check::eventually([&] { return runtime->stats().writes_aborted == (drains ? 0U : 1U); }),
                 "incorrect write abort count");
        }
        if (scenario == "destructor") {
            auto destroyed_here = std::move(runtime).value();
        } else {
            const auto close_start = std::chrono::steady_clock::now();
            const auto close = runtime->close();
            const auto close_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now() - close_start);
            std::cout << "close=" << lacal::to_string(close.error().kind) << " elapsed_ms=" << close_ms.count()
                      << " " << close.error().message << '\n';
            if (scenario == "close-timeout") {
                need(close.error().kind == lacal::ErrorKind::timeout && close_ms >= 350ms && close_ms < 1s,
                     "close_timeout did not bound WebSocket/TLS close");
            } else if (scenario == "close-truncated") {
                need(close.error().kind == lacal::ErrorKind::connection_closed &&
                         close.error().message.find("stream truncated") != std::string::npos,
                     "missing close_notify was not reported as truncation");
            } else {
                need(close.ok(), "unexpected close error");
            }
            need(runtime->close().ok(), "repeat close was not idempotent");
        }
    }
    need(check::threads_named("ams-lacal") == 0, "runtime thread leaked");
    need(socket_count() == sockets, "client socket leaked");
    std::cout << "cleanup=ok\n";
}