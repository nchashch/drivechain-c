// TODO: Very carefully check that there are no memory management bugs.
use drivechain::*;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::str::FromStr;
use std::sync::RwLock;

static mut DRIVECHAIN: Option<RwLock<Drivechain>> = None;

#[no_mangle]
pub unsafe extern "C" fn init(
    // NOTE: The caller is responsible for freeing these strings after init is
    // done.
    db_path: *const libc::c_char,
    this_sidechain: usize,
    rpcuser: *const libc::c_char,
    rpcpassword: *const libc::c_char,
) {
    let db_path = CStr::from_ptr(db_path).to_str().unwrap();
    let rpcuser = CStr::from_ptr(rpcuser).to_str().unwrap();
    let rpcpassword = CStr::from_ptr(rpcpassword).to_str().unwrap();
    DRIVECHAIN = Drivechain::new(db_path, this_sidechain, rpcuser, rpcpassword)
        .map(RwLock::new)
        .ok();
    dbg!(DRIVECHAIN
        .as_ref()
        .unwrap()
        .read()
        .unwrap()
        .format_deposit_address("address"));
}

#[no_mangle]
pub unsafe extern "C" fn flush() {
    DRIVECHAIN
        .as_mut()
        .unwrap()
        .write()
        .unwrap()
        .flush()
        .unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn attempt_bmm(critical_hash: *const libc::c_char, amount: u64) {
    // FIXME: Figure out if strings should be freed here.
    let critical_hash = CStr::from_ptr(critical_hash).to_str().unwrap();
    let critical_hash = bitcoin::hash_types::TxMerkleNode::from_str(critical_hash).unwrap();
    let amount = bitcoin::Amount::from_sat(amount);
    DRIVECHAIN
        .as_mut()
        .unwrap()
        .write()
        .unwrap()
        .attempt_bmm(&critical_hash, amount)
        .unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn confirm_bmm() -> u32 {
    match DRIVECHAIN.as_mut().unwrap().write().unwrap().confirm_bmm() {
        Ok(drivechain::BMMState::Succeded) => 0,
        Ok(drivechain::BMMState::Failed) => 1,
        Ok(drivechain::BMMState::Pending) => 2,
        Err(_) => 1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn verify_bmm(
    main_block_hash: *const libc::c_char,
    critical_hash: *const libc::c_char,
) -> bool {
    let main_block_hash = CStr::from_ptr(main_block_hash);
    let main_block_hash =
        bitcoin::hash_types::BlockHash::from_str(main_block_hash.to_str().unwrap()).unwrap();
    let critical_hash = CStr::from_ptr(critical_hash);
    let critical_hash =
        bitcoin::hash_types::TxMerkleNode::from_str(critical_hash.to_str().unwrap()).unwrap();
    DRIVECHAIN
        .as_ref()
        .unwrap()
        .read()
        .unwrap()
        .verify_bmm(&main_block_hash, &critical_hash)
        .is_ok()
}

#[no_mangle]
// NOTE: The caller is responsible for freeing the returned string with
// free_string after a more memory safe string type is constructed.
pub unsafe extern "C" fn get_prev_main_block_hash(
    main_block_hash: *const libc::c_char,
) -> *const libc::c_char {
    let main_block_hash = CStr::from_ptr(main_block_hash);
    let main_block_hash =
        bitcoin::hash_types::BlockHash::from_str(main_block_hash.to_str().unwrap()).unwrap();
    let prev = DRIVECHAIN
        .as_ref()
        .unwrap()
        .read()
        .unwrap()
        .get_prev_main_block_hash(&main_block_hash)
        .unwrap();
    let prev = CString::new(prev.to_string()).unwrap();
    // NOTE: This string must be reconstructed back into CString to be freed.
    // https://doc.rust-lang.org/alloc/ffi/struct.CString.html#method.into_raw
    prev.into_raw()
}

#[repr(C)]
pub struct Deposit {
    pub address: *const libc::c_char,
    pub amount: u64,
}

#[repr(C)]
pub struct Deposits {
    pub ptr: *mut Deposit,
    pub len: usize,
}

#[no_mangle]
// NOTE: The caller is responsible for freeing Deposits by calling free_deposits
// after a more memory safe "Deposits" data structure is constructed.
pub unsafe extern "C" fn get_deposit_outputs() -> Deposits {
    let deposits = DRIVECHAIN
        .as_ref()
        .unwrap()
        .read()
        .unwrap()
        .get_deposit_outputs()
        .unwrap();
    let deposits: Vec<Deposit> = deposits
        .into_iter()
        .map(|d| {
            let address = CString::new(d.address).unwrap();
            Deposit {
                // NOTE: This string must be reconstructed back into CString to be freed.
                // https://doc.rust-lang.org/alloc/ffi/struct.CString.html#method.into_raw
                address: address.into_raw(),
                amount: d.amount,
            }
        })
        .collect();
    let mut deposits = deposits.into_boxed_slice();
    let result = Deposits {
        ptr: deposits.as_mut_ptr(),
        len: deposits.len(),
    };
    std::mem::forget(deposits);
    result
}

#[no_mangle]
pub unsafe extern "C" fn connect_block(deposits: Deposits, just_check: bool) -> bool {
    let deposits = std::slice::from_raw_parts(deposits.ptr, deposits.len);
    let deposits: Vec<drivechain::Deposit> = deposits
        .iter()
        .map(|d| drivechain::Deposit {
            address: CStr::from_ptr(d.address).to_str().unwrap().into(),
            amount: d.amount,
        })
        .collect();
    DRIVECHAIN
        .as_mut()
        .unwrap()
        .write()
        .unwrap()
        .connect_block(&deposits, &HashMap::new(), &[], just_check)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn disconnect_block(deposits: Deposits, just_check: bool) -> bool {
    let deposits = std::slice::from_raw_parts(deposits.ptr, deposits.len);
    let deposits: Vec<drivechain::Deposit> = deposits
        .iter()
        .map(|d| drivechain::Deposit {
            address: CStr::from_ptr(d.address).to_str().unwrap().into(),
            amount: d.amount,
        })
        .collect();
    DRIVECHAIN
        .as_mut()
        .unwrap()
        .write()
        .unwrap()
        .disconnect_block(&deposits, &[], &[], just_check)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn free_string(string: *const libc::c_char) {
    drop(CString::from_raw(string as *mut libc::c_char));
}

#[no_mangle]
pub unsafe extern "C" fn free_deposits(deposits: Deposits) {
    // Convert raw pointer and length into &mut [Deposit].
    let deposits = std::slice::from_raw_parts_mut(deposits.ptr, deposits.len);
    // Free all address strings.
    for deposit in deposits.iter() {
        free_string(deposit.address);
    }
    // Free slice memory.
    std::ptr::drop_in_place(deposits);
}
