// Parse each line of the text into an expression tree and evaluate it, repeated.

#include <charconv>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <iterator>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

using Toks = std::vector<std::string_view>;

struct Expr {
    char op;  // 'n' for a number, else '+', '-' or '*'
    int64_t value;
    std::unique_ptr<Expr> left, right;
};

using Ptr = std::unique_ptr<Expr>;

static Ptr node(char op, Ptr left, Ptr right) {
    return std::make_unique<Expr>(op, 0, std::move(left), std::move(right));
}

static Toks split(std::string_view s, char sep) {
    Toks parts;
    for (std::size_t start = 0, end; start < s.size(); start = end + 1) {
        end = s.find(sep, start);
        if (end == s.npos) end = s.size();
        if (end > start) parts.push_back(s.substr(start, end - start));
    }
    return parts;
}

static Ptr parse_term(const Toks& toks, std::size_t& pos);
static Ptr parse_factor(const Toks& toks, std::size_t& pos);

static Ptr parse_expr(const Toks& toks, std::size_t& pos) {
    Ptr e = parse_term(toks, pos);
    while (pos < toks.size() && (toks[pos] == "+" || toks[pos] == "-")) {
        char op = toks[pos][0];
        pos += 1;
        Ptr right = parse_term(toks, pos);
        e = node(op, std::move(e), std::move(right));
    }
    return e;
}

static Ptr parse_term(const Toks& toks, std::size_t& pos) {
    Ptr e = parse_factor(toks, pos);
    while (pos < toks.size() && toks[pos] == "*") {
        pos += 1;
        Ptr right = parse_factor(toks, pos);
        e = node('*', std::move(e), std::move(right));
    }
    return e;
}

static Ptr parse_factor(const Toks& toks, std::size_t& pos) {
    pos += 1;
    std::string_view tok = toks[pos - 1];
    if (tok == "(") {
        Ptr e = parse_expr(toks, pos);
        pos += 1;
        return e;
    }
    Ptr e = node('n', nullptr, nullptr);
    std::from_chars(tok.data(), tok.data() + tok.size(), e->value);
    return e;
}

static int64_t eval(const Expr& e) {
    switch (e.op) {
        case '+': return eval(*e.left) + eval(*e.right);
        case '-': return eval(*e.left) - eval(*e.right);
        case '*': return eval(*e.left) * eval(*e.right);
        default: return e.value;
    }
}

int main(int argc, char** argv) {
    if (argc != 2) return 2;
    std::ifstream file(argv[1]);
    std::string text((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    auto lines = split(text, '\n');
    uint64_t total = 0;
    for (int r = 0; r < 10; r++)
        for (auto line : lines) {
            std::size_t pos = 0;
            Ptr e = parse_expr(split(line, ' '), pos);
            total += uint64_t(eval(*e));
        }
    std::printf("%lld\n", static_cast<long long>(total));
}
