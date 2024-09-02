use anyhow::Result;
use serde_yaml::Value;
use std::path::PathBuf;

#[derive(Debug)]
pub struct SamEnvConfig {
    pub template_path: PathBuf,
    pub package_name: String,
    pub output_path: String,
    pub output_filename: String,
    pub struct_name: String,
}

pub fn write_sam_env(config: SamEnvConfig) -> Result<()> {
    let template = std::fs::read(&config.template_path).map_err(|err| {
        anyhow::anyhow!(
            "Failed to read template file {:?}: {}",
            &config.template_path,
            err
        )
    })?;
    let template: Value = serde_yaml::from_slice(&template)
        .map_err(|err| anyhow::anyhow!("Failed to parse yaml template file: {}", err))?;

    let resources = template.get("Resources");
    let resources = match resources {
        None => {
            return Err(anyhow::anyhow!("Malformed yaml"));
        }
        Some(r) => r
            .as_mapping()
            .expect("Resources should be a mapping")
            .into_iter()
            .filter(|(_, value)| {
                value
                    .get("Properties")
                    .and_then(|p| p.get("CodeUri"))
                    .and_then(|c| c.as_str())
                    .map(|c| c.ends_with(&format!("/{}", config.package_name)))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v)
            .collect::<Vec<_>>(),
    };

    if resources.len() != 1 {
        return Err(anyhow::anyhow!(
            "Expect to find exactly one resource with CodeUri ends with {}. Found: {}",
            config.package_name,
            resources.len()
        ));
    }

    let env_properties = resources[0]
        .get("Properties")
        .and_then(|p| p.get("Environment"))
        .and_then(|e| e.get("Variables"))
        .and_then(|v| v.as_mapping())
        .map(|v| {
            v.keys()
                .map(|k| {
                    k.as_str()
                        .expect("Environment Variable key should be a string")
                })
                .collect::<Vec<_>>()
        });

    eprintln!("{:#?}", env_properties);

    let props = env_properties
        .map(|v| {
            let v: Vec<_> = v
                .into_iter()
                .map(|k| {
                    format!(
                        r##"
#[envconfig(from = "{}")]
{}: String,"##,
                        k,
                        k.to_lowercase()
                    )
                })
                .collect();
            v.join("\n")
        })
        .unwrap_or("".to_string());

    let output = format!(
        r#"
use envconfig::Envconfig;

#[derive(Debug, Envconfig)]
pub struct {} {{
    {}
}}"#,
        config.struct_name, props
    );

    eprintln!("{}", output);

    let out_dir = config.output_path;
    let dest_path = std::path::Path::new(&out_dir).join(config.output_filename);
    std::fs::write(&dest_path, output)?;

    Ok(())
}
