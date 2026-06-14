import { invokeApi } from "./invoke";
import type {
  LuaValidationReport,
  ValidateLuaScriptInput,
} from "../contracts/script";

export const scriptApi = {
  validateLuaScript(input: ValidateLuaScriptInput) {
    return invokeApi<LuaValidationReport>("validate_lua_script", { input });
  },
};
