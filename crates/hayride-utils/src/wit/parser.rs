use anyhow::Result;

use wit_parser::decoding::DecodedWasm;

#[derive(Debug, Clone)]
pub struct WitParser {
    imports: Vec<wit_parser::Package>,
    function_exports: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub function: wit_parser::Function,
    pub interface: Option<wit_parser::Interface>,
}

impl WitParser {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        let decoded: DecodedWasm = wit_parser::decoding::decode(&bytes)?;
        let resolved: &wit_parser::Resolve = decoded.resolve();
        let imports = parse_imports(resolved)?;
        let functions = parse_function_exports(resolved)?;

        Ok(Self {
            imports: imports,
            function_exports: functions,
        })
    }

    pub fn imports(&self) -> &Vec<wit_parser::Package> {
        return &self.imports;
    }

    pub fn function_exports(&self) -> &Vec<Function> {
        return &self.function_exports;
    }
}

/// parse imports and return a list of packages
fn parse_imports(wit: &wit_parser::Resolve) -> Result<Vec<wit_parser::Package>> {
    let mut packages: Vec<wit_parser::Package> = Vec::new();

    wit.worlds.iter().for_each(|w| {
        // Look at what imports this component expects
        w.1.imports.iter().for_each(|e| {
            // println!("Export: {:?}", e);
            // If WorldItem is an interface lookup interface
            match e.1 {
                wit_parser::WorldItem::Interface { id, .. } => {
                    // Get the interface by id
                    let interface = wit.interfaces.get(*id);
                    match interface {
                        Some(i) => {
                            // Check which packages are expected
                            if let Some(id) = i.package {
                                if let Some(package) = wit.packages.get(id) {
                                    // println!("Package: {:?}", package);
                                    packages.push(package.clone());
                                }
                            }
                        }
                        None => {
                            println!("Skipping non-interface");
                        }
                    }
                }
                _ => {
                    println!("Not an Interface");
                }
            }
        });
    });

    return Ok(packages);
}

fn parse_function_exports(resolved: &wit_parser::Resolve) -> Result<Vec<Function>> {
    let mut functions: Vec<Function> = Vec::new();

    resolved.worlds.iter().for_each(|w| {
        // println!("World: {:?}", w);
        // First check all exports to see what type of component this is
        w.1.exports.iter().for_each(|e| {
            // println!("Export: {:?}", e);
            // If WorldItem is an interface lookup interface
            match e.1 {
                wit_parser::WorldItem::Interface { id, .. } => {
                    // Get the interface by id
                    let interface = resolved.interfaces.get(*id);
                    match interface {
                        Some(i) => {
                            // Print functions from the exported interface
                            i.functions.iter().for_each(|f| {
                                let f = Function {
                                    function: f.1.clone(),
                                    interface: Some(i.clone()),
                                };
                                functions.push(f);
                            });
                        }
                        None => {
                            println!("Interface Not Found");
                        }
                    }
                }
                wit_parser::WorldItem::Function(f) => {
                    let f = Function {
                        function: f.clone(),
                        interface: None,
                    };
                    functions.push(f);
                }
                _ => {
                    println!("Not an Interface");
                }
            }
        });
    });

    return Ok(functions);
}
