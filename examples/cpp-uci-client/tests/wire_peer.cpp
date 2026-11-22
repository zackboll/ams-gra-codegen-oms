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
#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iostream>
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
        std::cerr << "usage: wire_peer observe|publish|check-golden ...\n";
        return 2;
    } catch (const std::exception& error) {
        std::cerr << "wire_peer: " << error.what() << '\n';
        return 1;
    }
}
