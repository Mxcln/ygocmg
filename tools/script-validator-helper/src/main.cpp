#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <map>
#include <optional>
#include <set>
#include <stdexcept>
#include <string>
#include <vector>

#include "card_data.h"
#include "common.h"
#include "json.hpp"
#include "ocgapi.h"

namespace json = ygo_helper_json;
namespace fs = std::filesystem;

namespace {

constexpr const char* kStage = "ocgcore_init";
constexpr const char* kHelperVersion = "0.1.0";

struct Issue {
  std::string severity;
  std::string code;
  std::string message;
  std::optional<int> line;
  std::optional<int> column;
  std::string suggestion;
};

struct CardInput {
  uint32_t code = 0;
  uint32_t alias = 0;
  std::vector<uint16_t> setcodes;
  uint32_t type = 0;
  uint32_t level = 0;
  uint32_t attribute = 0;
  uint32_t race = 0;
  int32_t attack = 0;
  int32_t defense = 0;
  uint32_t lscale = 0;
  uint32_t rscale = 0;
  uint32_t link_marker = 0;
  uint32_t rule_code = 0;
};

struct ValidationContext {
  std::string request_id;
  CardInput card;
  fs::path helper_root;
  fs::path core_script_root;
  std::map<std::string, std::string> served_scripts;
  std::vector<Issue> issues;
  std::vector<std::string> log;
  std::set<std::string> issue_keys;
  std::set<std::string> log_entries;
  int32_t field_count = 0;
};

ValidationContext* g_context = nullptr;

std::string read_file(const fs::path& path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("Could not open file: " + path.string());
  }
  return std::string(std::istreambuf_iterator<char>(input),
                     std::istreambuf_iterator<char>());
}

std::string normalize_script_name(std::string value) {
  std::replace(value.begin(), value.end(), '\\', '/');
  return value;
}

bool starts_with(const std::string& value, const std::string& prefix) {
  return value.size() >= prefix.size() &&
         value.compare(0, prefix.size(), prefix) == 0;
}

std::string file_name_from_script(const std::string& script_name) {
  const auto slash = script_name.find_last_of('/');
  if (slash == std::string::npos) {
    return script_name;
  }
  return script_name.substr(slash + 1);
}

bool is_core_script_name(const std::string& script_name) {
  const std::string file_name = file_name_from_script(script_name);
  return file_name == "constant.lua" || file_name == "utility.lua" ||
         file_name == "procedure.lua";
}

const json::Value* object_find(const json::Object& object,
                               const std::string& key) {
  const auto it = object.find(key);
  if (it == object.end()) {
    return nullptr;
  }
  return &it->second;
}

double required_number(const json::Object& object, const std::string& key) {
  return json::object_at(object, key).as_number();
}

void require_integer(double value, const std::string& key) {
  if (!std::isfinite(value) || std::floor(value) != value) {
    throw std::runtime_error("JSON field must be an integer: " + key);
  }
}

uint32_t read_u32(const json::Object& object, const std::string& key) {
  const double value = required_number(object, key);
  require_integer(value, key);
  if (value < 0 ||
      value > static_cast<double>(std::numeric_limits<uint32_t>::max())) {
    throw std::runtime_error("JSON field is outside uint32 range: " + key);
  }
  return static_cast<uint32_t>(value);
}

uint16_t read_u16_value(const json::Value& value, const std::string& key) {
  const double number = value.as_number();
  require_integer(number, key);
  if (number < 0 ||
      number > static_cast<double>(std::numeric_limits<uint16_t>::max())) {
    throw std::runtime_error("JSON field is outside uint16 range: " + key);
  }
  return static_cast<uint16_t>(number);
}

int32_t read_i32(const json::Object& object, const std::string& key) {
  const double value = required_number(object, key);
  require_integer(value, key);
  if (value < static_cast<double>(std::numeric_limits<int32_t>::min()) ||
      value > static_cast<double>(std::numeric_limits<int32_t>::max())) {
    throw std::runtime_error("JSON field is outside int32 range: " + key);
  }
  return static_cast<int32_t>(value);
}

