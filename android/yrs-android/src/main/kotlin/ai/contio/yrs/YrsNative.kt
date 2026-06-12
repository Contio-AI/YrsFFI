package ai.contio.yrs

/**
 * JNI bridge to the vendored `yrs` Rust CRDT core (`libyrs_android.so`).
 *
 * Android analog of this package's iOS `YrsFFI` C ABI: the SAME Rust core
 * (`yrs` 0.25.x), exposed to Kotlin through JNI instead of a C header. The
 * per-ABI `.so` is committed under `src/main/jniLibs` (the binary-artifact
 * analog of the committed `YrsFFI.xcframework`) and packaged into the AAR, so
 * consumers do NOT need Rust. Maintainers regenerate it with
 * `scripts/build-aar.sh` (cargo-ndk).
 *
 * **This object is the raw, unsafe boundary.** Handles are opaque `Long`
 * pointers into native memory; every `*New` has a matching `*Destroy`, and an
 * open transaction must be committed. Downstream code should layer safe
 * wrappers (the Android equivalent of the iOS Swift wrappers) over this and
 * never call these methods directly.
 *
 * `yrs` is not thread-safe, so callers must serialize access per document.
 */
public object YrsNative {
    init {
        System.loadLibrary("yrs_android")
    }

    // --- Document lifecycle (mirror: ydoc_new / ydoc_destroy) ---
    public external fun docNew(): Long

    public external fun docDestroy(doc: Long)

    // --- Transactions (mirror: ydoc_write_transaction / ytransaction_commit) ---

    /** Open a write transaction; returns 0 if one is already open. */
    public external fun writeTransaction(doc: Long): Long

    /** Commit and free a transaction handle. */
    public external fun commitTransaction(txn: Long)

    // --- Y.Text branch (mirror: ytext / ytext_insert / ytext_string / ytext_len) ---
    public external fun textRoot(
        doc: Long,
        name: String,
    ): Long

    public external fun textDestroy(text: Long)

    public external fun textInsert(
        text: Long,
        txn: Long,
        index: Int,
        value: String,
    )

    public external fun textRemoveRange(
        text: Long,
        txn: Long,
        index: Int,
        length: Int,
    )

    public external fun textString(
        text: Long,
        txn: Long,
    ): String

    public external fun textLen(
        text: Long,
        txn: Long,
    ): Int

    // --- Sync primitives (mirror: state_vector_v1 / state_diff_v1 / apply) ---
    public external fun stateVector(txn: Long): ByteArray

    public external fun stateDiff(
        txn: Long,
        stateVector: ByteArray,
    ): ByteArray

    /** Apply a remote update; returns 0 on success, non-zero on failure. */
    public external fun applyUpdate(
        txn: Long,
        update: ByteArray,
    ): Int
}
