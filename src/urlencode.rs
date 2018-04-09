fn is_alpha(v: u8) -> bool {
    (v >= 48 && v <= 57) || (v >= 65 && v <= 90) || (v >= 97 && v <= 122)
}

fn should_escape(v: u8) -> bool {
    is_alpha(v) || (v == 33) || (39 <= v && v <= 42) || v == 45 || v == 46 || v == 95 || v == 126
}

pub fn urlencode(bytes: &[u8]) -> String {
  let strs: Vec<String> = bytes
    .iter()
    .map(|b| {
        if should_escape(*b) {
            String::from_utf8(vec!(*b)).unwrap()
        } else {
            format!("%{:02x}", *b as i32)
        }
    })
    .collect();
  strs.join("")
}
