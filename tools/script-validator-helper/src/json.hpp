#pragma once

#include <cctype>
#include <cstdint>
#include <iomanip>
#include <map>
#include <sstream>
#include <stdexcept>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace ygo_helper_json {

struct Value;
using Object = std::map<std::string, Value>;
using Array = std::vector<Value>;

struct Value {
  using Storage =
      std::variant<std::nullptr_t, bool, double, std::string, Array, Object>;

  Storage storage;

  Value() : storage(nullptr) {}
  Value(std::nullptr_t) : storage(nullptr) {}
  Value(bool value) : storage(value) {}
  Value(double value) : storage(value) {}
  Value(int value) : storage(static_cast<double>(value)) {}
  Value(std::string value) : storage(std::move(value)) {}
  Value(const char* value) : storage(std::string(value)) {}
  Value(Array value) : storage(std::move(value)) {}
  Value(Object value) : storage(std::move(value)) {}

  const Object& as_object() const {
    if (!std::holds_alternative<Object>(storage))
      throw std::runtime_error("JSON value is not an object");
    return std::get<Object>(storage);
  }

  const Array& as_array() const {
    if (!std::holds_alternative<Array>(storage))
      throw std::runtime_error("JSON value is not an array");
    return std::get<Array>(storage);
  }

  const std::string& as_string() const {
    if (!std::holds_alternative<std::string>(storage))
      throw std::runtime_error("JSON value is not a string");
    return std::get<std::string>(storage);
  }

  double as_number() const {
    if (!std::holds_alternative<double>(storage))
      throw std::runtime_error("JSON value is not a number");
    return std::get<double>(storage);
  }

  bool is_null() const { return std::holds_alternative<std::nullptr_t>(storage); }
};

class Parser {
 public:
  explicit Parser(const std::string& input) : input_(input) {}

  Value parse() {
    skip_ws();
    Value value = parse_value();
    skip_ws();
    if (pos_ != input_.size())
      throw std::runtime_error("Unexpected trailing JSON content");
    return value;
  }

 private:
  const std::string& input_;
  std::size_t pos_ = 0;

  void skip_ws() {
    while (pos_ < input_.size() &&
           std::isspace(static_cast<unsigned char>(input_[pos_]))) {
      ++pos_;
    }
  }

  char peek() const {
    if (pos_ >= input_.size())
      throw std::runtime_error("Unexpected end of JSON");
    return input_[pos_];
  }

  char get() {
    char ch = peek();
    ++pos_;
    return ch;
  }

  bool consume(char expected) {
    skip_ws();
    if (pos_ < input_.size() && input_[pos_] == expected) {
      ++pos_;
      return true;
    }
    return false;
  }

  void expect_literal(const char* literal) {
    for (const char* p = literal; *p; ++p) {
      if (get() != *p)
        throw std::runtime_error("Invalid JSON literal");
    }
  }

  Value parse_value() {
    skip_ws();
    char ch = peek();
    if (ch == '{')
      return Value(parse_object());
    if (ch == '[')
      return Value(parse_array());
    if (ch == '"')
      return Value(parse_string());
    if (ch == 't') {
      expect_literal("true");
      return Value(true);
    }
    if (ch == 'f') {
      expect_literal("false");
      return Value(false);
    }
    if (ch == 'n') {
      expect_literal("null");
      return Value(nullptr);
    }
    return Value(parse_number());
  }

  Object parse_object() {
    Object object;
    if (get() != '{')
      throw std::runtime_error("Expected object");
    skip_ws();
    if (consume('}'))
      return object;
    while (true) {
      skip_ws();
      std::string key = parse_string();
      if (!consume(':'))
        throw std::runtime_error("Expected ':' after object key");
      object.emplace(std::move(key), parse_value());
      if (consume('}'))
        break;
      if (!consume(','))
        throw std::runtime_error("Expected ',' between object members");
    }
    return object;
  }

  Array parse_array() {
    Array array;
    if (get() != '[')
      throw std::runtime_error("Expected array");
    skip_ws();
    if (consume(']'))
      return array;
    while (true) {
      array.push_back(parse_value());
      if (consume(']'))
        break;
      if (!consume(','))
        throw std::runtime_error("Expected ',' between array values");
    }
    return array;
  }

  std::string parse_string() {
    if (get() != '"')
      throw std::runtime_error("Expected string");
    std::string value;
    while (true) {
      char ch = get();
      if (ch == '"')
        break;
      if (ch != '\\') {
        value.push_back(ch);
        continue;
      }
      char escaped = get();
      switch (escaped) {
        case '"':
        case '\\':
        case '/':
          value.push_back(escaped);
          break;
        case 'b':
          value.push_back('\b');
          break;
        case 'f':
          value.push_back('\f');
          break;
        case 'n':
          value.push_back('\n');
          break;
        case 'r':
          value.push_back('\r');
          break;
        case 't':
          value.push_back('\t');
          break;
        case 'u':
          value += parse_unicode_escape();
          break;
        default:
          throw std::runtime_error("Invalid JSON string escape");
      }
    }
    return value;
  }