fs::path find_helper_root(const char* executable_path,
                          const fs::path& input_path) {
  fs::path current = fs::absolute(executable_path).parent_path();
  for (int depth = 0; depth < 8 && !current.empty(); ++depth) {
    if (fs::exists(current / "fixtures") && fs::exists(current / "scripts")) {
      return current;
    }
    if (current == current.parent_path()) {
      break;
    }
    current = current.parent_path();
  }

  const fs::path input_parent = input_path.parent_path();
  if (input_parent.filename() == "fixtures") {
    return input_parent.parent_path();
  }
  return fs::current_path();
}

fs::path resolve_from_helper_root(const fs::path& helper_root,
                                  const std::string& raw_path) {
  fs::path path(raw_path);
  if (path.is_absolute()) {
    return path.lexically_normal();
  }
  return (helper_root / path).lexically_normal();
}

void add_issue(ValidationContext& context,
               std::string severity,
               std::string code,
               std::string message,
               std::optional<int> line,
               std::optional<int> column,
               std::string suggestion) {
  const std::string key = severity + "|" + code + "|" + message + "|" +
                          (line ? std::to_string(*line) : "") + "|" +
                          (column ? std::to_string(*column) : "");
  if (!context.issue_keys.insert(key).second) {
    return;
  }
  context.issues.push_back(Issue{std::move(severity), std::move(code),
                                 std::move(message), line, column,
                                 std::move(suggestion)});
}

void add_log(ValidationContext& context, const std::string& message) {
  if (message.empty() || !context.log_entries.insert(message).second) {
    return;
  }
  context.log.push_back(message);
}

std::optional<int> parse_lua_line(const std::string& message) {
  const std::string marker = "\"]:";
  auto pos = message.find(marker);
  if (pos == std::string::npos) {
    return std::nullopt;
  }
  pos += marker.size();
  const auto end = message.find(':', pos);
  if (end == std::string::npos || end == pos) {
    return std::nullopt;
  }
  try {
    return std::stoi(message.substr(pos, end - pos));
  } catch (...) {
    return std::nullopt;
  }
}

bool is_syntax_message(const std::string& message) {
  return message.find(" expected") != std::string::npos ||
         message.find("near <eof>") != std::string::npos ||
         message.find("unexpected symbol") != std::string::npos ||
         message.find("malformed number") != std::string::npos ||
         message.find("syntax") != std::string::npos;
}

void record_ocgcore_message(ValidationContext& context,
                            const std::string& message) {
  add_log(context, message);
  const std::optional<int> line = parse_lua_line(message);
  if (line && is_syntax_message(message)) {
    add_issue(context, "error", "lua_syntax_error", message, line,
              std::nullopt, "Fix the Lua syntax error reported by ocgcore.");
    return;
  }

  if (message.find("CallCardFunction") != std::string::npos ||
      message.find("attempt to") != std::string::npos ||
      message.find("[string ") != std::string::npos) {
    add_issue(context, "error", "lua_runtime_error", message, line,
              std::nullopt,
              "Fix the runtime error raised while ocgcore initialized the "
              "card script.");
    return;
  }

  add_issue(context, "error", "ocgcore_message", message, line, std::nullopt,
            "Inspect the ocgcore message for the script initialization issue.");
}

CardInput parse_card(const json::Object& card_object) {
  CardInput card;
  card.code = read_u32(card_object, "code");
  card.alias = read_u32(card_object, "alias");
  card.type = read_u32(card_object, "type");
  card.level = read_u32(card_object, "level");
  card.attribute = read_u32(card_object, "attribute");
  card.race = read_u32(card_object, "race");
  card.attack = read_i32(card_object, "attack");
  card.defense = read_i32(card_object, "defense");
  card.lscale = read_u32(card_object, "lscale");
  card.rscale = read_u32(card_object, "rscale");
  card.link_marker = read_u32(card_object, "linkMarker");
  card.rule_code = read_u32(card_object, "ruleCode");

  const auto& setcodes = json::object_at(card_object, "setcodes").as_array();
  for (std::size_t i = 0; i < setcodes.size() && i < SIZE_SETCODE; ++i) {
    card.setcodes.push_back(read_u16_value(setcodes[i], "setcodes"));
  }
  return card;
}

