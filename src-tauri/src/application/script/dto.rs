use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{CardId, PackId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationLevelDto {
    Static,
    OcgcoreInit,
    Smoke,
    Scenario,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationStatusDto {
    Pass,
    Warning,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationConfidenceDto {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationIssueSeverityDto {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateLuaScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub script_text: Option<String>,
    #[serde(default)]
    pub levels: Option<Vec<LuaValidationLevelDto>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaValidationIssueDto {
    pub severity: LuaValidationIssueSeverityDto,
    pub stage: LuaValidationLevelDto,
    pub code: String,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaValidationStageResultDto {
    pub stage: LuaValidationLevelDto,
    pub status: LuaValidationStatusDto,
    pub duration_ms: u64,
    pub issues: Vec<LuaValidationIssueDto>,
    #[serde(default)]
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaValidationReportDto {
    pub status: LuaValidationStatusDto,
    pub confidence: LuaValidationConfidenceDto,
    pub summary: String,
    pub issues: Vec<LuaValidationIssueDto>,
    pub stages: Vec<LuaValidationStageResultDto>,
    pub limitations: Vec<String>,
}
