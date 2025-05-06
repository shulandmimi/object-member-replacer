use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};

use anyhow::Result;
use glob::glob;

use omm_transformer::TransformOption;
use rustc_hash::FxHashSet;

struct FixtureConfig {
    file: PathBuf,
    cwd: PathBuf,
}

fn fixtures<F: Fn(FixtureConfig) -> Result<()>>(pattern: &str, callback: F) -> Result<()> {
    for entry in glob(pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                callback(FixtureConfig {
                    file: path.clone(),
                    cwd: path.parent().unwrap().to_path_buf(),
                })?;
                println!("{:?}", path.display())
            }
            Err(e) => println!("{:?}", e),
        }
    }

    Ok(())
}

fn try_read_config_files(cwd: PathBuf) -> Result<FxHashSet<PathBuf>> {
    let mut result = FxHashSet::default();
    for entry in read_dir(&cwd)? {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        if !path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_string().starts_with("config"))
        {
            continue;
        }

        result.insert(path);
    }

    result.insert(cwd.join("config.json"));

    Ok(result)
}

fn try_merge_config(file: PathBuf) -> Option<TransformOption> {
    let result = read_to_string(file).ok()?;

    serde_json::from_str::<TransformOption>(&result).ok()
}

fn is_update_snapshot() -> bool {
    std::env::var("UPDATE_SNAPSHOT").is_ok()
}

fn format_output_name(file_path: PathBuf) -> PathBuf {
    let mut file_path = file_path;
    file_path.set_extension("md");
    if let Some(name) = file_path.file_name() {
        let name = name.to_string_lossy().to_string();
        if name.starts_with("config") {
            file_path.set_file_name(name.replace("config", "output"));
        }
    }
    file_path
}

fn test_factory(config: FixtureConfig) -> Result<()> {
    let content = read_to_string(&config.file)?;
    let files = try_read_config_files(config.cwd)?;

    for file in files {
        let config = try_merge_config(file.clone()).unwrap_or_default();

        let output = omm_transformer::transform(
            content.clone(),
            TransformOption {
                filename: Some(file.to_string_lossy().to_string()),
                ..config.clone()
            },
        )?;

        let output_file = format_output_name(file);
        let output_content = format!(
            r#"
## Config

```json
{}
```

## Output

```js
{}
```
        "#,
            serde_json::to_string_pretty(&config).unwrap(),
            output.content.trim(),
        )
        .trim()
        .to_string();

        if !output_file.exists() || is_update_snapshot() {
            std::fs::write(&output_file, &output_content)?;
        } else {
            let expected = read_to_string(&output_file)?;

            if expected != output_content {
                println!("Expected: {:?}", expected);
                println!("Output: {:?}", output_content);
                panic!("Snapshot mismatch");
            }
        }
    }

    Ok(())
}

fn fixtures_factor(pattern: &str) -> Result<()> {
    fixtures(pattern, |config| {
        println!("start testing: {:?}", config.file);

        test_factory(config)?;

        Ok(())
    })
}

#[test]
fn options() -> Result<()> {
    fixtures_factor("tests/fixtures/options/**/*.js")
}

#[test]
fn examples() -> Result<()> {
    fixtures_factor("tests/fixtures/examples/**/*.js")
}