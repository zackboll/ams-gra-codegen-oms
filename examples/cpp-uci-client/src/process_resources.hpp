// CPP-OMS-008: Linux /proc observation of this process's runtime resources,
// printed after close() as lifecycle evidence (runtime threads joined, no
// socket left open). Returns -1 when /proc cannot be read; never throws.
#pragma once

#include <dirent.h>
#include <unistd.h>

#include <cerrno>
#include <fstream>
#include <sstream>
#include <string>

namespace cpp_uci_client {

/// Live threads of this process whose name starts with `prefix`. A joined
/// thread can briefly remain listed while Linux tears it down; tasks marked
/// PF_EXITING can run no further userspace code and are not counted.
inline int count_threads_named(const std::string& prefix) {
    DIR* dir = opendir("/proc/self/task");
    if (dir == nullptr) {
        return -1;
    }
    int count = 0;
    for (;;) {
        errno = 0;
        const dirent* entry = readdir(dir);
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
        const std::string base = "/proc/self/task/" + id;
        std::string name;
        {
            std::ifstream comm(base + "/comm");
            if (!std::getline(comm, name)) {
                continue;  // task vanished
            }
        }
        if (name.rfind(prefix, 0) != 0) {
            continue;
        }
        std::string stat;
        {
            std::ifstream file(base + "/stat");
            if (!std::getline(file, stat)) {
                continue;
            }
        }
        const auto end = stat.rfind(')');
        unsigned long flags = 0;
        bool parsed = false;
        if (end != std::string::npos) {
            std::istringstream fields(stat.substr(end + 1));
            char state = '\0';
            long skip = 0;
            parsed = static_cast<bool>(fields >> state >> skip >> skip >> skip >> skip >> skip >> flags);
        }
        constexpr unsigned long exiting = 0x00000004UL;  // PF_EXITING
        if (!parsed || (flags & exiting) == 0) {
            ++count;
        }
    }
    closedir(dir);
    return count;
}

/// Open socket file descriptors of this process.
inline int count_open_sockets() {
    DIR* dir = opendir("/proc/self/fd");
    if (dir == nullptr) {
        return -1;
    }
    const int own = dirfd(dir);
    int count = 0;
    while (const dirent* entry = readdir(dir)) {
        const std::string fd = entry->d_name;
        if (fd == "." || fd == ".." || fd == std::to_string(own)) {
            continue;
        }
        char target[128] = {};
        const ssize_t n = readlink(("/proc/self/fd/" + fd).c_str(), target, sizeof(target) - 1);
        if (n > 0 && std::string(target, static_cast<std::size_t>(n)).rfind("socket:[", 0) == 0) {
            ++count;
        }
    }
    closedir(dir);
    return count;
}

}  // namespace cpp_uci_client
