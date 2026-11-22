// CPP-OMS-008 TEST-ONLY independent wire peer for real-Sleet evidence.
//
// Built on tests/cpp-lacal/raw_owp_client.hpp (CPP-OMS-003): its own Beast
// handshake, INIT and frame strings; it shares NO code with the C++ LA-CAL
// runtime and never uses the generated encoder or decoder. Documents are
// hand-written (documents.hpp). Roles:
//
//   wire_peer observe URL SERVICE SCHEMA SECONDS OUT_FILE
//       SUB wire Log uci.log, then appends every MSG text (one per line) to
//       OUT_FILE until SECONDS elapse; prints "OBSERVER READY" after +OK.
//   wire_peer publish URL SERVICE SCHEMA PLAN
//       PLAN is a comma list of steps: v<N> (variant N), golden, decoder-bad,
//       broker-bad, logmt (global element LogMT), sleep<MS>, wait-file<PATH>.
//       Prints "PUB <step> -> <status>" for each Sleet acknowledgment.
//   wire_peer render SCHEMA PLAN
//       Prints the exact envelope text each publish step of PLAN sends.
//   wire_peer check-golden SCHEMA FIXTURE
//       documents::golden(SCHEMA) must equal FIXTURE with whitespace removed.
//   wire_peer round-trip|round-trip-three URL SERVICE SCHEMA PLAN_TSV
//       Check verbose statuses and exact deliveries on both subscriptions.
#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <sstream>
#include <string>
#include <thread>
#include <vector>

#include "documents.hpp"
#include "raw_owp_client.hpp"

namespace {

constexpr const char* kTopic = "uci.log";

struct Where {
    std::string host;
    std::string port;
};

Where split(const std::string& url) {
    const std::size_t scheme = url.find("://");
    const std::string rest = url.substr(scheme == std::string::npos ? 0 : scheme + 3);
    const std::string authority = rest.substr(0, rest.find('/'));
    const std::size_t colon = authority.rfind(':');
    return Where{authority.substr(0, colon), authority.substr(colon + 1)};
}

std::string without_whitespace(const std::string& text) {
    std::string out;
    for (const char c : text) {
        if (c != ' ' && c != '\n' && c != '\r' && c != '\t') {
            out += c;
        }
    }
    return out;
}

int observe(const std::string& url, const std::string& service, const std::string& schema, int seconds,
            const std::string& path) {
    const Where where = split(url);
    raw::Client client(where.host, where.port, service, schema, std::chrono::seconds(10));
    const std::string status = client.command(std::string("SUB wire Log ") + kTopic);
    std::cout << "OBSERVER SUB -> " << status << std::endl;
    if (status != "+OK") {
        return 1;
    }
    std::cout << "OBSERVER READY" << std::endl;
    std::ofstream out(path, std::ios::app);
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(seconds);
    int count = 0;
    while (std::chrono::steady_clock::now() < deadline) {
        auto frame = client.receive(std::chrono::milliseconds(100));
        if (!frame) {
            continue;
        }
        if (frame->rfind("MSG wire ", 0) == 0) {
            out << frame->substr(9) << '\n';
            out.flush();
            ++count;
        } else {
            std::cout << "OBSERVER unexpected frame: " << frame->substr(0, 40) << std::endl;
        }
    }
    std::cout << "OBSERVER MSG count=" << count << std::endl;
    return 0;
}

std::vector<std::string> split_plan(const std::string& plan) {
    std::vector<std::string> steps;
    std::stringstream stream(plan);
    std::string step;
    while (std::getline(stream, step, ',')) {
        if (!step.empty()) {
            steps.push_back(step);
        }
    }
    return steps;
}

/// The exact `{"<element>":<body>}` text a publish step sends (empty for
/// non-publishing steps), so the script can compare observed MSGs byte for byte.
std::string envelope(const std::string& schema, const std::string& step) {
    if (step.rfind('v', 0) == 0) {
        return "{\"Log\":" + documents::variant(schema, std::atoi(step.c_str() + 1)) + "}";
    }
    if (step == "golden") {
        return "{\"Log\":" + documents::golden(schema) + "}";
    }
    if (step == "decoder-bad") {
        return "{\"Log\":" + documents::decoder_rejected(schema) + "}";
    }
    if (step == "broker-bad") {
        return "{\"Log\":" + documents::broker_rejected(schema) + "}";
    }
    if (step == "logmt") {
        return "{\"LogMT\":" + documents::golden(schema) + "}";  // payload type name, NOT the global message
    }
    return {};
}

int render(const std::string& schema, const std::string& plan) {
    for (const std::string& step : split_plan(plan)) {
        const std::string text = envelope(schema, step);
        if (text.empty()) {
            std::cerr << "render: not a publish step: " << step << '\n';
            return 2;
        }
        std::cout << text << '\n';
    }
    return 0;
}

int publish(const std::string& url, const std::string& service, const std::string& schema, const std::string& plan) {
    const Where where = split(url);
    raw::Client client(where.host, where.port, service, schema, std::chrono::seconds(10));
    for (const std::string& step : split_plan(plan)) {
        if (step.rfind("sleep", 0) == 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(std::atoi(step.c_str() + 5)));
            continue;
        }
        if (step.rfind("wait-file", 0) == 0) {
            const std::string file = step.substr(9);
            const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
            while (!std::ifstream(file).good() && std::chrono::steady_clock::now() < deadline) {
                std::this_thread::sleep_for(std::chrono::milliseconds(20));
            }
            continue;
        }
        const std::string text = envelope(schema, step);
        if (text.empty()) {
            std::cerr << "unknown plan step " << step << '\n';
            return 2;
        }
        const std::string status = client.command(std::string("PUB ") + kTopic + " " + text);
        std::cout << "PUB " << step << " -> " << status.substr(0, 200) << std::endl;
    }
    return 0;
}

