#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace example::order {

template <typename T, std::size_t Min, std::size_t Max>
class BoundedVector {
public:
    static std::optional<BoundedVector> create(std::vector<T> values) {
        if (values.size() < Min || values.size() > Max) {
            return std::nullopt;
        }
        return BoundedVector(std::move(values));
    }

    const std::vector<T>& values() const noexcept { return values_; }

private:
    explicit BoundedVector(std::vector<T> values) : values_(std::move(values)) {}
    std::vector<T> values_;
};

class IncludedId {
public:
    static constexpr std::int64_t min_value = 1;
    static constexpr std::int64_t max_value = 65535;

    static std::optional<IncludedId> create(std::int64_t value) noexcept {
        if (value < min_value || value > max_value) {
            return std::nullopt;
        }
        return IncludedId(value);
    }

    std::int64_t value() const noexcept { return value_; }

private:
    explicit IncludedId(std::int64_t value) noexcept : value_(value) {}
    std::int64_t value_;
};

enum class IncludedQuality {
    Unknown,
    Good,
    Bad,
};

struct RecordFirstInSource {
    IncludedId id;
    IncludedQuality quality;
};

}  // namespace example::order
