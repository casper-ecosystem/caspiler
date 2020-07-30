// Got blocks count: 1
// Var: a, bool
// Var: r, bool
// Vars Done
// Got blocks count: 3
// Var: a, bool
// Var: b, bool
// Var: or.temp.2, bool
// Var: r, bool
// Vars Done
// Got blocks count: 3
// Var: a, bool
// Var: b, bool
// Var: and.temp.2, bool
// Var: r, bool
// Vars Done
// Got blocks count: 5
// Var: a, bool
// Var: b, bool
// Var: or.temp.2, bool
// Var: and.temp.3, bool
// Var: r, bool
// Vars Done
// Got blocks count: 3
// Var: a, bool
// Vars Done
// Got blocks count: 4
// Var: a, bool
// Var: r, bool
// Vars Done
// Got blocks count: 1
// Vars Done

#![no_main]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(non_snake_case)]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
};
use core::convert::TryInto;

use casperlabs_contract::{
    contract_api::{runtime, storage},
    unwrap_or_revert::UnwrapOrRevert,
};
use casperlabs_contract_macro::{casperlabs_constructor, casperlabs_contract, casperlabs_method};
use casperlabs_types::{
    account::AccountHash,
    bytesrepr::{FromBytes, ToBytes},
    contracts::{EntryPoint, EntryPointAccess, EntryPointType, EntryPoints},
    runtime_args, CLType, CLTyped, CLValue, Group, Parameter, RuntimeArgs, URef, U256,
};

#[casperlabs_contract]
mod Contract {

    #[casperlabs_method]
    fn not(a: bool) {
        let r: bool = !(a);
    }

    #[casperlabs_method]
    fn or(a: bool, b: bool) {
        let ortemp2: bool = true;
        if a {
            let r: bool = ortemp2;
        } else {
            let ortemp2: bool = b;
            let r: bool = ortemp2;
        }
    }

    #[casperlabs_method]
    fn and(a: bool, b: bool) {
        let andtemp2: bool = false;
        if a {
            let andtemp2: bool = b;
            let r: bool = andtemp2;
        } else {
            let r: bool = andtemp2;
        }
    }

    #[casperlabs_method]
    fn combined(a: bool, b: bool) {
        let ortemp2: bool = true;
        if true {
            let andtemp3: bool = false;
            if ortemp2 {
                let andtemp3: bool = !(b);
                let r: bool = !(andtemp3);
            } else {
                let r: bool = !(andtemp3);
            }
        } else {
            let ortemp2: bool = b;
            let andtemp3: bool = false;
            if ortemp2 {
                let andtemp3: bool = !(b);
                let r: bool = !(andtemp3);
            } else {
                let r: bool = !(andtemp3);
            }
        }
    }

    #[casperlabs_method]
    fn ifStm(a: bool) {
        if a {
            let a: bool = false;
        }
    }

    #[casperlabs_method]
    fn ifElseStm(a: bool) {
        if a {
            let r: bool = false;
        } else {
            let r: bool = true;
        }
    }

    #[casperlabs_constructor]
    fn constructor() {}
}

fn get_key<T: FromBytes + CLTyped + Default>(name: &str) -> T {
    match runtime::get_key(name) {
        None => Default::default(),
        Some(value) => {
            let key = value.try_into().unwrap_or_revert();
            storage::read(key).unwrap_or_revert().unwrap_or_revert()
        }
    }
}

fn set_key<T: ToBytes + CLTyped>(name: &str, value: T) {
    match runtime::get_key(name) {
        Some(key) => {
            let key_ref = key.try_into().unwrap_or_revert();
            storage::write(key_ref, value);
        }
        None => {
            let key = storage::new_uref(value).into();
            runtime::put_key(name, key);
        }
    }
}

fn new_key(a: &str, b: AccountHash) -> String {
    format!("{}_{}", a, b)
}
