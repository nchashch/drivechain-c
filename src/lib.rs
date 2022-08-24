use drivechain::*;
use std::ffi::{CStr, CString};
use std::str::FromStr;
use std::sync::RwLock;

#[no_mangle]
pub extern "C" fn test_function() {}

static mut DRIVECHAIN: Option<RwLock<Drivechain>> = None;

#[no_mangle]
pub unsafe extern "C" fn init(
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
pub unsafe extern "C" fn attempt_bmm(
    critical_hash: *const libc::c_char,
    block_data: *const libc::c_char,
    amount: u64,
) {
    let critical_hash = CStr::from_ptr(critical_hash).to_str().unwrap();
    let critical_hash = bitcoin::hash_types::TxMerkleNode::from_str(critical_hash).unwrap();
    let block_data = CStr::from_ptr(block_data).to_str().unwrap();
    let block_data = hex::decode(block_data).unwrap();
    let amount = bitcoin::Amount::from_sat(amount);
    DRIVECHAIN
        .as_mut()
        .unwrap()
        .write()
        .unwrap()
        .attempt_bmm(&critical_hash, &block_data, amount)
        .unwrap();
}

#[repr(C)]
pub struct Block {
    // hex encoded block data
    pub data: *const libc::c_char,
    // unix timestamp
    pub time: i64,
    // hex encoded main block hash
    pub main_block_hash: *const libc::c_char,
}

#[no_mangle]
pub unsafe extern "C" fn confirm_bmm() -> BMMState {
    let block = DRIVECHAIN
        .as_mut()
        .unwrap()
        .write()
        .unwrap()
        .confirm_bmm()
        .unwrap();
    match block {
        Some(block) => {
            let data = CString::new(hex::encode(block.data.as_slice())).unwrap();
            let data = Box::new(data);
            let data: *const CString = std::mem::transmute(data);
            let data = (&*data).as_ptr();
            let time = block.time;
            let main_block_hash = CString::new(block.main_block_hash.to_string()).unwrap();
            let main_block_hash = Box::new(main_block_hash);
            let main_block_hash: *const CString = std::mem::transmute(main_block_hash);
            let main_block_hash = (&*main_block_hash).as_ptr();
            // It is the responsibility of the caller to free data,
            // main_block_hash, and block struct after use.
            let block = Block {
                data,
                time,
                main_block_hash,
            };
            let block = Box::new(block);
            let block: *const Block = std::mem::transmute(block);
            block
        }
        None => std::ptr::null(),
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
    // Put prev on the heap, so it is not deallocated when the stack frame is dropped.
    let prev = Box::new(prev);
    // Move prev out of the Rust memory model, so it is not deallocated automatically.
    let prev: *const CString = std::mem::transmute(prev);
    // Now it is the responsibility of the caller to free this string after use
    // (most likely after converting it into a more memory safe type).
    (&*prev).as_ptr()
}
