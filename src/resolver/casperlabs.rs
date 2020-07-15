use std::collections::BTreeSet;
use num_traits::ToPrimitive;
use crate::resolver::{Contract, FunctionDecl, Type, Parameter,
    cfg::{ControlFlowGraph, Instr, Variable},
    expression::Expression, Namespace, ContractVariable
};

const MSG_SENDER: &str = "msg_sender";
const GET_CALLER: &str = "runtime::get_caller().to_str()";

use crate::parser::pt::{FunctionTy};
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

    // Stated Render Functions

    pub fn render(&self) -> String {
        let mut result = Vec::<String>::new();
        result.push(self.render_header());
        result.push(self.render_functions());
        result.push(self.render_footer());
        result.join("\n")
    }

    fn render_header(&self) -> String {
        format!("
            #[casperlabs_contract]
            mod {name} {{", 
            name = self.contract.name
        )
    }

    fn render_footer(&self) -> String {
        format!("
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

    // No state Render Functions

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
        for var in &cfg.vars {
            println!("Var: {}, {}", var.id.name, self.render_type(&var.ty));
        }
        println!("Vars Done");
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
            Instr::Eval { expr } => {
                None
            },
            Instr::Return { value } => {
                if value.is_empty() { None } else {
                    let expression = self.render_expression(
                        &value.first().unwrap(), &vars);
                    Some(format!("runtime::ret({});", expression))
                }
            },
            Instr::SetStorage { ty, local, storage } => {
                // println!("instr: SetStorage: ty {:?}", );
                Some(format!(
                    "set_key({}, {});",
                    self.render_var_name_or_default(&storage, &vars),
                    vars[*local].id.name
                ))
            },
            Instr::Set { res, expr } => {
                let left = vars[*res].id.name.clone();
                let right = self.render_expression(&expr, &vars);
                if left == right { return None };
                // println!("instr: Set: {:?}", right);
                Some(format!(
                    "let {}: {} = {};",
                    left,
                    self.render_type(&vars[*res].ty),
                    right
                ))
            },
            _ => {
                // println!("{:?}", instruction);
                Some(format!("unknown_instruction"))
            }
        }
    }

    fn render_expression(&self, expression: &Expression, vars: &Vec<Variable>) -> String {
        match expression {
            Expression::StorageLoad(_, ty, expr) => {
                match self.render_var_name_or_default(expr, vars).as_str() {
                    GET_CALLER => GET_CALLER.to_string(),
                    result => format!(
                        "get_key::<{}>::({})",
                        self.render_type(ty),
                        result
                    )
                }


                // let result = self.render_var_name_or_default(expr, vars);
                // if result == GET_CALLER {
                //     result
                // } else {
                //     format!(
                //         "get_key::<{}>::({})",
                //         self.render_type(ty),
                //         result
                //     )
                // }
            },
            Expression::FunctionArg(_, pos) => vars[*pos].id.name.clone(),
            Expression::Variable(_, res) => format!("{}", vars[*res].id.name),
            Expression::NumberLiteral(_, bits, n) => format!("{}", n.to_str_radix(10)),
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
                    "new_key({}, {})",
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
            Expression::NumberLiteral(_, bits, n) => {
                let position: usize = n.to_usize().unwrap();
                self.variable_name(position)
            }
            _ => self.render_expression(expression, vars)
        }
    }

    fn render_type(&self, ty: &Type) -> String {
        match ty {
            Type::Bool => "bool",
            Type::String => "String",
            Type::Uint(8) => "u8",
            Type::Uint(256) => "U256",
            Type::Address(false) => "AccountKey",
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

