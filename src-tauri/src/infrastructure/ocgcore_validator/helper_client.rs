use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationStageResultDto, LuaValidationStatusDto,
};
use crate::domain::common::error::AppResult;
use crate::infrastructure::ocgcore_validator::input::HelperInput;
use crate::infrastructure::ocgcore_validator::output::HelperOutput;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(3000);

#[derive(Debug, Clone)]
pub struct OcgcoreValidatorHelperClient {
    executable: PathBuf,
    prefix_args: Vec<String>,
    timeout: Duration,
}

impl OcgcoreValidatorHelperClient {
    pub fn discover() -> Self {
        if let Some(path) = std::env::var_os("YGOCMG_SCRIPT_VALIDATOR_HELPER") {
            return Self {
                executable: PathBuf::from(path),
                prefix_args: Vec::new(),
                timeout: DEFAULT_TIMEOUT,
            };
        }

        let candidates = [
            "../tools/script-validator-helper/build/bin/Release/script-validator-helper.exe",
            "../tools/script-validator-helper/build/bin/script-validator-helper.exe",
            "tools/script-validator-helper/build/bin/Release/script-validator-helper.exe",
            "tools/script-validator-helper/build/bin/script-validator-helper.exe",
        ];
        let executable = candidates
            .iter()
            .map(PathBuf::from)
            .find(|path| path.exists())
            .unwrap_or_else(|| PathBuf::from(candidates[0]));

        Self {
            executable,
            prefix_args: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    #[cfg(test)]
    pub fn for_test(executable: PathBuf, prefix_args: Vec<String>, timeout: Duration) -> Self {
        Self {
            executable,
            prefix_args,
            timeout,
        }
    }

    pub fn run(&self, input: &HelperInput) -> AppResult<LuaValidationStageResultDto> {
        if self.prefix_args.is_empty() && !self.executable.exists() {
            return Ok(failure_stage(
                "helper_not_found",
                "The ocgcore script validator helper executable was not found.",
                Some(
                    "Run powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1 before requesting ocgcore_init validation.",
                ),
                Vec::new(),
            ));
        }

        let temp_dir = match create_temp_dir() {
            Ok(path) => path,
            Err(message) => {
                return Ok(failure_stage(
                    "helper_temp_io_failed",
                    &format!("Could not create helper input directory: {message}"),
                    Some("Check temporary directory permissions and try again."),
                    Vec::new(),
                ));
            }
        };
        let input_path = temp_dir.join("input.json");
        let encoded = match serde_json::to_vec_pretty(input) {
            Ok(value) => value,
            Err(source) => {
                cleanup_temp_dir(&temp_dir);
                return Ok(failure_stage(
                    "helper_input_encode_failed",
                    &format!("Could not encode helper input JSON: {source}"),
                    None,
                    Vec::new(),
                ));
            }
        };
        if let Err(source) = fs::write(&input_path, encoded) {
            cleanup_temp_dir(&temp_dir);
            return Ok(failure_stage(
                "helper_temp_io_failed",
                &format!("Could not write helper input JSON: {source}"),
                Some("Check temporary directory permissions and try again."),
                Vec::new(),
            ));
        }

        let result = self.run_process(&input_path);
        cleanup_temp_dir(&temp_dir);
        Ok(result)
    }

    fn run_process(&self, input_path: &Path) -> LuaValidationStageResultDto {
        let mut command = Command::new(&self.executable);
        command
            .args(&self.prefix_args)
            .arg("--input")
            .arg(input_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(source) => {
                let code = if source.kind() == std::io::ErrorKind::NotFound {
                    "helper_not_found"
                } else {
                    "helper_spawn_failed"
                };
                return failure_stage(
                    code,
                    &format!("Could not start ocgcore script validator helper: {source}"),
                    Some("Build the helper and verify YGOCMG_SCRIPT_VALIDATOR_HELPER if it is set."),
                    Vec::new(),
                );
            }
        };

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) if started.elapsed() >= self.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return failure_stage(
                        "helper_timeout",
                        "The ocgcore script validator helper timed out.",
                        Some("Try again after reducing script complexity or rebuilding the helper."),
                        Vec::new(),
                    );
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(source) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return failure_stage(
                        "helper_process_failed",
                        &format!("Could not observe helper process status: {source}"),
                        None,
                        Vec::new(),
                    );
                }
            }
        }

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(source) => {
                return failure_stage(
                    "helper_process_failed",
                    &format!("Could not collect helper process output: {source}"),
                    None,
                    Vec::new(),
                );
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let mut log = Vec::new();
        if !stderr.is_empty() {
            log.push(stderr.clone());
        }
        if !stdout.is_empty() && !output.status.success() {
            log.push(stdout.clone());
        }

        if !output.status.success() {
            return failure_stage(
                "helper_failed",
                &format!(
                    "The ocgcore script validator helper exited with status {}.",
                    output.status
                ),
                Some("Inspect helper stderr and rebuild the helper if needed."),
                log,
            );
        }
        if stdout.is_empty() {
            return failure_stage(
                "helper_invalid_output",
                "The ocgcore script validator helper wrote no stdout JSON.",
                Some("Rebuild the helper and try validation again."),
                log,
            );
        }

        match serde_json::from_str::<HelperOutput>(&stdout)
            .map_err(|source| source.to_string())
            .and_then(HelperOutput::into_stage_result)
        {
            Ok(stage) => stage,
            Err(message) => {
                if !stdout.is_empty() {
                    log.push(stdout);
                }
                failure_stage(
                    "helper_invalid_output",
                    &format!("Could not parse helper stdout JSON: {message}"),
                    Some("Rebuild the helper and try validation again."),
                    log,
                )
            }
        }
    }
}

