#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

#include "json.hpp"

namespace json = ygo_helper_json;

namespace {

std::string read_file(const std::string& path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("Could not open input file: " + path);
  }
  return std::string(std::istreambuf_iterator<char>(input),
                     std::istreambuf_iterator<char>());
}

std::string read_request_id(const json::Value& root) {
  const auto& object = root.as_object();
  return json::object_at(object, "requestId").as_string();
}

json::Value skeleton_output(const std::string& request_id) {
  json::Object issue;
  issue["severity"] = "info";
  issue["stage"] = "ocgcore_init";
  issue["code"] = "helper_not_linked";
  issue["message"] =
      "The helper CLI parsed input but ocgcore is not linked in this task.";
  issue["line"] = nullptr;
  issue["column"] = nullptr;
  issue["suggestion"] = "Build the ocgcore integration task.";

  json::Object output;
  output["requestId"] = request_id;
  output["stage"] = "ocgcore_init";
  output["status"] = "inconclusive";
  output["durationMs"] = 0;
  output["issues"] = json::Array{json::Value(issue)};
  output["log"] = json::Array{};
  output["helperVersion"] = "0.1.0";
  return json::Value(output);
}

int run(int argc, char** argv) {
  if (argc != 3 || std::string(argv[1]) != "--input") {
    std::cerr << "Usage: script-validator-helper --input <input.json>\n";
    return 2;
  }

  json::Value input = json::parse(read_file(argv[2]));
  std::cout << json::stringify(skeleton_output(read_request_id(input))) << "\n";
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
