// Count the words of the text in a map, repeated; each count is added as it grows.

#include <cstdint>
#include <cstdio>
#include <fstream>
#include <iterator>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    std::ifstream file(argv[1]);
    std::string text((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    std::vector<std::string_view> words;
    for (std::size_t start = 0, end; start < text.size(); start = end + 1) {
        end = text.find_first_of(" \n", start);
        if (end == text.npos) end = text.size();
        if (end > start) words.push_back(std::string_view(text).substr(start, end - start));
    }
    int64_t total = 0;
    for (int r = 0; r < 20; r++) {
        std::unordered_map<std::string_view, int64_t> counts;
        for (auto w : words) total += ++counts[w];
        total += int64_t(counts.size());
    }
    std::printf("%lld\n", static_cast<long long>(total));
}