fn create_temp_dir() -> Result<PathBuf, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| source.to_string())?
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ygocmg-script-validator-{}-{now}",
        std::process::id()
    ));
    fs::create_dir_all(&path).map_err(|source| source.to_string())?;
    Ok(path)
}

fn cleanup_temp_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub fn failure_stage(
    code: &str,
    message: &str,
    suggestion: Option<&str>,
    log: Vec<String>,
) -> LuaValidationStageResultDto {
    LuaValidationStageResultDto {
        stage: LuaValidationLevelDto::OcgcoreInit,
        status: LuaValidationStatusDto::Inconclusive,
        duration_ms: 0,
        issues: vec![LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Info,
            stage: LuaValidationLevelDto::OcgcoreInit,
            code: code.to_string(),
            message: message.to_string(),
            line: None,
            column: None,
            suggestion: suggestion.map(str::to_string),
        }],
        log,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use tempfile::tempdir;

    use crate::application::script::dto::LuaValidationStatusDto;
    use crate::infrastructure::ocgcore_validator::input::{HelperCardInput, HelperInput};

    use super::*;

    fn minimal_input() -> HelperInput {
        HelperInput {
            request_id: "req-1".to_string(),
            level: "ocgcore_init".to_string(),
            card: HelperCardInput {
                code: 99999999,
                alias: 0,
                setcodes: Vec::new(),
                raw_type: 33,
                level: 4,
                attribute: 16,
                race: 1,
                attack: 0,
                defense: 0,
                lscale: 0,
                rscale: 0,
                link_marker: 0,
                rule_code: 0,
            },
            scripts: std::collections::BTreeMap::from([(
                "./script/c99999999.lua".to_string(),
                "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n".to_string(),
            )]),
            core_script_root: "D:/Game/YGODIY/ygocmg/third_party/ygopro-scripts".to_string(),
            timeout_ms: 3000,
        }
    }

    fn write_fake_helper(dir: &Path, body: &str) -> PathBuf {
        let script = dir.join("fake-helper.ps1");
        fs::write(&script, body).unwrap();
        script
    }

    fn powershell_client(script: PathBuf, timeout: Duration) -> OcgcoreValidatorHelperClient {
        OcgcoreValidatorHelperClient::for_test(
            "powershell".into(),
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                script.display().to_string(),
            ],
            timeout,
        )
    }

    #[test]
    fn run_returns_helper_stage_for_valid_stdout() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Write-Output '{"requestId":"req-1","stage":"ocgcore_init","status":"pass","durationMs":3,"issues":[],"log":[],"helperVersion":"test"}'
"#,
        );
        let client = powershell_client(script, Duration::from_secs(10));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Pass);
        assert_eq!(stage.issues.len(), 0);
    }

    #[test]
    fn missing_helper_returns_inconclusive_stage() {
        let client = OcgcoreValidatorHelperClient::for_test(
            "Z:/missing/script-validator-helper.exe".into(),
            Vec::new(),
            Duration::from_millis(100),
        );

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_not_found");
    }

    #[test]
    fn invalid_stdout_returns_inconclusive_stage() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Write-Output 'not json'
"#,
        );
        let client = powershell_client(script, Duration::from_secs(10));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_invalid_output");
    }

    #[test]
    fn non_zero_exit_returns_inconclusive_stage_with_log() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Write-Error 'boom'
exit 9
"#,
        );
        let client = powershell_client(script, Duration::from_secs(10));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_failed");
        assert!(stage.log.iter().any(|entry| entry.contains("boom")));
    }

    #[test]
    fn timeout_returns_inconclusive_stage() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Start-Sleep -Seconds 2
Write-Output '{"requestId":"req-1","stage":"ocgcore_init","status":"pass","durationMs":3,"issues":[],"log":[],"helperVersion":"test"}'
"#,
        );
        let client = powershell_client(script, Duration::from_millis(100));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_timeout");
    }
}