ValidationContext parse_request(const json::Value& input,
                                const fs::path& input_path,
                                const char* executable_path) {
  const auto& root = input.as_object();
  ValidationContext context;
  context.request_id = json::object_at(root, "requestId").as_string();
  context.helper_root = find_helper_root(executable_path, input_path);
  context.card = parse_card(json::object_at(root, "card").as_object());
  context.core_script_root = resolve_from_helper_root(
      context.helper_root, json::object_at(root, "coreScriptRoot").as_string());

  const auto& scripts = json::object_at(root, "scripts").as_object();
  for (const auto& [script_name, script_value] : scripts) {
    const std::string normalized = normalize_script_name(script_name);
    context.served_scripts.emplace(normalized, script_value.as_string());
  }

  if (const json::Value* level = object_find(root, "level")) {
    const std::string& requested_level = level->as_string();
    if (requested_level != kStage) {
      add_issue(context, "error", "unsupported_level",
                "Only ocgcore_init requests are supported by this helper.",
                std::nullopt, std::nullopt,
                "Set level to ocgcore_init for this helper.");
    }
  }
  return context;
}

byte* helper_script_reader(const char* script_name, int* len) {
  if (len) {
    *len = 0;
  }
  if (!g_context || !script_name) {
    return nullptr;
  }

  const std::string normalized = normalize_script_name(script_name);
  auto served = g_context->served_scripts.find(normalized);
  if (served == g_context->served_scripts.end() &&
      is_core_script_name(normalized)) {
    const std::string file_name = file_name_from_script(normalized);
    const fs::path core_script = g_context->core_script_root / file_name;
    try {
      served =
          g_context->served_scripts
              .emplace(normalized, read_file(core_script))
              .first;
    } catch (const std::exception&) {
      add_issue(*g_context, "error", "missing_core_script",
                "Could not load required ocgcore script: " + normalized,
                std::nullopt, std::nullopt,
                "Verify coreScriptRoot points at ygopro-scripts.");
      return nullptr;
    }
  }

  if (served == g_context->served_scripts.end()) {
    if (starts_with(normalized, "./script/c")) {
      add_issue(*g_context, "error", "missing_card_script",
                "Card script was not provided: " + normalized, std::nullopt,
                std::nullopt,
                "Provide the card script in the request scripts map.");
    } else {
      add_issue(*g_context, "error", "missing_script",
                "Script was not provided: " + normalized, std::nullopt,
                std::nullopt,
                "Provide the requested script or core script root.");
    }
    return nullptr;
  }

  if (len) {
    *len = static_cast<int>(served->second.size());
  }
  return reinterpret_cast<byte*>(served->second.data());
}

uint32_t helper_card_reader(uint32_t code, card_data* data) {
  if (!g_context || !data) {
    return 0;
  }
  if (code != g_context->card.code) {
    data->clear();
    add_issue(*g_context, "error", "missing_card_data",
              "Card data was not provided for code " + std::to_string(code),
              std::nullopt, std::nullopt,
              "Provide card metadata matching the script code.");
    return 0;
  }

  data->clear();
  data->code = g_context->card.code;
  data->alias = g_context->card.alias;
  for (std::size_t i = 0;
       i < g_context->card.setcodes.size() && i < SIZE_SETCODE; ++i) {
    data->setcode[i] = g_context->card.setcodes[i];
  }
  data->type = g_context->card.type;
  data->level = g_context->card.level;
  data->attribute = g_context->card.attribute;
  data->race = g_context->card.race;
  data->attack = g_context->card.attack;
  data->defense = g_context->card.defense;
  data->lscale = g_context->card.lscale;
  data->rscale = g_context->card.rscale;
  data->link_marker = g_context->card.link_marker;
  data->rule_code = g_context->card.rule_code;
  return 1;
}

