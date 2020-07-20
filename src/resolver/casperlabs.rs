use std::collections::BTreeSet;
use num_traits::ToPrimitive;
use crate::resolver::{Contract, FunctionDecl, Type,
    cfg::{ControlFlowGraph, Instr, Variable},
    expression::Expression
};

const MSG_SENDER: &str = "msg_sender";
const GET_CALLER: &str = "runtime::get_caller()";

pub struct CasperlabsContract<'a> {
    pub contract: &'a Contract
}

impl<'a> CasperlabsContract<'a> {
    pub fn new(contract: &'a Contract) -> Self {
        CasperlabsContract { contract }
    }

    // Api for Solang's Contract.

    pub fn functions(&self) -> Vec<&FunctionDecl> {
        self.contract.functions.iter()
            .filter(|f| !is_blacklisted_fn(&f.signature.to_string()))
            .collect()
    }

    pub fn variable_name(&self, id: usize) -> String {
        let name = self.contract.variables.get(id).unwrap().name.clone();
        if name == MSG_SENDER {
            GET_CALLER.to_string()
        } else {
            format!("\"{}\"", name)
        }
    }

    // Render functions

    pub fn render(&self) -> String {
        let mut result = Vec::<String>::new();
        result.push(self.render_header());
        result.push(self.render_functions());
        result.push(self.render_footer());
        result.join("\n")
    }

