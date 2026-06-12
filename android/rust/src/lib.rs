//! JNI bridge over the `yrs` Rust CRDT core for Android.
//!
//! The Android analog of this repo's Apple C ABI: it exposes just enough of
//! `yrs` to Kotlin to create a `YDoc`, get-or-create a root `Y.Text`,
//! read/write inside a transaction, and — critically — round-trip a document
//! update (state vector -> state diff -> apply) so two docs converge. The
//! Kotlin `YrsNative` binding is the JNI counterpart of the Apple C ABI.
//!
//! ## Ownership model
//! Native objects are handed to the JVM as opaque `jlong` handles (boxed raw
//! pointers). Each `*_new` allocates; the matching `*_destroy` frees. yrs is
//! NOT thread-safe, so the Kotlin side serializes access per document — this
//! layer does no locking of its own (same contract as the iOS `YDoc` actor).
//!
//! `Transact`/`ReadTxn`/`WriteTxn` borrow the doc, so we model an open
//! transaction as a heap-boxed `TransactionMut<'static>` whose lifetime we
//! manage manually; callers MUST commit (free) every transaction they open,
//! exactly like the iOS `ytransaction_commit` contract.

use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;

use yrs::types::text::TextRef;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{
    Doc, GetString, ReadTxn, StateVector, Text, Transact, TransactionMut, Update,
};

// ---------------------------------------------------------------------------
// Handle helpers
// ---------------------------------------------------------------------------

/// Box `value` and leak it as a `jlong` handle the JVM holds onto.
fn into_handle<T>(value: T) -> jlong {
    Box::into_raw(Box::new(value)) as jlong
}

/// Reconstitute a `&mut T` from a handle without taking ownership.
///
/// # Safety
/// `handle` must be a live pointer previously produced by `into_handle::<T>`
/// and not yet freed.
unsafe fn as_mut<'a, T>(handle: jlong) -> &'a mut T {
    &mut *(handle as *mut T)
}

/// Take ownership back from a handle and drop it.
///
/// # Safety
/// `handle` must be a live pointer previously produced by `into_handle::<T>`
/// and not freed since.
unsafe fn drop_handle<T>(handle: jlong) {
    if handle != 0 {
        drop(Box::from_raw(handle as *mut T));
    }
}

// A `Doc` is the owner of everything. We box it directly.
type DocHandle = Doc;
// An open write transaction borrows the doc for its whole life; we erase the
// borrow lifetime to 'static because the Kotlin layer guarantees the doc
// outlives the transaction (it commits before destroying the doc).
type TxnHandle = TransactionMut<'static>;
// A `TextRef` is a lightweight, copyable branch reference into the doc.
type TextHandle = TextRef;

// ---------------------------------------------------------------------------
// Document lifecycle  (mirror: ydoc_new / ydoc_destroy)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_docNew(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    into_handle::<DocHandle>(Doc::new())
}

#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_docDestroy(
    _env: JNIEnv,
    _class: JClass,
    doc: jlong,
) {
    unsafe { drop_handle::<DocHandle>(doc) }
}

// ---------------------------------------------------------------------------
// Transactions  (mirror: ydoc_write_transaction / commit)
// ---------------------------------------------------------------------------

/// Open a write transaction on `doc`. Returns 0 if a transaction is already
/// open (yrs forbids concurrent transactions on a doc).
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_writeTransaction(
    _env: JNIEnv,
    _class: JClass,
    doc: jlong,
) -> jlong {
    let doc = unsafe { as_mut::<DocHandle>(doc) };
    match doc.try_transact_mut() {
        Ok(txn) => {
            // SAFETY: we erase the borrow lifetime to 'static. The Kotlin layer
            // commits this transaction (which frees the handle) before the doc
            // is destroyed, so the borrow never actually dangles.
            let txn: TransactionMut<'static> = unsafe { std::mem::transmute(txn) };
            into_handle::<TxnHandle>(txn)
        }
        Err(_) => 0,
    }
}

/// Commit and free a transaction handle. Always call this for every
/// transaction opened (mirror: ytransaction_commit).
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_commitTransaction(
    _env: JNIEnv,
    _class: JClass,
    txn: jlong,
) {
    // Dropping a TransactionMut commits it.
    unsafe { drop_handle::<TxnHandle>(txn) }
}

// ---------------------------------------------------------------------------
// Y.Text branch  (mirror: ytext / ytext_insert / ytext_string / ytext_len)
// ---------------------------------------------------------------------------

/// Get-or-create the root `Y.Text` named `name`. Returns a branch handle that
/// the caller frees with `textDestroy`.
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_textRoot(
    mut env: JNIEnv,
    _class: JClass,
    doc: jlong,
    name: JString,
) -> jlong {
    let doc = unsafe { as_mut::<DocHandle>(doc) };
    let name: String = match env.get_string(&name) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let text: TextRef = doc.get_or_insert_text(name.as_str());
    into_handle::<TextHandle>(text)
}

#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_textDestroy(
    _env: JNIEnv,
    _class: JClass,
    text: jlong,
) {
    unsafe { drop_handle::<TextHandle>(text) }
}