  std::string parse_unicode_escape() {
    uint32_t value = 0;
    for (int i = 0; i < 4; ++i) {
      char ch = get();
      value <<= 4;
      if (ch >= '0' && ch <= '9')
        value += static_cast<uint32_t>(ch - '0');
      else if (ch >= 'a' && ch <= 'f')
        value += static_cast<uint32_t>(ch - 'a' + 10);
      else if (ch >= 'A' && ch <= 'F')
        value += static_cast<uint32_t>(ch - 'A' + 10);
      else
        throw std::runtime_error("Invalid JSON unicode escape");
    }
    if (value <= 0x7F)
      return std::string(1, static_cast<char>(value));
    if (value <= 0x7FF) {
      return std::string{
          static_cast<char>(0xC0 | ((value >> 6) & 0x1F)),
          static_cast<char>(0x80 | (value & 0x3F)),
      };
    }
    return std::string{
        static_cast<char>(0xE0 | ((value >> 12) & 0x0F)),
        static_cast<char>(0x80 | ((value >> 6) & 0x3F)),
        static_cast<char>(0x80 | (value & 0x3F)),
    };
  }

  double parse_number() {
    std::size_t start = pos_;
    if (input_[pos_] == '-')
      ++pos_;
    while (pos_ < input_.size() &&
           std::isdigit(static_cast<unsigned char>(input_[pos_]))) {
      ++pos_;
    }
    if (pos_ < input_.size() && input_[pos_] == '.') {
      ++pos_;
      while (pos_ < input_.size() &&
             std::isdigit(static_cast<unsigned char>(input_[pos_]))) {
        ++pos_;
      }
    }
    if (pos_ < input_.size() && (input_[pos_] == 'e' || input_[pos_] == 'E')) {
      ++pos_;
      if (pos_ < input_.size() && (input_[pos_] == '+' || input_[pos_] == '-'))
        ++pos_;
      while (pos_ < input_.size() &&
             std::isdigit(static_cast<unsigned char>(input_[pos_]))) {
        ++pos_;
      }
    }
    return std::stod(input_.substr(start, pos_ - start));
  }
};

inline Value parse(const std::string& text) {
  return Parser(text).parse();
}

inline std::string escape_string(const std::string& value) {
  std::ostringstream out;
  out << '"';
  for (char ch : value) {
    switch (ch) {
      case '"':
        out << "\\\"";
        break;
      case '\\':
        out << "\\\\";
        break;
      case '\b':
        out << "\\b";
        break;
      case '\f':
        out << "\\f";
        break;
      case '\n':
        out << "\\n";
        break;
      case '\r':
        out << "\\r";
        break;
      case '\t':
        out << "\\t";
        break;
      default:
        if (static_cast<unsigned char>(ch) < 0x20) {
          out << "\\u" << std::hex << std::setw(4) << std::setfill('0')
              << static_cast<int>(static_cast<unsigned char>(ch));
        } else {
          out << ch;
        }
    }
  }
  out << '"';
  return out.str();
}

inline std::string stringify(const Value& value);

inline std::string stringify_array(const Array& array) {
  std::ostringstream out;
  out << '[';
  for (std::size_t i = 0; i < array.size(); ++i) {
    if (i)
      out << ',';
    out << stringify(array[i]);
  }
  out << ']';
  return out.str();
}

inline std::string stringify_object(const Object& object) {
  std::ostringstream out;
  out << '{';
  bool first = true;
  for (const auto& [key, value] : object) {
    if (!first)
      out << ',';
    first = false;
    out << escape_string(key) << ':' << stringify(value);
  }
  out << '}';
  return out.str();
}

inline std::string stringify(const Value& value) {
  if (std::holds_alternative<std::nullptr_t>(value.storage))
    return "null";
  if (std::holds_alternative<bool>(value.storage))
    return std::get<bool>(value.storage) ? "true" : "false";
  if (std::holds_alternative<double>(value.storage)) {
    std::ostringstream out;
    out << std::setprecision(15) << std::get<double>(value.storage);
    return out.str();
  }
  if (std::holds_alternative<std::string>(value.storage))
    return escape_string(std::get<std::string>(value.storage));
  if (std::holds_alternative<Array>(value.storage))
    return stringify_array(std::get<Array>(value.storage));
  return stringify_object(std::get<Object>(value.storage));
}

inline const Value& object_at(const Object& object, const std::string& key) {
  auto it = object.find(key);
  if (it == object.end())
    throw std::runtime_error("Missing JSON field: " + key);
  return it->second;
}

}  // namespace ygo_helper_json
