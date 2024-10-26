use std::error::Error;
use std::process::Command;
use tokio::process::Command as AsyncCommand;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn get_paths(path: &String) -> (bool, String) {
    let output = Command::new("git")
        .current_dir(path)
        .arg("diff")
        .arg("--name-only")
        .output()
        .expect("pwd command filed");

    if output.status.success() {
        match String::from_utf8(output.stdout) {
            Ok(val) => (false, val),
            Err(e) => (true, format!("{}", e)),
        }
    } else {
        match String::from_utf8(output.stderr) {
            Ok(err) => (true, err),
            Err(e) => (true, format!("{}", e)),
        }
    }
}

fn get_file_diff(path: &String) -> (bool, String) {
    let output = match Command::new("git")
        .current_dir(path)
        .arg("diff")
        .arg(path)
        .output()
    {
        Ok(output) => output,
        Err(e) => return (true, format!("{}", e)),
    };

    if output.status.success() {
        match String::from_utf8(output.stdout) {
            Ok(s) => (false, s),
            Err(e) => (false, format!("{}", e)),
        }
    } else {
        match String::from_utf8(output.stderr) {
            Ok(err) => (true, err),
            Err(e) => (true, format!("{}", e)),
        }
    }
}

#[tauri::command]
async fn get_commit_suggestion(path: String) -> Result<String, String> {

    let is_git_repo =  Command::new("git")
        .current_dir(&path)
        .arg("rev-parse")
        .arg("--git-dir")
        .output().expect("Error while checking if dir is git repo");
    
    println!("{}", String::from_utf8(is_git_repo.stdout).unwrap());
    
    if !is_git_repo.status.success() {
        return Ok("Not a git repo".to_string());
    }
    
    println!("Found a git repo");
    
    match generate_commit_suggestion(&path).await {
        Ok(commit_suggestion) => Ok(commit_suggestion),
        Err(e) => Err(e.to_string()),
    }
}

async fn generate_commit_suggestion(path: &String) -> Result<String, Box<dyn Error>> {
    let (path_err, paths_str) = get_paths(path);

    if path_err {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Path error occurred",
        )));
    }

    let collection: Vec<&str> = paths_str.lines().collect();

    let mut result = String::new();

    for item in collection.iter() {
        let (is_error, value) = get_file_diff(&item.to_string());

        if is_error {
            break;
        }

        result.push_str(item);
        result.push_str(&format!("\n{}\n", item));
    }

    let mut prompt = String::from(
        "Given the following git diffs, generate concise, present-tense commit messages in a single line. \
If multiple distinct changes are detected, provide separate commit messages for each. Use the conventional \
git style, summarizing what the changes do (e.g., 'Add', 'Update', 'Fix', 'Remove'). Avoid past tense, \
and aim for clarity in a single line per commit. Return the results formatted in HTML.\n\nGit Diffs:\n"
    );

    prompt.push_str(&result);

    let output = AsyncCommand::new("ollama")
        .arg("run")
        .arg("llama2")
        .arg(prompt)
        .output()
        .await?;

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Path error occurred",
        )));
    }

    Ok(String::from_utf8(output.stdout).expect("Error"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, get_commit_suggestion])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
