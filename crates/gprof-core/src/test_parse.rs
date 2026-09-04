use std::fs::File;
use std::time::Instant;
use memmap2::Mmap;

#[test]
fn test_mmap_parse_time() {
    let path = "../../fixtures/capture 21_30_18.json";
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return, // skip if not found
    };
    let t0 = Instant::now();
    let mmap = unsafe { Mmap::map(&file).unwrap() };
    println!("Mmap took {:?}", t0.elapsed());

    let t1 = Instant::now();
    let v: serde_json::Value = serde_json::from_slice(&mmap).unwrap();
    println!("Generic Value parse took {:?}", t1.elapsed());
    if let Some(cmds) = v.get("commands").and_then(|c| c.as_array()) {
        println!("Parsed {} commands successfully", cmds.len());
    }
}
