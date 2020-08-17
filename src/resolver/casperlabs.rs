use std::collections::BTreeSet;
use num_traits::ToPrimitive;
use num_bigint::BigInt;
use crate::resolver::{Contract, FunctionDecl, Type,
    cfg::{ControlFlowGraph, Instr, Variable, BasicBlock},
    expression::Expression
};

const MSG_SENDER: &str = "msg_sender";
const GET_CALLER: &str = "runtime::get_caller()";

pub struct CasperlabsContract<'a> {
    pub contract: &'a Contract,
    pub visited: u32
}

impl<'a> CasperlabsContract<'a> {
    pub fn new(contract: &'a Contract) -> Self {
        CasperlabsContract { contract, visited: 0u32 }
    }

    // Api for Solang's Contract.

    pub fn functions(&self) -> Vec<&FunctionDecl> {
        self.contract.functions.iter()
            .filter(|f| !is_blacklisted_fn(&f.signature.to_string()))
            .collect()
    }

    pub fn variable_name(&self, id: usize) -> Option<String> {
        println!("// var: {:?}", id);
        // println!("// all vars: {:?}", self.contract.variables);
        match self.contract.variables.get(id) {
            Some(variable) => {
                let name = variable.name.clone();
                if name == MSG_SENDER {
                    Some(GET_CALLER.to_string())
                } else {
                    Some(format!("\"{}\"", name))
                }
            },
            None => None
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
                runtime_args, CLValue, CLTyped, CLType, Group, Parameter, RuntimeArgs, URef, U256, ApiError,
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

            fn assert(condition: bool) {{
                if !condition {{
                    runtime::revert(ApiError::User(1u16));
                }}
            }}

            fn revert() {{
                assert(false);
            }}

            fn require(condition: bool) {{
                assert(condition);
            }}
        ")
    }

    fn render_functions(&self) -> String {
        let mut result = Vec::<String>::new(); 
        for function in self.functions() {
            result.push(self.render_function(function));
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
        // println!("// Got blocks count: {}", cfg.bb.len());
        // for var in &cfg.vars {
        //     println!("// Var: {}, {}", var.id.name, self.render_type(&var.ty));
        // }
        // println!("// Vars Done");
        self.render_block(0, cfg, Vec::new())
    }

    fn render_block(
        &self, 
        block_id: usize, 
        cfg: &ControlFlowGraph, 
        mut visited_bbs: Vec<usize>
    ) -> String {
        visited_bbs.push(block_id);
        let block = cfg.bb.get(block_id).unwrap();
        let mut result = Vec::<String>::new();
        for instruction in &block.instr {
            match self.render_instruction(&instruction, &cfg, visited_bbs.clone()) {
                Some(i) => result.push(i),
                None => {}
            }
        }
        result.join("")
    }

    fn render_instruction(
        &self, 
        instruction: &Instr, 
        cfg: &ControlFlowGraph, 
        visited_bbs: Vec<usize>
    ) -> Option<String> {
        match instruction {
            Instr::Eval { expr } => { 
                // println!("expression: {}", self.render_expression(&expr, vars));
                // Some(self.render_expression(&expr, cfg))
                None
            },
            Instr::Return { value } => {
                if value.is_empty() { None } else {
                    let expression = self.render_expression(
                        &value.first().unwrap(), cfg);
                    Some(format!("ret({});", expression))
                }
            },
            Instr::SetStorage { ty: _, local, storage } => {
                Some(format!(
                    "set_key({}, {});",
                    self.render_var_name_or_default(&storage, cfg),
                    self.render_local_var(*local, cfg)
                ))
            },
            Instr::Set { res, expr } => {
                let left = self.render_local_var(*res, cfg);
                let right = self.render_expression(&expr, cfg);
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
                    result.push(self.render_expression(arg, cfg));
                }
                Some(format!(
                    "{}({});",
                    fn_name,
                    result.join(", ")
                ))
            },
            Instr::BranchCond { cond, true_, false_} => {
                let true_bb = cfg.bb.get(*true_).unwrap();
                let false_bb = cfg.bb.get(*false_).unwrap();
                let cond = self.render_expression(&cond, cfg);
                println!("// false {:?}", false_bb);
                println!("// true {:?}", true_bb);

                match false_bb.name.as_str() {
                    "endwhile" => {
                        Some(format!(
                            "while {} {{ {} }} {}",
                            cond,
                            self.render_block(*true_, cfg, visited_bbs.clone()),
                            self.render_block(*false_, cfg, visited_bbs.clone())
                        ))
                    },
                    "endfor" => {
                        Some(format!(
                            "while {} {{ {} }} {}",
                            cond,
                            self.render_block(*true_, cfg, visited_bbs.clone()),
                            self.render_block(*false_, cfg, visited_bbs.clone())
                        ))
                    },
                    "endif" | "else" => {
                        let else_stm = match self.render_block(*false_, cfg, visited_bbs.clone()) {
                            code if code.len() == 0 => code,
                            code => format!("else {{ {} }}", code)
                        };
                        Some(format!(
                            "if {} {{ {} }} {}",
                            cond,
                            self.render_block(*true_, cfg, visited_bbs.clone()),
                            else_stm
                        ))
                    }
                    _ => None
                }

            },
            Instr::Branch { bb} => {
                println!("// branch {:?} {:?}", bb, cfg.bb.get(*bb));
                if visited_bbs.contains(bb) {
                    None
                } else {
                    Some(self.render_block(*bb, cfg, visited_bbs))
                }
                // None
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
                // panic!("Unhandled Instr::Store");
                Some(format!("{} = {}",
                    self.render_var_name_or_default(dest, cfg),
                    self.render_local_var(*pos, cfg)
                ))
            },
            Instr::AssertFailure { expr} =>
                self.render_instruction(&Instr::Unreachable, &cfg, visited_bbs),
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
                Some(format!("assert(false);"))
            },
            Instr::SelfDestruct { recipient} => {
                panic!("Unhandled Instr::SelfDestruct");
            },
            Instr::Hash { res, hash, expr} => {
                panic!("Unhandled Instr::Hash");
            }
        }
    }

