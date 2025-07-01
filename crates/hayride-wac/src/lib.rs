use anyhow::{anyhow, Context, Result};
use indexmap::IndexMap;
use miette::SourceSpan;
use std::path::Path;
use std::{collections::HashMap, fs, path::PathBuf};

// use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
use wac_parser::Document;
use wac_resolver::{packages, Error};
use wac_types::BorrowedPackageKey;

use hayride_host_traits::wac::{errors::ErrorCode, WacTrait};

#[derive(Clone)]
pub struct WacBackend {
    registry_path: String,
}

impl WacBackend {
    pub fn new(registry_path: String) -> Self {
        Self { registry_path }
    }
}

impl WacTrait for WacBackend {
    fn compose(&mut self, contents: String) -> Result<Vec<u8>, ErrorCode> {
        let mut registry_path = hayride_utils::paths::hayride::default_hayride_dir()
            .map_err(|_| ErrorCode::ComposeFailed)?;
        registry_path.push(self.registry_path.clone());

        let document = Document::parse(&contents).map_err(|e| {
            log::error!("Failed to parse wac compose contents: {}", e);
            ErrorCode::ComposeFailed
        })?;

        let mut resolver = PackageResolver::new(
            registry_path,  // deps
            HashMap::new(), // overrides
        )
        .map_err(|e| {
            log::error!("Failed to create package resolver: {}", e);
            ErrorCode::ComposeFailed
        })?;

        let packages = resolver.resolve(&document).map_err(|e| {
            log::error!("Failed to resolve packages: {}", e);
            ErrorCode::ResolveFailed
        })?;

        let resolution = document.resolve(packages).map_err(|e| {
            log::error!("Failed to resolve document: {}", e);
            ErrorCode::ResolveFailed
        })?;

        let bytes = resolution
            .encode(EncodeOptions {
                define_components: true,
                validate: true,
                ..Default::default()
            })
            .map_err(|e| {
                log::error!("Failed to encode component: {}", e);
                ErrorCode::EncodeFailed
            })?;

        return Ok(bytes);
    }

    fn plug(&mut self, socket_path: String, plug_paths: Vec<String>) -> Result<Vec<u8>, ErrorCode> {
        // Build registry path from home directory
        let mut registry_path = hayride_utils::paths::hayride::default_hayride_dir()
            .map_err(|_| ErrorCode::ComposeFailed)?;
        registry_path.push(self.registry_path.clone());
        let registry_path = registry_path
            .to_str()
            .ok_or_else(|| ErrorCode::ComposeFailed)?;

        let mut graph = CompositionGraph::new();

        // Register the plug dependencies into the graph
        let mut plug_packages = Vec::new();
        for plug_path in plug_paths {
            let plug_path = resolve_morph_path(registry_path, &plug_path)?;

            let name = Path::new(&plug_path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| ErrorCode::FileNotFound)?; // Convert OsStr to &str

            let package = Package::from_file(name, None, plug_path.clone(), graph.types_mut())
                .map_err(|e| {
                    log::error!("Failed to find plug: {}", e);
                    ErrorCode::FileNotFound
                })?;
            let plug = graph.register_package(package).unwrap();
            plug_packages.push(plug);
        }

        // Socket component
        let socket_path = resolve_morph_path(registry_path, &socket_path)?;

        let package =
            Package::from_file("socket", None, socket_path, graph.types_mut()).map_err(|e| {
                log::error!("Failed to find socket: {}", e);
                ErrorCode::FileNotFound
            })?;
        let socket = graph.register_package(package).map_err(|e| {
            log::error!("Failed to register socket: {}", e);
            ErrorCode::EncodeFailed
        })?;

        wac_graph::plug(&mut graph, plug_packages, socket).map_err(|e| {
            log::error!("Failed to plug packages: {}", e);
            ErrorCode::EncodeFailed
        })?;

        // Encode the graph into a WASM bytes
        let encoding = graph.encode(EncodeOptions::default()).map_err(|e| {
            log::error!("Failed to encode to bytes: {}", e);
            ErrorCode::EncodeFailed
        })?;
        return Ok(encoding);
    }
}

/// Used to resolve packages from the Hayride file system.
pub struct HayridePackageResolver {
    root: PathBuf,
    overrides: HashMap<String, PathBuf>,
    error_on_unknown: bool,
}

impl HayridePackageResolver {
    /// Creates a new file system resolver with the given root directory.
    pub fn new(
        root: impl Into<PathBuf>,
        overrides: HashMap<String, PathBuf>,
        error_on_unknown: bool,
    ) -> Self {
        Self {
            root: root.into(),
            overrides,
            error_on_unknown,
        }
    }

