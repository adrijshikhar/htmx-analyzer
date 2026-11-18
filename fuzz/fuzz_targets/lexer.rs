#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let tags = htmx_parser::lex(data);
    let len = data.len();
    for tag in tags {
        assert!(tag.start <= tag.end);
        assert!(tag.end <= len);
        assert!(data.is_char_boundary(tag.start));
        assert!(data.is_char_boundary(tag.end));

        for attr in tag.attributes {
            assert!(attr.name_range.start <= attr.end);
            assert!(attr.end <= len);
            assert!(data.is_char_boundary(attr.name_range.start));
            assert!(data.is_char_boundary(attr.end));

            assert!(attr.name_range.start <= attr.name_range.end);
            assert!(attr.name_range.end <= len);
            assert!(data.is_char_boundary(attr.name_range.start));
            assert!(data.is_char_boundary(attr.name_range.end));

            if let Some(val) = attr.value {
                assert!(val.range.start <= val.range.end);
                assert!(val.range.end <= len);
                assert!(data.is_char_boundary(val.range.start));
                assert!(data.is_char_boundary(val.range.end));
            }
        }
    }
});
