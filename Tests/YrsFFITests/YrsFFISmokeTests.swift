import XCTest
@testable import YrsFFI

/// Smoke tests for the YrsFFI xcframework. These exist to catch link-time
/// and ABI breakage when bumping the pinned yffi version. Idiomatic Swift
/// wrappers belong in downstream consumer packages, not here.
final class YrsFFISmokeTests: XCTestCase {

    /// Verifies the static lib was linked correctly: we can construct
    /// and destroy a YDoc without crashing.
    func testCreateAndDestroyDoc() {
        guard let doc = ydoc_new() else {
            XCTFail("ydoc_new() returned NULL — yffi static lib not linked correctly")
            return
        }
        ydoc_destroy(doc)
    }

    /// Round-trips a string through Y.Text. Exercises:
    ///   - ydoc_new / ydoc_destroy lifecycle
    ///   - ydoc_write_transaction / ydoc_read_transaction / ytransaction_commit
    ///   - ytext (named-root accessor)
    ///   - ytext_insert / ytext_string round-trip
    ///   - ystring_destroy for the C string returned by ytext_string
    func testTextInsertAndReadBack() {
        guard let doc = ydoc_new() else {
            XCTFail("ydoc_new() returned NULL")
            return
        }
        defer { ydoc_destroy(doc) }

        let textBranch = "hello-fragment".withCString { cstr in
            ytext(doc, cstr)
        }
        XCTAssertNotNil(textBranch, "ytext(doc, name) returned NULL")

        let writeTxn = ydoc_write_transaction(doc, 0, nil)
        XCTAssertNotNil(writeTxn, "ydoc_write_transaction returned NULL")
        "hello".withCString { value in
            ytext_insert(textBranch, writeTxn, 0, value, nil)
        }
        ytransaction_commit(writeTxn)

        let readTxn = ydoc_read_transaction(doc)
        XCTAssertNotNil(readTxn, "ydoc_read_transaction returned NULL")
        guard let cString = ytext_string(textBranch, readTxn) else {
            ytransaction_commit(readTxn)
            XCTFail("ytext_string returned NULL")
            return
        }
        let result = String(cString: cString)
        ystring_destroy(cString)
        ytransaction_commit(readTxn)

        XCTAssertEqual(result, "hello")
    }
}
