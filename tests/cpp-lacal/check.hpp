// CPP-OMS-003 test harness: counted checks, named tests, exact summary line.
#pragma once

#include <atomic>
#include <cerrno>
#include <chrono>
#include <cstdlib>
#include <dirent.h>
#include <fstream>
#include <functional>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <thread>
#include <vector>
#include <unistd.h>

namespace check {

// Atomic: CHECK is also used on helper threads inside tests.
inline std::atomic<int> failures{0};
inline std::atomic<int> checks{0};
inline int tests = 0;

#define CHECK(condition)                                                                  \
    do {                                                                                  \
        ++::check::checks;                                                                \
        if (!(condition)) {                                                               \
            ++::check::failures;                                                          \
            std::cerr << __FILE__ << ':' << __LINE__ << ": CHECK failed: " << #condition << '\n'; \
        }                                                                                 \
    } while (false)

#define CHECK_EQ(actual, expected)                                                        \
    do {                                                                                  \
        ++::check::checks;                                                                \
        const auto check_actual_ = (actual);                                             \
        const auto check_expected_ = (expected);                                         \
        if (!(check_actual_ == check_expected_)) {                                        \
            ++::check::failures;                                                          \
            std::cerr << __FILE__ << ':' << __LINE__ << ": CHECK_EQ failed: " << #actual  \
                      << "\n  actual:   " << check_actual_ << "\n  expected: " << check_expected_ << '\n'; \
        }                                                                                 \
    } while (false)

struct Test {
    const char* name;
    std::function<void()> body;
};

inline std::vector<Test>& registry() {
    static std::vector<Test> all;
    return all;
}

struct Register {
    Register(const char* name, std::function<void()> body) { registry().push_back(Test{name, std::move(body)}); }
};

#define TEST_CASE(name)                                         \
    static void name();                                         \
    static const ::check::Register name##_registration(#name, name); \
    static void name()

/// Polls `predicate` until true or `timeout` elapses.
template <typename Predicate>
bool eventually(Predicate predicate, std::chrono::milliseconds timeout = std::chrono::seconds(5)) {
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    while (!predicate()) {
        if (std::chrono::steady_clock::now() > deadline) {
            return false;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(2));
    }
    return true;
}

/// Linux /proc stat field 9, after the parenthesized comm (which may contain spaces).
inline std::optional<unsigned long> task_flags(const std::string& stat) {
    const auto end = stat.rfind(')');
    if (end == std::string::npos) {
        return std::nullopt;
    }
    std::istringstream fields(stat.substr(end + 1));
    char state = '\0';
    long parent = 0, group = 0, session = 0, tty = 0, foreground = 0;
    unsigned long flags = 0;
    if (!(fields >> state >> parent >> group >> session >> tty >> foreground >> flags)) {
        return std::nullopt;
    }
    return flags;
}

namespace detail {

inline bool read_task_line(const std::string& path, std::string& line) {
    std::ifstream file(path);
    return static_cast<bool>(std::getline(file, line));
}

// Test-harness I/O seam for deterministic observation failures; no global hooks.
struct ProcTasks {
    DIR* open() const { return opendir("/proc/self/task"); }
    dirent* next(DIR* dir) const { return readdir(dir); }
    int close(DIR* dir) const { return closedir(dir); }
    bool read(const std::string& path, std::string& line) const { return read_task_line(path, line); }
    bool vanished(const std::string& path) const {
        return access(path.c_str(), F_OK) != 0 && errno == ENOENT;
    }
};

template <typename Tasks>
int threads_named(const std::string& prefix, Tasks& tasks) {
    int count = 0;
    auto* dir = tasks.open();
    if (dir == nullptr) {
        return -1;
    }
    for (;;) {
        errno = 0;
        const auto* entry = tasks.next(dir);
        if (entry == nullptr) {
            if (errno != 0) {
                count = -1;
            }
            break;
        }
        const std::string id = entry->d_name;
        if (id == "." || id == "..") {
            continue;
        }
        const std::string path = "/proc/self/task/" + id;
        std::string name;
        if (!tasks.read(path + "/comm", name) || name.empty()) {
            if (tasks.vanished(path)) {
                continue;
            }
            count = -1;
            break;
        }
        if (name.rfind(prefix, 0) == 0) {
            std::string record;
            if (!tasks.read(path + "/stat", record)) {
                // A task can disappear between reading comm and stat. Any
                // other read failure is a failed observation, not zero workers.
                if (tasks.vanished(path)) {
                    continue;
                }
                count = -1;
                break;
            }
            const auto flags = task_flags(record);
            if (!flags) {
                count = -1;
                break;
            }
            constexpr unsigned long exiting = 0x00000004UL;  // Linux PF_EXITING
            if ((*flags & exiting) != 0) {
                continue;
            }
            ++count;
        }
    }
    return tasks.close(dir) == 0 ? count : -1;
}

}  // namespace detail

/// Number of userspace-live threads whose name starts with `prefix`.
/// pthread_join can return before Linux unlinks the exiting task from /proc.
/// PF_EXITING tasks cannot execute further userspace code; they are not workers.
inline int threads_named(const std::string& prefix) {
    detail::ProcTasks tasks;
    return detail::threads_named(prefix, tasks);
}

/// Runs every registered test (or those named on the command line) and prints
/// `<marker> tests=N checks=M` on success. Exit status 1 on any failure.
inline int run_all(const char* marker, int argc, char** argv) {
    for (const Test& test : registry()) {
        bool selected = argc <= 1;
        for (int i = 1; i < argc; ++i) {
            selected = selected || std::string(argv[i]) == test.name;
        }
        if (!selected) {
            continue;
        }
        const int before = failures.load();
        const auto start = std::chrono::steady_clock::now();
        test.body();
        ++tests;
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start);
        std::cerr << (failures.load() == before ? "ok   " : "FAIL ") << test.name << " (" << elapsed.count() << " ms)\n";
    }
    if (failures != 0 || tests == 0) {
        std::cerr << failures.load() << " of " << checks.load() << " checks failed\n";
        return 1;
    }
    std::cout << marker << " tests=" << tests << " checks=" << checks.load() << '\n';
    return 0;
}

}  // namespace check