    fn render_header(&self) -> String {
        format!("
{imports}

#[casperlabs_contract]
mod {name} {{
            ", 
            name = self.contract.name,
            imports = self.render_imports()
        )
    }

    fn render_imports(&self) -> String {
        format!("
use core::convert::TryInto;
use alloc::{{collections::BTreeSet, string::String}};

use contract_macro::{{casperlabs_constructor, casperlabs_contract, casperlabs_method}};
use casperlabs_contract::{{
    contract_api::{{runtime, storage}},
    unwrap_or_revert::UnwrapOrRevert,
}};
use casperlabs_types::{{
    runtime_args, CLValue, CLTyped, CLType, Group, Parameter, RuntimeArgs, URef, U256,
    bytesrepr::{{ToBytes, FromBytes}}, account::AccountHash,
    contracts::{{EntryPoint, EntryPointAccess, EntryPointType, EntryPoints}},
}};
        ")
    }

    fn render_footer(&self) -> String {
        format!("
}}

fn get_key<T: FromBytes + CLTyped>(name: &str) -> T {{
    let key = runtime::get_key(name)
        .unwrap_or_revert()
        .try_into()
        .unwrap_or_revert();
    storage::read(key)
        .unwrap_or_revert()
        .unwrap_or_revert()
}}

fn set_key<T: ToBytes + CLTyped>(name: &str, value: T) {{
    match runtime::get_key(name) {{
        Some(key) => {{
            let key_ref = key.try_into().unwrap_or_revert();
            storage::write(key_ref, value);
        }}
        None => {{
            let key = storage::new_uref(value).into();
            runtime::put_key(name, key);
        }}
    }}
}}

fn new_key(a: &str, b: AccountHash) -> String {{
    format!(\"{{}}_{{}}\", a, b)
}}

fn ret<T: CLTyped + ToBytes>(value: T) {{
    runtime::ret(CLValue::from_t(value).unwrap_or_revert())
}}
        ")
    }

    fn render_functions(&self) -> String {
        // for f in self.contract.functions.iter() {
        //     println!("Function: {:?} {:?}", f.name, f.ast_index);
        // }
        let mut result = Vec::<String>::new(); 
        for function in &self.functions() {
            result.push(self.render_function(&function));
        }
        result.join("\n")
    }

    fn render_function(&self, function: &FunctionDecl) -> String {
        format!("
    {attr}
    fn {name}({args}) {{ {body}
    }}",
            attr = self.render_function_macro_name(&function),
            name = self.render_function_name(&function),
            args = self.render_function_args(&function),
            body = self.render_function_body(&function)
        )
    }

    fn render_function_macro_name(&self, function: &FunctionDecl) -> String {
        match (function.is_constructor(), function.is_public()) {
            (true, true) => "#[casperlabs_constructor]",
            (false, true) => "#[casperlabs_method]",
            _ => ""
        }.to_string()
    }

    fn render_function_name(&self, function: &FunctionDecl) -> String {
        if function.is_constructor() {
            "constructor".to_string()
        } else {
            function.name.clone()
        }
    }

    fn render_function_args(&self, function: &FunctionDecl) -> String {
        let mut result = Vec::<String>::new();
        for param in &function.params {
            result.push(format!(
                "{}: {}", param.name, self.render_type(&param.ty)));
        }
        result.join(", ")
    }

    fn render_function_body(&self, function: &FunctionDecl) -> String {
        self.render_function_cfg(&function.cfg.as_ref().unwrap())
    }

    fn render_function_cfg(&self, cfg: &ControlFlowGraph) -> String {
        let mut result = Vec::<String>::new();
        let block = cfg.bb.first().unwrap();
        // for var in &cfg.vars {
        //     println!("Var: {}, {}", var.id.name, self.render_type(&var.ty));
        // }
        // println!("Vars Done");
        for instruction in &block.instr {
            match self.render_instruction(&instruction, &cfg.vars) {
                Some(i) => result.push(format!{"
        {}", i}),
                None => {}
            }
        }
        result.join("")
    }

    fn render_instruction(&self, instruction: &Instr, vars: &Vec<Variable>) -> Option<String> {
        match instruction {
            Instr::Eval { expr: _ } => { None },
            Instr::Return { value } => {
                if value.is_empty() { None } else {
                    let expression = self.render_expression(
                        &value.first().unwrap(), &vars);
                    Some(format!("ret({});", expression))
                }
            },
            Instr::SetStorage { ty: _, local, storage } => {
                Some(format!(
                    "set_key({}, {});",
                    self.render_var_name_or_default(&storage, &vars),
                    self.render_local_var(*local, vars)
                ))
            },
            Instr::Set { res, expr } => {
                let left = self.render_local_var(*res, vars);
                let right = self.render_expression(&expr, &vars);
                if left == right { return None };
                Some(format!(
                    "let {}: {} = {};",
                    left,
                    self.render_type(&vars[*res].ty),
                    right
                ))
            },
            Instr::Call { res: _, func, args } => {
                let fn_name = self.contract.functions.get(*func).unwrap().name.clone();
                let mut result = Vec::<String>::new();
                for arg in args {
                    result.push(self.render_expression(arg, vars));
                }
                Some(format!(
                    "{}({});",
                    fn_name,
                    result.join(", ")
                ))
            },
            _ => {
                Some(format!("unknown_instruction"))
            }
        }
    }

    fn render_expression(&self, expression: &Expression, vars: &Vec<Variable>) -> String {
        match expression {
            Expression::BoolLiteral(_, false) => "false".to_string(),
            Expression::BoolLiteral(_, true) => "true".to_string(),
            Expression::StorageLoad(_, ty, expr) => {
                match self.render_var_name_or_default(expr, vars).as_str() {
                    GET_CALLER => GET_CALLER.to_string(),
                    result => format!(
                        "get_key::<{}>({})",
                        self.render_type(ty),
                        result
                    )
                }
            },
            Expression::FunctionArg(_, pos) => self.render_local_var(*pos, vars),
            Expression::Variable(_, res) => self.render_local_var(*res, vars),
            Expression::NumberLiteral(_, _bits, n) => format!("{}", n.to_str_radix(10)),
            Expression::Add(_, l, r) => format!(
                "({} + {})",
                self.render_expression(&l, &vars),
                self.render_expression(&r, vars)
            ),
            Expression::Subtract(_, l, r) => format!(
                "({} - {})",
                self.render_expression(&l, &vars),
                self.render_expression(&r, vars)
            ),
            Expression::Keccak256(_, exprs) => {
                let first = &exprs.get(0).unwrap().0;
                let second = &exprs.get(1).unwrap().0;
                format!(
                    "&new_key({}, {})",
                    self.render_var_name_or_default(first, vars),
                    self.render_expression(second, vars)
                )
            },
            _ => {
                println!("{:?}", expression);
                format!("unknown_expression")
            }
        }
    }

    fn render_var_name_or_default(&self, expression: &Expression, vars: &Vec<Variable>) -> String {
        match expression {
            Expression::NumberLiteral(_, _bits, n) => {
                let position: usize = n.to_usize().unwrap();
                self.variable_name(position)
            }
            _ => self.render_expression(expression, vars)
        }
    }

    fn render_local_var(&self, id: usize, vars: &Vec<Variable>) -> String {
        vars[id].id.name.replace(".", "").clone()
    }

    fn render_type(&self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool",
            Type::String => "String",
            Type::Uint(8) => "u8",
            Type::Uint(256) => "U256",
            Type::Address(false) => "AccountHash",
            _ => {
                "unknown_type"
            }
        }.to_string()
    }
}



fn is_blacklisted_fn(name: &str) -> bool {
    let mut fns = BTreeSet::new();
    fns.insert("print(string)");
    fns.insert("revert(string)");
    fns.insert("assert(bool)");
    fns.insert("revert()");
    fns.insert("require(bool,string)");
    fns.insert("require(bool)");
    fns.insert("selfdestruct(address)");
    fns.insert("keccak256(bytes)");
    fns.insert("ripemd160(bytes)");
    fns.insert("sha256(bytes)");
    fns.insert("blake2_128(bytes)");
    fns.insert("blake2_256(bytes)");
    fns.contains(name)
}

