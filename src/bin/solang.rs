extern crate clap;
extern crate serde;
extern crate solang;

use clap::{App, Arg, ArgMatches};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use solang::abi;
use solang::output;
use solang::resolver::casperlabs;

#[derive(Serialize)]
pub struct EwasmContract {
    pub wasm: String,
}

#[derive(Serialize)]
pub struct JsonContract {
    abi: Vec<abi::ethereum::ABI>,
    ewasm: EwasmContract,
}

#[derive(Serialize)]
pub struct JsonResult {
    pub errors: Vec<output::OutputJson>,
    pub contracts: HashMap<String, HashMap<String, JsonContract>>,
}

fn main() {
    let matches = App::new("solang")
        .version(&*format!("version {}", env!("GIT_HASH")))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("INPUT")
                .help("Solidity input files")
                .required(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("EMIT")
                .help("Emit compiler state at early stage")
                .long("emit")
                .takes_value(true)
                .default_value("casperlabs")
                .possible_values(&["cfg", "llvm", "bc", "object", "casperlabs"]),
        )
        .arg(
            Arg::with_name("OPT")
                .help("Set optimizer level")
                .short("O")
                .takes_value(true)
                .possible_values(&["none", "less", "default", "aggressive"])
                .default_value("default"),
        )
        .arg(
            Arg::with_name("TARGET")
                .help("Target to build for")
                .long("target")
                .takes_value(true)
                .possible_values(&["substrate", "ewasm", "sabre"])
                .default_value("substrate"),
        )
        .arg(
            Arg::with_name("STD-JSON")
                .help("mimic solidity json output on stdout")
                .long("standard-json"),
        )
        .arg(
            Arg::with_name("VERBOSE")
                .help("show verbose messages")
                .short("v")
                .long("verbose"),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("output directory")
                .short("o")
                .long("output")
                .takes_value(true),
        )
        .get_matches();

    let mut json = JsonResult {
        errors: Vec::new(),
        contracts: HashMap::new(),
    };

    let target = match matches.value_of("TARGET") {
        Some("substrate") => solang::Target::Substrate,
        Some("ewasm") => solang::Target::Ewasm,
        Some("sabre") => solang::Target::Sabre,
        _ => unreachable!(),
    };

    if matches.is_present("VERBOSE") {
        eprintln!("info: Solang version {}", env!("GIT_HASH"));
    }

    for filename in matches.values_of("INPUT").unwrap() {
        process_filename(filename, target, &matches, &mut json);
    }

    if matches.is_present("STD-JSON") {
        println!("{}", serde_json::to_string(&json).unwrap());
    }
}

