use anyhow::Result;
use semver::Version;
use std::fs;
use std::path::PathBuf;

/// Find a wasm file path with the given package and morph name and optional version
/// in the format package:name@version
/// If version is not set, the latest version will be used
pub fn find_morph_path(registry_path: String, input: &str) -> Result<PathBuf> {
    match parse_identifier(input) {
        Some((package, name, version)) => {
            let mut path = PathBuf::new();
            path.push(registry_path);
            path.push(package);

            if let Some(version) = version {
                path.push(version);
            } else {
                // Check for latest version in this path by semver directory names
                let latest_version = fs::read_dir(&path)?
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                    .filter_map(|entry| {
                        entry.file_name().into_string().ok().and_then(|package| {
                            Version::parse(&package)
                                .ok()
                                .map(|version| (version, entry))
                        })
                    })
                    .max_by_key(|(version, _)| version.clone())
                    .map(|(_, entry)| entry)
                    .ok_or_else(|| anyhow::anyhow!("No versions found for package: {}", package))?;

                path.push(latest_version.file_name());
            }

            path.push(format!("{}.wasm", name));
            path = path.canonicalize()?;
            Ok(path)
        }
        None => Err(anyhow::anyhow!(
            "Invalid morph identifier: [{}] expected format: <package>:<name>@<version>",
            input
        )),
    }
}

fn parse_identifier(input: &str) -> Option<(&str, &str, Option<&str>)> {
    let (package, rest) = input.split_once(':')?;
    let (name, version) = rest
        .split_once('@')
        .map_or((rest, None), |(ns_name, ver)| (ns_name, Some(ver)));
    Some((package, name, version))
}
