use crate::settings::AppSettings;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempFiles {
    paths: Vec<PathBuf>,
}

impl TempFiles {
    fn new() -> Self {
        Self { paths: Vec::new() }
    }

    fn write(&mut self, prefix: &str, contents: &str) -> Result<PathBuf, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Failed to get system time: {}", e))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "handy-post-process-{}-{}-{}.txt",
            prefix,
            std::process::id(),
            timestamp
        ));

        fs::write(&path, contents)
            .map_err(|e| format!("Failed to write temp file '{}': {}", path.display(), e))?;
        self.paths.push(path.clone());
        Ok(path)
    }
}

impl Drop for TempFiles {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = fs::remove_file(path);
        }
    }
}

fn apply_placeholders(
    value: &str,
    rendered_prompt: &str,
    prompt_template: &str,
    transcription: &str,
    prompt_file: &Path,
    transcript_file: &Path,
    cwd: Option<&str>,
) -> String {
    value
        .replace("{{prompt}}", rendered_prompt)
        .replace("{{prompt_template}}", prompt_template)
        .replace("{{transcript}}", transcription)
        .replace("{{prompt_file}}", &prompt_file.to_string_lossy())
        .replace("{{transcript_file}}", &transcript_file.to_string_lossy())
        .replace("{{cwd}}", cwd.unwrap_or(""))
}

fn preview(value: &str) -> String {
    const LIMIT: usize = 240;
    let trimmed = value.trim();
    if trimmed.len() <= LIMIT {
        return trimmed.to_string();
    }
    format!("{}...", &trimmed[..LIMIT])
}

pub async fn run_post_process_command(
    settings: &AppSettings,
    prompt_template: &str,
    transcription: &str,
) -> Result<String, String> {
    let program = settings
        .post_process_command_program
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or("Post-process command program is not configured")?
        .to_string();

    let rendered_prompt = prompt_template.replace("${output}", transcription);
    let args_template = settings.post_process_command_args.clone();
    let cwd = settings
        .post_process_command_working_directory
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let mut temp_files = TempFiles::new();
    let prompt_file = temp_files.write("prompt", &rendered_prompt)?;
    let transcript_file = temp_files.write("transcript", transcription)?;

    let args = args_template
        .iter()
        .map(|arg| {
            apply_placeholders(
                arg,
                &rendered_prompt,
                prompt_template,
                transcription,
                &prompt_file,
                &transcript_file,
                cwd.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    debug!(
        "Handing post-processing off to external command. program='{}', cwd='{}', args={:?}, prompt_preview='{}', transcript_preview='{}'",
        program,
        cwd.as_deref().unwrap_or(""),
        args,
        preview(&rendered_prompt),
        preview(transcription)
    );

    let program_for_task = program.clone();
    let cwd_for_task = cwd.clone();
    let rendered_prompt_for_task = rendered_prompt.clone();
    let prompt_template_for_task = prompt_template.to_string();
    let transcription_for_task = transcription.to_string();
    let prompt_file_for_task = prompt_file.clone();
    let transcript_file_for_task = transcript_file.clone();

    let output = tauri::async_runtime::spawn_blocking(move || {
        let mut command = Command::new(&program_for_task);
        command.args(&args);

        if let Some(ref dir) = cwd_for_task {
            command.current_dir(dir);
        }

        command.env("HANDY_PROMPT", &rendered_prompt_for_task);
        command.env("HANDY_PROMPT_TEMPLATE", &prompt_template_for_task);
        command.env("HANDY_TRANSCRIPT", &transcription_for_task);
        command.env(
            "HANDY_PROMPT_FILE",
            prompt_file_for_task.to_string_lossy().to_string(),
        );
        command.env(
            "HANDY_TRANSCRIPT_FILE",
            transcript_file_for_task.to_string_lossy().to_string(),
        );
        command.env("HANDY_WORKDIR", cwd_for_task.clone().unwrap_or_default());

        command
            .output()
            .map_err(|e| format!("Failed to execute post-process command '{}': {}", program_for_task, e))
    })
    .await
    .map_err(|e| format!("Post-process command task failed: {}", e))??;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    debug!(
        "External post-process command completed. success={}, exit_code={:?}, stdout_preview='{}', stderr_preview='{}'",
        output.status.success(),
        output.status.code(),
        preview(&stdout),
        preview(&stderr)
    );

    if !output.status.success() {
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit code {:?}", output.status.code())
        };
        return Err(format!("Post-process command failed: {}", detail));
    }

    if stdout.is_empty() {
        return Err("Post-process command completed but produced no stdout".to_string());
    }

    Ok(stdout)
}
