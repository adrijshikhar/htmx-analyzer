#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: (&str, usize)| {
    let (text, raw_offset) = data;
    let offset = if text.is_empty() {
        0
    } else {
        let mut off = raw_offset % (text.len() + 1);
        while off > 0 && !text.is_char_boundary(off) {
            off -= 1;
        }
        off
    };
    let _ = htmx_parser::CursorScanner::scan(text, offset);
});
