use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationTarget {
    pub scope: String,
    pub entity_id: Option<String>,
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationIssue {
    pub code: String,
    pub level: IssueLevel,
    pub target: ValidationTarget,
    pub params: BTreeMap<String, Value>,
}

impl ValidationTarget {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            entity_id: None,
            field: None,
        }
    }

    pub fn with_entity_id(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl ValidationIssue {
    pub fn error(code: impl Into<String>, target: ValidationTarget) -> Self {
        Self {
            code: code.into(),
            level: IssueLevel::Error,
            target,
            params: BTreeMap::new(),
        }
    }

    pub fn warning(code: impl Into<String>, target: ValidationTarget) -> Self {
        Self {
            code: code.into(),
            level: IssueLevel::Warning,
            target,
            params: BTreeMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        let encoded = serde_json::to_value(value).unwrap_or(Value::Null);
        self.params.insert(key.into(), encoded);
        self
    }
}
