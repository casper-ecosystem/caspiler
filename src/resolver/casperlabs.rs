use std::collections::BTreeSet;
use num_traits::ToPrimitive;
use crate::resolver::{Contract, FunctionDecl, Type,
    cfg::{ControlFlowGraph, Instr, Variable, BasicBlock},
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
            #![no_main]
            #![allow(unused_imports)]
            #![allow(unused_parens)]
            #![allow(non_snake_case)]

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
            extern crate alloc;

            use core::convert::TryInto;
            use alloc::{{collections::{{BTreeSet, BTreeMap}}, string::String}};

            use casperlabs_contract_macro::{{casperlabs_constructor, casperlabs_contract, casperlabs_method}};
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

            fn get_key<T: FromBytes + CLTyped + Default>(name: &str) -> T {{
                match runtime::get_key(name) {{
                    None => Default::default(),
                    Some(value) => {{
                        let key = value.try_into().unwrap_or_revert();
                        storage::read(key).unwrap_or_revert().unwrap_or_revert()
                    }}
                }}
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
        ")
    }

    fn render_functions(&self) -> String {
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
                "{}: {}", 
                param.name, 
                self.render_type(&param.ty)));
        }
        result.join(", ")
    }

    fn render_function_body(&self, function: &FunctionDecl) -> String {
        self.render_function_cfg(&function.cfg.as_ref().unwrap())
    }

    fn render_function_cfg(&self, cfg: &ControlFlowGraph) -> String {
        println!("// Got blocks count: {}", cfg.bb.len());
        for var in &cfg.vars {
            println!("// Var: {}, {}", var.id.name, self.render_type(&var.ty));
        }
        println!("// Vars Done");
        self.render_block(0, cfg)
    }

    fn render_block(&self, block_id: usize, cfg: &ControlFlowGraph) -> String {
        let block = cfg.bb.get(block_id).unwrap();
        let mut result = Vec::<String>::new();
        for instruction in &block.instr {
            match self.render_instruction(&instruction, &cfg) {
                Some(i) => result.push(format!{"{}", i}),
                None => {}
            }
        }
        result.join("")
    }

    fn render_instruction(&self, instruction: &Instr, cfg: &ControlFlowGraph) -> Option<String> {
        match instruction {
            Instr::Eval { expr } => { 
                // println!("expression: {}", self.render_expression(&expr, vars));
                // Some(self.render_expression(&expr, &cfg.vars))
                None
            },
            Instr::Return { value } => {
                if value.is_empty() { None } else {
                    let expression = self.render_expression(
                        &value.first().unwrap(), &cfg.vars);
                    Some(format!("ret({});", expression))
                }
            },
            Instr::SetStorage { ty: _, local, storage } => {
                Some(format!(
                    "set_key({}, {});",
                    self.render_var_name_or_default(&storage, &cfg.vars),
                    self.render_local_var(*local, &cfg.vars)
                ))
            },
            Instr::Set { res, expr } => {
                let left = self.render_local_var(*res, &cfg.vars);
                let right = self.render_expression(&expr, &cfg.vars);
                if left == right { 
                    return None 
                };
                Some(format!(
                    "let {}: {} = {};",
                    left,
                    self.render_type(&cfg.vars[*res].ty),
                    right
                ))
            },
            Instr::Call { res: _, func, args } => {
                let fn_name = self.contract.functions.get(*func).unwrap().name.clone();
                let mut result = Vec::<String>::new();
                for arg in args {
                    result.push(self.render_expression(arg, &cfg.vars));
                }
                Some(format!(
                    "{}({});",
                    fn_name,
                    result.join(", ")
                ))
            },
            Instr::BranchCond { cond, true_, false_} => {
                let else_stm = match self.render_block(*false_, cfg) {
                    code if code.len() == 0 => code,
                    code => format!("else {{ {} }}", code)
                };
                Some(format!(
                    "if {} {{ {} }} {}",
                    self.render_expression(&cond, &cfg.vars),
                    self.render_block(*true_, cfg),
                    else_stm
                ))
            },
            Instr::Branch { bb} => {
                Some(self.render_block(*bb, cfg))
            },
            Instr::ClearStorage { ty, storage} => {
                panic!("Unhandled Instr::ClearStorage");
            },
            Instr::SetStorageBytes { local, storage, offset} => {
                panic!("Unhandled Instr::SetStorageBytes");
            },
            Instr::Constant { res, constant} => {
                panic!("Unhandled Instr::Constant");
            },

            Instr::Store { dest, pos} => {
                panic!("Unhandled Instr::Store");
            },
            Instr::AssertFailure { expr} => {
                panic!("Unhandled Instr::AssertFaulure");
            },
            Instr::Print{ expr} => {
                panic!("Unhandled Instr::Print");
            },
            Instr::Constructor {
                success,
                res,
                contract_no,
                constructor_no,
                args,
                value,
                gas,
                salt
            } => {
                panic!("Unhandled Instr::Constructor");
            },
            Instr::ExternalCall {
                success, 
                address,
                contract_no,
                function_no,
                args,
                value,
                gas
            } => {
                panic!("Unhandled Instr::ExternalCall");
            },
            Instr::AbiDecode {
                res,
                selector,
                exception,
                tys,
                data
            } => {
                panic!("Unhandled Instr::AbiDecode");
            },
            Instr::Unreachable => {
                panic!("Unhandled Instr::Unreachable");
            },
            Instr::SelfDestruct { recipient} => {
                panic!("Unhandled Instr::SelfDestruct");
            },
            Instr::Hash { res, hash, expr} => {
                panic!("Unhandled Instr::Hash");
            }
        }
    }

    fn render_expression(&self, expression: &Expression, vars: &Vec<Variable>) -> String {
        match expression {
            // Literals
            Expression::FunctionArg(_, pos) => self.render_local_var(*pos, vars),
            Expression::BoolLiteral(_, false) => "false".to_string(),
            Expression::BoolLiteral(_, true) => "true".to_string(),
            // Expression::BytesLiteral(_, s) =>
            Expression::NumberLiteral(_, _bits, n) => format!("{}", n.to_str_radix(10)),
            // Expression::StructLiteral(_, _, expr) =>
            // Expression::ConstArrayLiteral(_, dims, exprs) =>
            // Expression::ArrayLiteral(_, _, dims, exprs) => 
            
            // Arithmetic
            Expression::Add(_, l, r) => format!(
                "({} + {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::Subtract(_, l, r) => format!(
                "({} - {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            // Expression::BitwiseOr(_, l, r) => format!(
            // Expression::BitwiseAnd(_, l, r) => format!(
            // Expression::BitwiseXor(_, l, r) => format!(
            // Expression::ShiftLeft(_, l, r) => format!(
            // Expression::ShiftRight(_, l, r, _) => format!(
            Expression::Multiply(_, l, r) => format!(
                "({} * {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::UDivide(_, l, r) | Expression::SDivide(_, l, r) => format!(
                "({} / {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::UModulo(_, l, r) | Expression::SModulo(_, l, r) => format!(
                "({} % {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::Power(_, l, r) => format!(
                "{}.pow({})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),

            // Data
            Expression::Variable(_, res) => self.render_local_var(*res, vars),
            // Expression::Load(_, expr) => {
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
            Expression::ZeroExt(_, ty, expr) =>
                self.render_expression(&expr, vars),
            // Expression::SignExt(_, ty, e) => format!(
            // Expression::Trunc(_, ty, e) => format!(
            
            // Comparators 
            Expression::SMore(_, l, r) => format!(
                "({} > {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::SLess(_, l, r) => format!(
                "({} < {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::SMoreEqual(_, l, r) => format!(
                "({} >= {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::SLessEqual(_, l, r) => format!(
                "({} <= {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::UMore(_, l, r) => format!(
                "({} > {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::ULess(_, l, r) => format!(
                "({} < {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::UMoreEqual(_, l, r) => format!(
                "({} >= {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::ULessEqual(_, l, r) => format!(
                "({} <= {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::Equal(_, l, r) => format!(
                "({} == {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::NotEqual(_, l, r) => format!(
                "({} != {})",
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            
            // Arrays and Structs
            // Expression::ArraySubscript(_, a, i) => format!(
            // Expression::DynamicArraySubscript(_, a, _, i) => format!(
            // Expression::StorageBytesSubscript(_, a, i) => format!(
            // Expression::StorageBytesPush(_, a, i) => format!(
            // Expression::StorageBytesPop(_, a) => format!(
            // Expression::StorageBytesLength(_, a) => format!(
            // Expression::StructMember(_, a, f) => format!(

            // Bool operators
            // Expression::Or(_, l, r) => format!(
            //     "({} || {})", 
            //     self.render_expression(&l, vars),
            //     self.render_expression(&r, vars)
            // ),
            // Expression::And(_, l, r) => format!(
            //     "({} && {})", 
            //     self.render_expression(&l, vars),
            //     self.render_expression(&r, vars)
            // ),
            Expression::Ternary(_, c, l, r) => format!(
                "if {} {{ {} }} else {{ {} }}",
                self.render_expression(&c, vars),
                self.render_expression(&l, vars),
                self.render_expression(&r, vars)
            ),
            Expression::Not(_, expr) => format!(
                "!({})", 
                self.render_expression(&expr, vars)
            ),
            // Expression::Complement(_, e) => format!("~{}", self.expr_to_string(contract, ns, e)),
            Expression::UnaryMinus(_, expr) => format!(
                "-{}", 
                self.render_expression(&expr, vars)
            ),

            // Others
            // Expression::Poison => "☠".to_string(),
            // Expression::Unreachable => "❌".to_string(),
            // Expression::AllocDynamicArray(_, ty, size, None) => format!(
            // Expression::AllocDynamicArray(_, ty, size, Some(init)) => format!(
            // Expression::DynamicArrayLength(_, a) => {
            // Expression::StringCompare(_, l, r) => format!(
            // Expression::StringConcat(_, l, r) => format!(
            // Expression::LocalFunctionCall(_, f, args) => format!(
            // Expression::Constructor {
            //     contract_no,
            //     constructor_no,
            //     args,
            //     ..
            // } =>
            // Expression::CodeLiteral(_, contract_no, runtime) => format!(
            // Expression::ExternalFunctionCall {
            //     function_no,
            //     contract_no,
            //     address,
            //     args,
            //     ..
            // } => format!(
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
                println!("// Unknown expression {:?}", expression);
                format!("unknown_expresson")
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