/// Insert `value` at char index `index` inside an open write transaction.
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_textInsert(
    mut env: JNIEnv,
    _class: JClass,
    text: jlong,
    txn: jlong,
    index: jint,
    value: JString,
) {
    let text = unsafe { as_mut::<TextHandle>(text) };
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    let value: String = match env.get_string(&value) {
        Ok(s) => s.into(),
        Err(_) => return,
    };
    text.insert(txn, index as u32, value.as_str());
}

/// Remove `length` chars starting at `index` inside an open write transaction.
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_textRemoveRange(
    _env: JNIEnv,
    _class: JClass,
    text: jlong,
    txn: jlong,
    index: jint,
    length: jint,
) {
    let text = unsafe { as_mut::<TextHandle>(text) };
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    text.remove_range(txn, index as u32, length as u32);
}

/// Read the text content as a Java string. Reads use a write transaction's
/// read view (a `TransactionMut` also implements `ReadTxn`).
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_textString(
    env: JNIEnv,
    _class: JClass,
    text: jlong,
    txn: jlong,
) -> jstring {
    let text = unsafe { as_mut::<TextHandle>(text) };
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    let s = text.get_string(&*txn);
    match env.new_string(s) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Number of characters in the text content.
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_textLen(
    _env: JNIEnv,
    _class: JClass,
    text: jlong,
    txn: jlong,
) -> jint {
    let text = unsafe { as_mut::<TextHandle>(text) };
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    text.len(&*txn) as jint
}

// ---------------------------------------------------------------------------
// Sync primitives  (mirror: state_vector_v1 / state_diff_v1 / apply)
// ---------------------------------------------------------------------------

/// Encode the document's state vector (lib0 v1) inside an open transaction.
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_stateVector(
    env: JNIEnv,
    _class: JClass,
    txn: jlong,
) -> jbyteArray {
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    let bytes = txn.state_vector().encode_v1();
    byte_array(&env, &bytes)
}

/// Encode this document's update relative to a peer's state vector (lib0 v1).
/// Pass an empty `state_vector` to encode the full document state.
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_stateDiff(
    env: JNIEnv,
    _class: JClass,
    txn: jlong,
    state_vector: JByteArray,
) -> jbyteArray {
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    let sv_bytes = env.convert_byte_array(&state_vector).unwrap_or_default();
    let sv = if sv_bytes.is_empty() {
        StateVector::default()
    } else {
        match StateVector::decode_v1(&sv_bytes) {
            Ok(sv) => sv,
            Err(_) => return byte_array(&env, &[]),
        }
    };
    let bytes = txn.encode_diff_v1(&sv);
    byte_array(&env, &bytes)
}

/// Apply a remote update (lib0 v1) inside an open write transaction. Returns 0
/// on success, non-zero on a malformed/failed update (mirror: ytransaction_apply).
#[no_mangle]
pub extern "system" fn Java_ai_contio_yrs_YrsNative_applyUpdate(
    env: JNIEnv,
    _class: JClass,
    txn: jlong,
    update: JByteArray,
) -> jint {
    let txn = unsafe { as_mut::<TxnHandle>(txn) };
    let bytes = match env.convert_byte_array(&update) {
        Ok(b) => b,
        Err(_) => return 1,
    };
    let update = match Update::decode_v1(&bytes) {
        Ok(u) => u,
        Err(_) => return 2,
    };
    match txn.apply_update(update) {
        Ok(_) => 0,
        Err(_) => 3,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn byte_array(env: &JNIEnv, bytes: &[u8]) -> jbyteArray {
    match env.byte_array_from_slice(bytes) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Rust-side unit tests — exercise the core round-trip without the JVM. These
// run on host with `cargo test` and prove the yrs core + our usage converge,
// independent of the JNI marshaling (which the Kotlin tests cover on-device).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_insert_and_read_back() {
        let doc = Doc::new();
        let text = doc.get_or_insert_text("hello-fragment");
        {
            let mut txn = doc.transact_mut();
            text.insert(&mut txn, 0, "hello");
        }
        let txn = doc.transact();
        assert_eq!(text.get_string(&txn), "hello");
    }

    #[test]
    fn update_round_trips_between_two_docs() {
        // Doc A writes "world".
        let doc_a = Doc::new();
        let text_a = doc_a.get_or_insert_text("shared");
        {
            let mut txn = doc_a.transact_mut();
            text_a.insert(&mut txn, 0, "world");
        }

        // Doc B asks for A's diff against B's (empty) state vector.
        let doc_b = Doc::new();
        let text_b = doc_b.get_or_insert_text("shared");
        let sv_b = doc_b.transact().state_vector().encode_v1();
        let diff = {
            let txn_a = doc_a.transact();
            let sv = StateVector::decode_v1(&sv_b).unwrap();
            txn_a.encode_diff_v1(&sv)
        };

        // B applies A's update and converges.
        {
            let mut txn_b = doc_b.transact_mut();
            let update = Update::decode_v1(&diff).unwrap();
            txn_b.apply_update(update).unwrap();
        }
        assert_eq!(text_b.get_string(&doc_b.transact()), "world");
    }
}
