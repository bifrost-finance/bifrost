#![cfg(test)]
use iost_chain::verify::BlockHead;

#[test]
fn verify_block_head_should_work() {
    let head = BlockHead {
        version: 1,
        parent_hash: "ayIjoV383UIPRxlXM5AHtNmboqKZXZBhNl6rElpuCRA=".as_bytes().to_vec(),
        tx_merkle_hash: "YghPcRrtsuJ/8AqXeK8DdFtOl8j9lyKeTT1rPpp/wBQ=".as_bytes().to_vec(),
        tx_receipt_merkle_hash: "vSGIHJPnI6eWrJ5Oh6AZ/fe2DoIF35WY94kCwW2bPn4=".as_bytes().to_vec(),
        info: "eyJtb2RlIjowLCJ0aHJlYWQiOjAsImJhdGNoIjpudWxsfQ==".as_bytes().to_vec(),
        number: 102492000,
        // 102504000
        witness: "G5DPSoGy4J4y5ZzGQ5uPXbddJFCyzBzva2r5XjFSsNVa".as_bytes().to_vec(),
        time: 1603139621500090226,
        hash: "".as_bytes().to_vec(),
        algorithm: 2,
        sig: "BXoieBOEDU6/u5wsPvEjOAhR6es9kPOV4fObcQb0/lw1QUx5MpWut09McJXq75Rh4vt1eYv+SqF9CfTJVixPBQ==".as_bytes().to_vec(),
        pub_key: "3/OiFQp5j4y3AOAE5mfqImSIrdQHNLm0KqrEmzBJpw0=".as_bytes().to_vec()
    };

    // dbg!(head.parse_head());
    // dbg!(head.parse_sign());
    assert!(head.verify_self());
}