// TSV: label, topic, expected status prefix, compact envelope. Separate raw
// connections observe and publish; compare admitted MSGs without printing JSON.
void round_trip(const std::string& url, const std::string& service, const std::string& schema,
                const std::string& path, bool survivability) {
    const Where where = split(url);
    raw::Client observer(where.host, where.port, service, schema, std::chrono::seconds(5));
    raw::Client publisher(where.host, where.port, service, schema, std::chrono::seconds(5));
    if (observer.command("SUB wrong-name RDMAInitializeSetupMT uci.rdma").rfind("-ERR Illegal-Argument", 0) != 0) {
        throw std::runtime_error("C++ type name was not rejected as an OWP SUB message name");
    }
    std::cout << "SUB wrong-name -> -ERR Illegal-Argument" << std::endl;
    if (observer.command("SUB wire-log Log uci.log") != "+OK" ||
        observer.command("SUB wire-rdma RDMA_InitializeSetup uci.rdma") != "+OK") {
        throw std::runtime_error("observer SUB not acknowledged");
    }
    if (survivability) {
        if (observer.command("SUB wrong-risk SurvivabilityRiskLevelMT uci.survivability").rfind("-ERR Illegal-Argument", 0) != 0 ||
            observer.command("SUB wire-risk SurvivabilityRiskLevel uci.survivability") != "+OK") {
            throw std::runtime_error("unexpected Survivability SUB result");
        }
        std::cout << "SUB wrong-risk -> -ERR Illegal-Argument\nSUB wire-risk -> +OK" << std::endl;
    }
    std::ifstream plan(path);
    if (!plan) {
        throw std::runtime_error("cannot open publication plan");
    }
    std::string row;
    while (std::getline(plan, row)) {
        std::istringstream fields(row);
        std::string label, topic, expected, envelope_text;
        if (!std::getline(fields, label, '\t') || !std::getline(fields, topic, '\t') ||
            !std::getline(fields, expected, '\t') || !std::getline(fields, envelope_text)) {
            throw std::runtime_error("invalid publication plan row");
        }
        const std::string result = publisher.command("PUB " + topic + " " + envelope_text);
        if (result.rfind(expected, 0) != 0) {
            throw std::runtime_error("unexpected PUB status for " + label);
        }
        std::cout << "PUB " << label << " -> " << expected << std::endl;
        if (expected == "+OK") {
            const std::string sid = topic == kTopic ? "wire-log" : topic == "uci.rdma" ? "wire-rdma" : "wire-risk";
            if (observer.receive().value_or("") != "MSG " + sid + " " + envelope_text) {
                throw std::runtime_error("observer MSG differs for " + label);
            }
            std::cout << "OBSERVED " << label << " sid=" << sid << " byte-exact" << std::endl;
        } else if (observer.receive(std::chrono::milliseconds(100))) {
            throw std::runtime_error("unexpected MSG after broker rejection for " + label);
        } else if (survivability) {
            std::cout << "NO MSG " << label << std::endl;
        }
    }
}

}  // namespace

int main(int argc, char** argv) {
    try {
        const std::vector<std::string> args(argv + 1, argv + argc);
        if (args.size() == 6 && args[0] == "observe") {
            return observe(args[1], args[2], args[3], std::atoi(args[4].c_str()), args[5]);
        }
        if (args.size() == 5 && args[0] == "publish") {
            return publish(args[1], args[2], args[3], args[4]);
        }
        if (args.size() == 5 && (args[0] == "round-trip" || args[0] == "round-trip-three")) {
            round_trip(args[1], args[2], args[3], args[4], args[0] == "round-trip-three");
            return 0;
        }
        if (args.size() == 3 && args[0] == "render") {
            return render(args[1], args[2]);
        }
        if (args.size() == 3 && args[0] == "check-golden") {
            std::ifstream file(args[2]);
            std::stringstream text;
            text << file.rdbuf();
            if (!file.is_open() || without_whitespace(text.str()) != without_whitespace(documents::golden(args[1]))) {
                std::cerr << "golden mismatch against " << args[2] << '\n';
                return 1;
            }
            std::cout << "GOLDEN " << args[1] << " matches " << args[2] << std::endl;
            return 0;
        }
        std::cerr << "usage: wire_peer observe|publish|check-golden|round-trip|round-trip-three ...\n";
        return 2;
    } catch (const std::exception& error) {
        std::cerr << "wire_peer: " << error.what() << '\n';
        return 1;
    }
}
