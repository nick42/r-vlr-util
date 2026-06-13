use r_vlr_util::strings::{SplitOptions, split_path};

fn main() {
    for element in split_path(r"C:\Windows\System32", SplitOptions::default()) {
        println!("{element}");
    }
}