uint32_t helper_message_handler(intptr_t pduel, uint32_t msg_type) {
  if (!g_context || msg_type != 1) {
    return 0;
  }
  char buffer[256]{};
  get_log_message(pduel, buffer);
  if (buffer[0]) {
    record_ocgcore_message(*g_context, buffer);
  }
  return 0;
}

bool has_error_issue(const ValidationContext& context) {
  for (const Issue& issue : context.issues) {
    if (issue.severity == "error") {
      return true;
    }
  }
  return false;
}

void run_ocgcore_validation(ValidationContext& context) {
  set_script_reader(helper_script_reader);
  set_card_reader(helper_card_reader);
  set_message_handler(helper_message_handler);

  g_context = &context;
  intptr_t duel = 0;
  try {
    duel = create_duel(0);
    set_player_info(duel, 0, 8000, 0, 1);
    set_player_info(duel, 1, 8000, 0, 1);
    new_card(duel, context.card.code, 0, 0, LOCATION_MZONE, 0,
             POS_FACEUP_ATTACK);
    context.field_count = query_field_count(duel, 0, LOCATION_MZONE);
    end_duel(duel);
    duel = 0;
  } catch (...) {
    if (duel) {
      end_duel(duel);
    }
    g_context = nullptr;
    throw;
  }
  g_context = nullptr;

  if (!has_error_issue(context) && context.field_count != 1) {
    add_issue(context, "error", "helper_error",
              "ocgcore did not place the validation card on the field.",
              std::nullopt, std::nullopt,
              "Inspect helper input and ocgcore callback behavior.");
  }
}

json::Value issue_to_json(const Issue& issue) {
  json::Object object;
  object["severity"] = issue.severity;
  object["stage"] = kStage;
  object["code"] = issue.code;
  object["message"] = issue.message;
  if (issue.line) {
    object["line"] = *issue.line;
  } else {
    object["line"] = nullptr;
  }
  if (issue.column) {
    object["column"] = *issue.column;
  } else {
    object["column"] = nullptr;
  }
  object["suggestion"] = issue.suggestion;
  return json::Value(object);
}

json::Value output_for_context(const ValidationContext& context,
                               int64_t duration_ms) {
  json::Array issues;
  for (const Issue& issue : context.issues) {
    issues.push_back(issue_to_json(issue));
  }

  json::Array log;
  for (const std::string& entry : context.log) {
    log.push_back(entry);
  }

  json::Object output;
  output["requestId"] = context.request_id;
  output["stage"] = kStage;
  output["status"] = has_error_issue(context) ? "fail" : "pass";
  output["durationMs"] = static_cast<double>(duration_ms);
  output["issues"] = issues;
  output["log"] = log;
  output["helperVersion"] = kHelperVersion;
  return json::Value(output);
}

int run(int argc, char** argv) {
  if (argc != 3 || std::string(argv[1]) != "--input") {
    std::cerr << "Usage: script-validator-helper --input <input.json>\n";
    return 2;
  }

  const auto started_at = std::chrono::steady_clock::now();
  const fs::path input_path = fs::absolute(argv[2]);
  ValidationContext context = parse_request(json::parse(read_file(input_path)),
                                            input_path, argv[0]);
  run_ocgcore_validation(context);
  const auto duration_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                               std::chrono::steady_clock::now() - started_at)
                               .count();
  std::cout << json::stringify(output_for_context(context, duration_ms))
            << "\n";
  return 0;
}

}  // namespace

int main(int argc, char** argv) {
  try {
    return run(argc, argv);
  } catch (const std::exception& error) {
    std::cerr << "script-validator-helper error: " << error.what() << "\n";
    return 1;
  }
}