    /// Resolves the provided package keys to packages.
    pub fn resolve<'a>(
        &self,
        keys: &IndexMap<BorrowedPackageKey<'a>, SourceSpan>,
    ) -> Result<IndexMap<BorrowedPackageKey<'a>, Vec<u8>>, Error> {
        let mut packages = IndexMap::new();
        for (key, span) in keys.iter() {
            let path = match self.overrides.get(key.name) {
                Some(path) if key.version.is_none() => {
                    if !path.is_file() {
                        return Err(Error::PackageResolutionFailure {
                            name: key.name.to_string(),
                            span: *span,
                            source: anyhow!(
                                "local path `{path}` for package `{name}` does not exist",
                                path = path.display(),
                                name = key.name
                            ),
                        });
                    }

                    path.clone()
                }
                _ => {
                    let mut path = self.root.clone();
                    for segment in key.name.split(':') {
                        path.push(segment);
                    }

                    if let Some(version) = key.version {
                        path = path
                            .parent()
                            .map(|p| p.join(version.to_string()).join(path.file_name().unwrap()))
                            .unwrap();
                    }

                    // If the path is not a directory, use a `.wasm` or `.wat` extension
                    if !path.is_dir() {
                        append_extension(&mut path, "wasm");
                    }

                    path
                }
            };

            if !path.is_file() {
                log::debug!(
                    "package `{key}` does not exist at `{path}`",
                    path = path.display()
                );
                if self.error_on_unknown {
                    return Err(Error::UnknownPackage {
                        name: key.name.to_string(),
                        span: *span,
                    });
                }
                continue;
            }

            log::debug!(
                "loading package `{key}` from `{path}`",
                path = path.display()
            );
            let bytes = fs::read(&path)
                .with_context(|| format!("failed to read package `{path}`", path = path.display()))
                .map_err(|e| Error::PackageResolutionFailure {
                    name: key.name.to_string(),
                    span: *span,
                    source: e,
                })?;

            packages.insert(*key, bytes);
        }

        Ok(packages)
    }
}

/// Similar to Path::set_extension except it always appends.
/// For example "0.0.1" -> "0.0.1.wasm" (instead of to "0.0.wasm").
fn append_extension(path: &mut PathBuf, extension: &str) {
    let os_str = path.as_mut_os_string();
    os_str.push(".");
    os_str.push(extension)
}

/// Represents a package resolver used to resolve packages
/// referenced from a document.
///
/// The resolver first checks the file system for a matching package.
///
/// If it cannot find a matching package, it will check the registry.
pub struct PackageResolver {
    fs: HayridePackageResolver,
}

impl PackageResolver {
    /// Creates a new package resolver.
    pub fn new(dir: impl Into<PathBuf>, overrides: HashMap<String, PathBuf>) -> Result<Self> {
        Ok(Self {
            fs: HayridePackageResolver::new(dir, overrides, false),
        })
    }

    /// Resolve all packages referenced in the given document.
    pub fn resolve<'a>(
        &mut self,
        document: &'a Document<'a>,
    ) -> Result<IndexMap<BorrowedPackageKey<'a>, Vec<u8>>, Error> {
        let mut keys = packages(document)?;

        // Next, we resolve as many of the packages from the file system as possible
        // and filter out the ones that were resolved.
        #[allow(unused_mut)]
        let mut packages = self.fs.resolve(&keys)?;
        keys.retain(|key, _| !packages.contains_key(key));

        // At this point keys should be empty, otherwise we have an unknown package
        if let Some((key, span)) = keys.first() {
            return Err(Error::UnknownPackage {
                name: key.name.to_string(),
                span: *span,
            });
        }

        Ok(packages)
    }
}

fn resolve_morph_path(registry_path: &str, morph_path: &str) -> Result<PathBuf, ErrorCode> {
    // First, check if the morph path is a valid morph path
    let result = match hayride_utils::paths::registry::find_morph_path(
        registry_path.to_string(),
        &morph_path,
    ) {
        Ok(path) => path,
        Err(_) => {
            // If not a valid morph path, processes it as a regular file path returning PathBuf
            let path = Path::new(&morph_path);
            if !path.is_file() {
                return Err(ErrorCode::FileNotFound);
            }
            let path = path.canonicalize().map_err(|e| {
                log::error!("Failed to canonicalize plug path: {}", e);
                ErrorCode::FileNotFound
            })?;
            path
        }
    };

    Ok(result)
}
