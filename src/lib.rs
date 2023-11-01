// TODO: Very carefully check that there are no memory management bugs.
use drivechain::*;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::str::FromStr;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// FIXME: Get rid of all .unwrap() calls.
static mut DRIVECHAIN: Option<RwLock<Drivechain>> = None;

unsafe fn read_drivechain<'r>() -> RwLockReadGuard<'r, Drivechain> {
    DRIVECHAIN.as_mut().unwrap().read().unwrap()
}

unsafe fn write_drivechain<'r>() -> RwLockWriteGuard<'r, Drivechain> {
    DRIVECHAIN.as_mut().unwrap().write().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn init(
    // NOTE: The caller is responsible for freeing these strings after init is
    // done.
    db_path: *const libc::c_char,
    this_sidechain: usize,
    host: *const libc::c_char,
    port: u16,
    rpcuser: *const libc::c_char,
    rpcpassword: *const libc::c_char,
) -> bool {
    let db_path = CStr::from_ptr(db_path).to_str().unwrap();
    let host = CStr::from_ptr(host).to_str().unwrap();
    let rpcuser = CStr::from_ptr(rpcuser).to_str().unwrap();
    let rpcpassword = CStr::from_ptr(rpcpassword).to_str().unwrap();
    DRIVECHAIN = Drivechain::new(db_path, this_sidechain, host, port, rpcuser, rpcpassword)
        .map(RwLock::new)
        .ok();
    DRIVECHAIN.is_some()
}

#[no_mangle]
pub unsafe extern "C" fn flush() -> usize {
    write_drivechain().flush().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn attempt_bmm(
    critical_hash: *const libc::c_char,
    prev_main_block_hash: *const libc::c_char,
    amount: u64,
) {
    // FIXME: Figure out if strings have to be explicitly freed here.
    let critical_hash = CStr::from_ptr(critical_hash).to_str().unwrap();
    let critical_hash = bitcoin::hash_types::TxMerkleNode::from_str(critical_hash).unwrap();
    let prev_main_block_hash = CStr::from_ptr(prev_main_block_hash).to_str().unwrap();
    let prev_main_block_hash =
        bitcoin::hash_types::BlockHash::from_str(prev_main_block_hash).unwrap();
    let amount = bitcoin::Amount::from_sat(amount);
    match write_drivechain().attempt_bmm(&critical_hash, &prev_main_block_hash, amount) {
        Ok(_) => (),
        Err(_) => println!("attempt_bmm call failed"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn confirm_bmm() -> u32 {
    match write_drivechain().confirm_bmm() {
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
    read_drivechain()
        .verify_bmm(&main_block_hash, &critical_hash)
        .unwrap_or(false)
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
    let prev = read_drivechain()
        .get_prev_main_block_hash(&main_block_hash)
        .map(|prev| CString::new(prev.to_string()).unwrap())
        .unwrap_or(CString::new("").unwrap());
    // NOTE: This string must be reconstructed back into CString to be freed.
    // https://doc.rust-lang.org/alloc/ffi/struct.CString.html#method.into_raw
    prev.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn get_mainchain_tip() -> *const libc::c_char {
    let tip = read_drivechain()
        .get_mainchain_tip()
        .map(|tip| CString::new(tip.to_string()).unwrap())
        .unwrap_or(CString::new("").unwrap());
    // NOTE: This string must be reconstructed back into CString to be freed.
    // https://doc.rust-lang.org/alloc/ffi/struct.CString.html#method.into_raw
    tip.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn format_deposit_address(
    address: *const libc::c_char,
) -> *const libc::c_char {
    let address = CStr::from_ptr(address).to_str().unwrap();
    let deposit_address = read_drivechain().format_deposit_address(address);
    let deposit_address = CString::new(deposit_address).unwrap();
    deposit_address.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn create_deposit(
    address: *const libc::c_char,
    amount: u64,
    fee: u64,
) -> bool {
    let address = CStr::from_ptr(address).to_str().unwrap();
    read_drivechain()
        .create_deposit(
            address,
            bitcoin::Amount::from_sat(amount),
            bitcoin::Amount::from_sat(fee),
        )
        .is_ok()
}

#[repr(C)]
pub struct WithdrawalAddress {
    pub address: [u8; 20],
}

#[no_mangle]
pub unsafe extern "C" fn get_new_mainchain_address() -> WithdrawalAddress {
    let address = read_drivechain().get_new_mainchain_address().unwrap();
    let address = drivechain::Drivechain::extract_mainchain_address_bytes(&address).unwrap();
    WithdrawalAddress { address }
}

#[no_mangle]
pub unsafe extern "C" fn format_mainchain_address(dest: WithdrawalAddress) -> *const libc::c_char {
    let address = drivechain::Drivechain::format_mainchain_address(dest.address).unwrap();
    let address = CString::new(address).unwrap();
    address.into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn attempt_bundle_broadcast() -> bool {
    write_drivechain().attempt_bundle_broadcast().is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn get_unspent_withdrawals() -> Withdrawals {
    let withdrawals = read_drivechain().get_unspent_withdrawals().unwrap();
    let withdrawals: Vec<Withdrawal> = withdrawals
        .iter()
        .map(|(id, w)| {
            let id = CString::new(id.as_slice()).unwrap();
            Withdrawal {
                id: id.into_raw(),
                address: w.dest,
                amount: w.amount,
                fee: w.mainchain_fee,
            }
        })
        .collect();
    // FIXME: Make sure there is no memory leak here. See free_withdrawals
    // function.
    let mut withdrawals = withdrawals.into_boxed_slice();
    let result = Withdrawals {
        ptr: withdrawals.as_mut_ptr(),
        len: withdrawals.len(),
    };
    std::mem::forget(withdrawals);
    result
}

#[repr(C)]
pub struct Deposit {
    pub address: *const libc::c_char,
    pub amount: u64,
}

#[repr(C)]
pub struct Deposits {
    pub valid: bool,
    pub ptr: *mut Deposit,
    pub len: usize,
}

#[repr(C)]
pub struct Withdrawal {
    pub id: *const libc::c_char,
    pub address: [u8; 20],
    pub amount: u64,
    pub fee: u64,
}

#[repr(C)]
pub struct Withdrawals {
    pub ptr: *mut Withdrawal,
    pub len: usize,
}

#[repr(C)]
pub struct Refund {
    pub id: *const libc::c_char,
    pub amount: u64,
}

#[repr(C)]
pub struct Refunds {
    pub ptr: *mut Refund,
    pub len: usize,
}

// NOTE: The caller is responsible for freeing Deposits by calling free_deposits
// after a more memory safe "Deposits" data structure is constructed.
#[no_mangle]
pub unsafe extern "C" fn get_deposit_outputs() -> Deposits {
    let deposits = match read_drivechain().get_deposit_outputs() {
        Ok(deposits) => deposits,
        Err(_) => {
            let mut deposits = vec![].into_boxed_slice();
            let result = Deposits {
                valid: false,
                ptr: deposits.as_mut_ptr(),
                len: deposits.len(),
            };
            std::mem::forget(deposits);
            return result;
        }
    };
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
    // FIXME: Make sure there is no memory leak here. See free_deposits
    // function.
    let mut deposits = deposits.into_boxed_slice();
    let result = Deposits {
        valid: true,
        ptr: deposits.as_mut_ptr(),
        len: deposits.len(),
    };
    std::mem::forget(deposits);
    result
}

#[no_mangle]
pub unsafe extern "C" fn connect_block(
    deposits: Deposits,
    withdrawals: Withdrawals,
    refunds: Refunds,
    just_check: bool,
) -> bool {
    let deposits = std::slice::from_raw_parts(deposits.ptr, deposits.len);
    let deposits: Vec<drivechain::Deposit> = deposits
        .iter()
        .map(|d| drivechain::Deposit {
            address: CStr::from_ptr(d.address).to_str().unwrap().into(),
            amount: d.amount,
        })
        .collect();
    let withdrawals = std::slice::from_raw_parts(withdrawals.ptr, withdrawals.len);
    let withdrawals: HashMap<Vec<u8>, drivechain::Withdrawal> = withdrawals
        .iter()
        .map(|w| {
            (
                CStr::from_ptr(w.id).to_bytes().into(),
                drivechain::Withdrawal {
                    dest: w.address,
                    amount: w.amount,
                    mainchain_fee: w.fee,
                    // height is set later in Db::connect_withdrawals.
                    height: 0,
                },
            )
        })
        .collect();
    let refunds = std::slice::from_raw_parts(refunds.ptr, refunds.len);
    let refunds: HashMap<Vec<u8>, u64> = refunds
        .iter()
        .map(|r| (CStr::from_ptr(r.id).to_bytes().into(), r.amount))
        .collect();
    let result = write_drivechain().connect_block(&deposits, &withdrawals, &refunds, just_check);
    result.is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn disconnect_block(
    deposits: Deposits,
    withdrawals: Withdrawals,
    refunds: Refunds,
    just_check: bool,
) -> bool {
    let deposits = std::slice::from_raw_parts(deposits.ptr, deposits.len);
    let deposits: Vec<drivechain::Deposit> = deposits
        .iter()
        .map(|d| drivechain::Deposit {
            address: CStr::from_ptr(d.address).to_str().unwrap().into(),
            amount: d.amount,
        })
        .collect();
    let withdrawals = std::slice::from_raw_parts(withdrawals.ptr, withdrawals.len);
    let withdrawals: Vec<Vec<u8>> = withdrawals
        .iter()
        .map(|w| CStr::from_ptr(w.id).to_bytes().into())
        .collect();
    let refunds = std::slice::from_raw_parts(refunds.ptr, refunds.len);
    let refunds: Vec<Vec<u8>> = refunds
        .iter()
        .map(|r| CStr::from_ptr(r.id).to_bytes().into())
        .collect();
    write_drivechain()
        .disconnect_block(&deposits, &withdrawals, &refunds, just_check)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn is_outpoint_spent(outpoint: *const libc::c_char) -> bool {
    let outpoint = CStr::from_ptr(outpoint).to_bytes();
    read_drivechain()
        .is_outpoint_spent(outpoint)
        .unwrap_or(true)
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

#[no_mangle]
pub unsafe extern "C" fn free_withdrawals(withdrawals: Withdrawals) {
    let withdrawals = std::slice::from_raw_parts_mut(withdrawals.ptr, withdrawals.len);
    for withdrawal in withdrawals.iter() {
        free_string(withdrawal.id);
    }
    std::ptr::drop_in_place(withdrawals);
}