fn process_filename(
    filename: &str,
    target: solang::Target,
    matches: &ArgMatches,
    json: &mut JsonResult,
) {
    let output_file = |stem: &str, ext: &str| -> PathBuf {
        Path::new(matches.value_of("OUTPUT").unwrap_or(".")).join(format!("{}.{}", stem, ext))
    };
    let verbose = matches.is_present("VERBOSE");
    let opt = match matches.value_of("OPT").unwrap() {
        "none" => inkwell::OptimizationLevel::None,
        "less" => inkwell::OptimizationLevel::Less,
        "default" => inkwell::OptimizationLevel::Default,
        "aggressive" => inkwell::OptimizationLevel::Aggressive,
        _ => unreachable!(),
    };
    let context = inkwell::context::Context::create();

    let mut json_contracts = HashMap::new();

    let mut f = match File::open(&filename) {
        Err(err_info) => {
            eprintln!(
                "error: cannot open {:?}: {}",
                &filename,
                err_info.to_string()
            );
            std::process::exit(1);
        }
        Ok(file) => file,
    };

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    // resolve phase
    let (ns, errors) = solang::parse_and_resolve(&contents, target);

    if matches.is_present("STD-JSON") {
        let mut out = output::message_as_json(filename, &contents, &errors);
        json.errors.append(&mut out);
    } else {
        output::print_messages(filename, &contents, &errors, verbose);
    }

    let ns = match ns {
        Some(ns) => ns,
        None => std::process::exit(1),
    };

    if ns.contracts.is_empty() {
        eprintln!("{}: error: no contracts found", filename);
        std::process::exit(1);
    }

    // emit phase
    for (contract_no, resolved_contract) in ns.contracts.iter().enumerate() {
        if let Some("cfg") = matches.value_of("EMIT") {
            println!("{}", resolved_contract.print_to_string(&ns));
            continue;
        }

        if let Some("casperlabs") = matches.value_of("EMIT") {
            let contract = casperlabs::CasperlabsContract::new(&resolved_contract);
            println!("{}", contract.render());
            continue;
        }

        if verbose {
            eprintln!(
                "info: Generating LLVM IR for contract {} with target {}",
                resolved_contract.name, ns.target
            );
        }

        let contract = resolved_contract.emit(&ns, &context, &filename, opt);

        if let Some("llvm") = matches.value_of("EMIT") {
            if let Some(runtime) = &contract.runtime {
                // In Ethereum, an ewasm contract has two parts, deployer and runtime. The deployer code returns the runtime wasm
                // as a byte string
                let llvm_filename = output_file(&format!("{}_deploy", contract.name), "ll");

                if verbose {
                    eprintln!(
                        "info: Saving deployer LLVM {} for contract {}",
                        llvm_filename.display(),
                        contract.name
                    );
                }

                contract.dump_llvm(&llvm_filename).unwrap();

                let llvm_filename = output_file(&format!("{}_runtime", contract.name), "ll");

                if verbose {
                    eprintln!(
                        "info: Saving runtime LLVM {} for contract {}",
                        llvm_filename.display(),
                        contract.name
                    );
                }

                runtime.dump_llvm(&llvm_filename).unwrap();
            } else {
                let llvm_filename = output_file(&contract.name, "ll");

                if verbose {
                    eprintln!(
                        "info: Saving LLVM {} for contract {}",
                        llvm_filename.display(),
                        contract.name
                    );
                }

                contract.dump_llvm(&llvm_filename).unwrap();
            }
            continue;
        }

        if let Some("bc") = matches.value_of("EMIT") {
            // In Ethereum, an ewasm contract has two parts, deployer and runtime. The deployer code returns the runtime wasm
            // as a byte string
            if let Some(runtime) = &contract.runtime {
                let bc_filename = output_file(&format!("{}_deploy", contract.name), "bc");

                if verbose {
                    eprintln!(
                        "info: Saving deploy LLVM BC {} for contract {}",
                        bc_filename.display(),
                        contract.name
                    );
                }

                contract.bitcode(&bc_filename);

                let bc_filename = output_file(&format!("{}_runtime", contract.name), "bc");

                if verbose {
                    eprintln!(
                        "info: Saving runtime LLVM BC {} for contract {}",
                        bc_filename.display(),
                        contract.name
                    );
                }

                runtime.bitcode(&bc_filename);
            } else {
                let bc_filename = output_file(&contract.name, "bc");

                if verbose {
                    eprintln!(
                        "info: Saving LLVM BC {} for contract {}",
                        bc_filename.display(),
                        contract.name
                    );
                }

                contract.bitcode(&bc_filename);
            }
            continue;
        }

        if let Some("object") = matches.value_of("EMIT") {
            let obj = match contract.wasm(false) {
                Ok(o) => o,
                Err(s) => {
                    println!("error: {}", s);
                    std::process::exit(1);
                }
            };

            let obj_filename = output_file(&contract.name, "o");

            if verbose {
                eprintln!(
                    "info: Saving Object {} for contract {}",
                    obj_filename.display(),
                    contract.name
                );
            }

            let mut file = File::create(obj_filename).unwrap();
            file.write_all(&obj).unwrap();
            continue;
        }

        let wasm = match contract.wasm(true) {
            Ok(o) => o,
            Err(s) => {
                println!("error: {}", s);
                std::process::exit(1);
            }
        };

        if matches.is_present("STD-JSON") {
            json_contracts.insert(
                contract.name.to_owned(),
                JsonContract {
                    abi: abi::ethereum::gen_abi(contract_no, &ns),
                    ewasm: EwasmContract {
                        wasm: hex::encode_upper(wasm),
                    },
                },
            );
        } else {
            let wasm_filename = output_file(&contract.name, "wasm");

            if verbose {
                eprintln!(
                    "info: Saving WebAssembly {} for contract {}",
                    wasm_filename.display(),
                    contract.name
                );
            }

            let mut file = File::create(wasm_filename).unwrap();
            file.write_all(&wasm).unwrap();

            let (abi_bytes, abi_ext) = ns.abi(contract_no, verbose);
            let abi_filename = output_file(&contract.name, abi_ext);

            if verbose {
                eprintln!(
                    "info: Saving ABI {} for contract {}",
                    abi_filename.display(),
                    contract.name
                );
            }

            file = File::create(abi_filename).unwrap();
            file.write_all(&abi_bytes.as_bytes()).unwrap();
        }
    }

    json.contracts.insert(filename.to_owned(), json_contracts);
}