    fn render_expression(&self, expression: &Expression, cfg: &ControlFlowGraph) -> String {
        match expression {
            // Literals
            Expression::FunctionArg(_, pos) => self.render_local_var(*pos, cfg),
            Expression::BoolLiteral(_, false) => "false".to_string(),
            Expression::BoolLiteral(_, true) => "true".to_string(),
            Expression::BytesLiteral(_, s) => format!("vec!{:?}", s),
            Expression::NumberLiteral(_, _bits, n) => format!("{}", n.to_str_radix(10)),
            // Expression::StructLiteral(_, _, expr) =>
            // Expression::ConstArrayLiteral(_, dims, exprs) =>
            Expression::ArrayLiteral(_, _, dims, exprs) => 
                self.render_static_array(dims, exprs, cfg),

                // Arithmetic
            Expression::Add(_, l, r) => format!(
                "({} + {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::Subtract(_, l, r) => format!(
                "({} - {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::BitwiseOr(_, l, r) => format!(
                "({}.iter().zip({}.iter()).map(|e| e.0 | e.1).collect::<Vec<u8>>())",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::BitwiseAnd(_, l, r) => format!(
                "({}.iter().zip({}.iter()).map(|e| e.0 & e.1).collect::<Vec<u8>>())",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::BitwiseXor(_, l, r) => format!(
                "({}.iter().zip({}.iter()).map(|e| e.0 ^ e.1).collect::<Vec<u8>>())",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            // Expression::ShiftLeft(_, l, r) => format!(
            // Expression::ShiftRight(_, l, r, _) => format!(
            Expression::Multiply(_, l, r) => format!(
                "({} * {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::UDivide(_, l, r) | Expression::SDivide(_, l, r) => format!(
                "({} / {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::UModulo(_, l, r) | Expression::SModulo(_, l, r) => format!(
                "({} % {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::Power(_, l, r) => format!(
                "{}.pow({})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),

            // Data
            Expression::Variable(_, res) => self.render_local_var(*res, cfg),
            // Expression::Load(_, expr) => {
            Expression::StorageLoad(_, ty, expr) => {
                // println!("// storage load {:?}", expr);
                match self.render_var_name_or_default(&expr, cfg).as_str() {
                    GET_CALLER => GET_CALLER.to_string(),
                    result => format!(
                        "get_key::<{}>({})",
                        self.render_type(ty),
                        result
                    )
                }
            },
            Expression::ZeroExt(_, ty, expr) =>
                self.render_expression(&expr, cfg),
            // Expression::SignExt(_, ty, e) => format!(
            // Expression::Trunc(_, ty, e) => format!(
            
            // Comparators 
            Expression::SMore(_, l, r) => format!(
                "({} > {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::SLess(_, l, r) => format!(
                "({} < {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::SMoreEqual(_, l, r) => format!(
                "({} >= {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::SLessEqual(_, l, r) => format!(
                "({} <= {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::UMore(_, l, r) => format!(
                "({} > {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::ULess(_, l, r) => format!(
                "({} < {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::UMoreEqual(_, l, r) => format!(
                "({} >= {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::ULessEqual(_, l, r) => format!(
                "({} <= {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::Equal(_, l, r) => format!(
                "({} == {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::NotEqual(_, l, r) => format!(
                "({} != {})",
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            
            // Arrays and Structs
            Expression::ArraySubscript(_, a, i) => format!(
                "{}[{} as usize]",
                self.render_expression(a, cfg),
                self.render_expression(i, cfg)
            ),
            // Expression::DynamicArraySubscript(_, a, _, i) => format!(
            // Expression::StorageBytesSubscript(_, a, i) => format!(
            // Expression::StorageBytesPush(_, a, i) => format!(
            // Expression::StorageBytesPop(_, a) => format!(
            // Expression::StorageBytesLength(_, a) => format!(
            // Expression::StructMember(_, a, f) => format!(

            // Bool operators
            // Expression::Or(_, l, r) => format!(
            //     "({} || {})", 
            //     self.render_expression(&l, cfg),
            //     self.render_expression(&r, cfg)
            // ),
            // Expression::And(_, l, r) => format!(
            //     "({} && {})", 
            //     self.render_expression(&l, cfg),
            //     self.render_expression(&r, cfg)
            // ),
            Expression::Ternary(_, c, l, r) => format!(
                "if {} {{ {} }} else {{ {} }}",
                self.render_expression(&c, cfg),
                self.render_expression(&l, cfg),
                self.render_expression(&r, cfg)
            ),
            Expression::Not(_, expr) => format!(
                "!({})", 
                self.render_expression(&expr, cfg)
            ),
            // Expression::Complement(_, e) => format!("~{}", self.expr_to_string(contract, ns, e)),
            Expression::UnaryMinus(_, expr) => format!(
                "-({})", 
                self.render_expression(&expr, cfg)
            ),

            // Others
            // Expression::Poison => "☠".to_string(),
            // Expression::Unreachable => "❌".to_string(),
            // Expression::AllocDynamicArray(_, ty, size, None) => format!(
            Expression::AllocDynamicArray(_, ty, size, Some(init)) => 
                format!(""),
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
                // println!("// expres: {:?}", exprs);
                match exprs.len() {
                    // 1 => {
                    //     let first = &exprs.get(0).unwrap().0;
                    //     match first {
                    //         Expression::NumberLiteral(_, bits, n) => {
                    //             let position: usize = n.to_usize().unwrap();
                    //             self.render_local_var(position, cfg)
                    //         }
                    //         _ => panic!("Unexpected keccak argument type")
                    //     }
                    // },
                    2 => {
                        let first = &exprs.get(0).unwrap().0;
                        let second = &exprs.get(1).unwrap().0;
                        format!(
                            "&new_key({}, {})",
                            self.render_var_name_or_default(first, cfg),
                            self.render_expression(second, cfg)
                        )
                    },
                    _ => panic!("Unsupportet number of keccak arguments")
                }
            },
            _ => {
                println!("// Unknown expression {:?}", expression);
                format!("unknown_expresson")
            }
        }
    }

    // fn render_if(&self, )

    fn render_static_array(&self,  dims: &Vec<u32>, exprs: &Vec<Expression>, cfg: &ControlFlowGraph) -> String {
        let mut result = Vec::new();
        for expr in exprs {
            result.push(self.render_expression(expr, cfg));
        }
        for dim in dims {
            let mut data = Vec::new();
            for elem in result.chunks(*dim as usize) {
                data.push(format!("[{}]", elem.join(", ")));
            }
            println!("// {:?}", data);
            result = data;
        }
        result.join(",")
    }

    fn render_var_name_or_default(&self, expression: &Expression, cfg: &ControlFlowGraph) -> String {
        match expression {
            Expression::NumberLiteral(_, _bits, n) => {
                let position: usize = n.to_usize().unwrap();
                match self.variable_name(position) {
                    Some(name) => name,
                    None => format!("\"{}\"", position)
                }
            },
            Expression::Add(_, _, _,) | Expression::Multiply(_, _, _) => {
                format!("&format!(\"{{}}\", {})", self.render_expression(expression, cfg))
            },
            _ => {
                self.render_expression(expression, cfg)
            }
        }
    }

    fn render_local_var(&self, id: usize, cfg: &ControlFlowGraph) -> String {
        cfg.vars[id].id.name.replace(".", "").clone()
    }

    fn render_type(&self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Uint(8) => "u8".to_string(),
            Type::Uint(16) => "u16".to_string(),
            Type::Uint(32) => "u32".to_string(),
            Type::Uint(64) => "u64".to_string(),
            Type::Uint(128) => "u128".to_string(),
            Type::Uint(256) => "U256".to_string(),
            Type::Int(8) => "i8".to_string(),
            Type::Int(16) => "i16".to_string(),
            Type::Int(32) => "i32".to_string(),
            Type::Int(64) => "i64".to_string(),
            Type::Int(128) => "i128".to_string(),
            Type::Address(false) => "AccountHash".to_string(),
            Type::Bytes(_) => "Vec<u8>".to_string(),
            Type::Array(inner_ty, dims) => 
                self.render_array_type(inner_ty, dims),
            Type::Ref(ty) => self.render_type(ty),
            Type::StorageRef(ty) => self.render_type(ty),
            _ => {
                print!("// unknow_type: {:?}", ty);
                "unknown_type".to_string()
            }
        }
    }

    fn render_array_type(&self, inner_ty: &Type, dims: &Vec<Option<BigInt>>) -> String {
        let mut result = self.render_type(inner_ty);
        for dim in dims.iter() {
            match dim {
                Some(dim) => {
                    result = format!("[{}; {}]", result, dim);
                },
                None => {
                    result = format!("Vec<{}>", result)
                }
            }
        }
        result
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
