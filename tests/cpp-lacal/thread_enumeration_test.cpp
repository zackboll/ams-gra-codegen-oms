// No broker or runtime: distinguish joined Linux task records from live workers.
#include "check.hpp"
#include <pthread.h>
#include <condition_variable>
#include <mutex>

namespace {
struct Tasks {
    struct Entry { const char* d_name; } entry{"123"};
    enum class Name { valid, open_failure, read_failure, empty } name = Name::valid;
    int handle = 0;
    int entries = 1;
    int position = 0;
    int closes = 0;
    bool open_failure = false;
    bool enumeration_failure = false;
    bool close_failure = false;
    bool disappeared = false;
    bool stat_failure = false;
    bool errno_was_clear = true;
    std::string comm = "ams-probe";
    std::string stat = "123 (ams-probe) S 1 2 3 4 5 0 0";

    int* open() { return open_failure ? nullptr : &handle; }
    const Entry* next(int*) {
        errno_was_clear = errno_was_clear && errno == 0;
        if (position++ < entries) {
            // Successful calls need not preserve errno, just like unrelated I/O.
            errno = EIO;
            return &entry;
        }
        if (enumeration_failure) errno = EIO;
        return nullptr;
    }
    int close(int*) { ++closes; return close_failure ? -1 : 0; }
    bool read(const std::string& path, std::string& line) const {
        if (path == "/proc/self/task/123/comm") {
            if (name == Name::open_failure) {
                return check::detail::read_task_line("/dev/null/not-a-file", line);
            }
            if (name == Name::read_failure) {
                // /dev/null opens successfully, but extracting a line fails.
                return check::detail::read_task_line("/dev/null", line);
            }
            line = name == Name::empty ? "" : comm;
        } else {
            if (stat_failure) return false;
            line = stat;
        }
        errno = EACCES;  // must not be mistaken for a later readdir error
        return true;
    }
    bool vanished(const std::string&) const { return disappeared; }
};

void observe(Tasks& tasks, int expected) {
    errno = EACCES;  // even the first readdir must receive a cleared errno
    CHECK_EQ(check::detail::threads_named("ams-probe", tasks), expected);
    CHECK_EQ(tasks.closes, tasks.open_failure ? 0 : 1);
    CHECK(tasks.errno_was_clear);
}
}

TEST_CASE(task_names_fail_closed_unless_proven_vanished) {
    for (auto name : {Tasks::Name::open_failure, Tasks::Name::read_failure, Tasks::Name::empty}) {
        for (bool vanished : {false, true}) {
            Tasks tasks;
            tasks.name = name;
            tasks.disappeared = vanished;
            observe(tasks, vanished ? 0 : -1);
        }
    }
    Tasks nonmatching;
    nonmatching.comm = "unrelated";
    observe(nonmatching, 0);
    // An unsuccessful disappearance check must not conceal the failed read.
    CHECK(!check::detail::ProcTasks{}.vanished("/dev/null"));
    CHECK(!check::detail::ProcTasks{}.vanished("/dev/null/not-a-file"));  // ENOTDIR, not ENOENT
    CHECK(check::detail::ProcTasks{}.vanished("/proc/self/task/no-such-task"));
}

TEST_CASE(directory_observation_failures_and_normal_end) {
    for (int entries : {0, 1, 2}) {
        for (bool error : {false, true}) {
            Tasks tasks;
            tasks.entries = entries;
            tasks.enumeration_failure = error;
            observe(tasks, error ? -1 : entries);
        }
    }
    for (int entries : {0, 1}) {
        Tasks tasks;
        tasks.entries = entries;
        tasks.close_failure = true;
        observe(tasks, -1);
    }
    Tasks closed_after_error;
    closed_after_error.name = Tasks::Name::read_failure;
    closed_after_error.close_failure = true;
    observe(closed_after_error, -1);
    Tasks unopened;
    unopened.open_failure = true;
    observe(unopened, -1);
}

TEST_CASE(task_statistics_and_exiting_workers) {
    for (const std::string& stat : {std::string(), std::string("malformed"), std::string("123 (name) R 1")}) {
        Tasks tasks;
        tasks.stat = stat;
        observe(tasks, -1);
    }
    for (bool vanished : {false, true}) {
        Tasks tasks;
        tasks.stat_failure = true;
        tasks.disappeared = vanished;
        observe(tasks, vanished ? 0 : -1);
    }
    for (const std::string& state : {std::string("R"), std::string("S"), std::string("D")}) {
        for (bool exiting : {false, true}) {
            Tasks tasks;
            tasks.stat = "123 (ams-probe) " + state + " 1 2 3 4 5 " + (exiting ? "4194380" : "4194376") + " 0";
            observe(tasks, exiting ? 0 : 1);
        }
    }
}

TEST_CASE(stat_parser_fails_closed) {
    CHECK(!check::task_flags("").has_value());
    CHECK(!check::task_flags("123 (name) R 1").has_value());
    CHECK(check::task_flags("123 (name with ) spaces) R 1 2 3 4 5 4194380 0").value() == 4194380UL);
    CHECK(check::task_flags("123 (name) S 1 2 3 4 5 0 0").value() == 0UL);
}

TEST_CASE(live_worker_is_not_hidden) {
    std::mutex mutex;
    std::condition_variable cv;
    bool stop = false;
    std::thread worker([&] {
        std::unique_lock<std::mutex> lock(mutex);
        cv.wait(lock, [&] { return stop; });
    });
    CHECK(pthread_setname_np(worker.native_handle(), "ams-probe-live") == 0);
    CHECK(check::threads_named("ams-probe-live") == 1);
    {
        std::lock_guard<std::mutex> lock(mutex);
        stop = true;
    }
    cv.notify_one();
    worker.join();
    CHECK(check::threads_named("ams-probe-live") == 0);
}

TEST_CASE(joined_worker_is_not_a_live_task) {
    for (int i = 0; i < 2000; ++i) {
        std::atomic<bool> release{false};
        std::atomic<bool> complete{false};
        std::thread worker([&] {
            while (!release.load()) { std::this_thread::yield(); }
            complete.store(true);
        });
        CHECK(pthread_setname_np(worker.native_handle(), "ams-probe-join") == 0);
        release.store(true);
        worker.join();
        CHECK(complete.load());
        CHECK(check::threads_named("ams-probe-join") == 0);
    }
}

int main(int argc, char** argv) {
    return check::run_all("CPP_LIVE_THREAD_ENUMERATION", argc, argv);
}